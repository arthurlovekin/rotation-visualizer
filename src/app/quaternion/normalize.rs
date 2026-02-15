//! Pure quaternion normalization with LRU strategy.
//!
//! When one component is fixed (user is dragging), the other three must be
//! adjusted to maintain |q| = 1. This module prefers adjusting the
//! least-recently-used component first for smoother UX.

/// Indices for x, y, z, w in xyzw convention.
pub const X: usize = 0;
pub const Y: usize = 1;
pub const Z: usize = 2;
pub const W: usize = 3;

/// Normalize quaternion components using LRU strategy.
///
/// - `values`: [x, y, z, w] - the active component was just set by the user
/// - `active`: index (0..4) of the component being dragged
/// - `order`: [oldest, ..., newest] - indices by last use
/// - Returns normalized [x, y, z, w]
pub fn normalize_lru(
    values: [f64; 4],
    active: usize,
    order: &[usize; 4],
) -> [f64; 4] {
    let a_sq = values[active] * values[active];
    let s = 1.0 - a_sq;

    if s <= 0.0 {
        let mut out = values;
        out[(active + 1) % 4] = 0.0;
        out[(active + 2) % 4] = 0.0;
        out[(active + 3) % 4] = 0.0;
        return out;
    }

    let non_active: Vec<usize> = (0..4).filter(|&i| i != active).collect();
    let pos_in_order = |i: usize| order.iter().position(|&o| o == i).unwrap_or(0);
    let mut ordered: Vec<usize> = non_active;
    ordered.sort_by_key(|&i| pos_in_order(i));
    let [u, v, w] = [ordered[0], ordered[1], ordered[2]];
    let u_old = values[u];
    let v_old = values[v];
    let w_old = values[w];

    // Try adjusting only u. Use sign that minimizes change for continuity near zero.
    let u_sq = s - v_old * v_old - w_old * w_old;
    if u_sq >= 0.0 {
        let sqrt_u = u_sq.sqrt();
        let u_new = if (sqrt_u - u_old).abs() <= (-sqrt_u - u_old).abs() {
            sqrt_u
        } else {
            -sqrt_u
        };
        let mut out = values;
        out[u] = u_new;
        return out;
    }

    // Try adjusting u and v
    let t = s - w_old * w_old;
    if t >= 0.0 {
        let denom = u_old * u_old + v_old * v_old;
        let (u_new, v_new) = if denom > 1e-20 {
            let factor = (t / denom).sqrt();
            (u_old * factor, v_old * factor)
        } else {
            (t.sqrt(), 0.0)
        };
        let mut out = values;
        out[u] = u_new;
        out[v] = v_new;
        return out;
    }

    // Adjust all three
    let denom = u_old * u_old + v_old * v_old + w_old * w_old;
    let factor = if denom > 1e-20 {
        (s / denom).sqrt()
    } else {
        0.0
    };
    let mut out = values;
    out[u] = u_old * factor;
    out[v] = v_old * factor;
    out[w] = w_old * factor;
    out
}

/// Move `index` to newest position in order. order is [oldest, ..., newest].
pub fn touch_order(order: &mut [usize; 4], index: usize) {
    if let Some(p) = order.iter().position(|&o| o == index) {
        for i in p..3 {
            order[i] = order[i + 1];
        }
        order[3] = index;
    }
}
