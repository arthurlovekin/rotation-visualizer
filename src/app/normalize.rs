//! Least-Recently-Used normalization for unit vectors and quaternions.
//!
//! When one component is fixed (user is dragging), the others must be
//! adjusted to maintain unit length. This module prefers adjusting the
//! least-recently-used component first for smoother UX.

/// Normalize 3 components (e.g. unit axis) using LRU strategy.
///
/// - `values`: [v0, v1, v2] - the active component was just set by the user
/// - `active`: index (0..3) of the component being dragged
/// - `order`: [oldest, ..., newest] - indices by last use
/// - Returns normalized [v0, v1, v2] with sum of squares = 1
pub fn normalize_lru_3(
    values: [f64; 3],
    active: usize,
    order: &[usize; 3],
) -> [f64; 3] {
    let s = 1.0 - values[active] * values[active];
    if s <= 0.0 {
        let mut out = values;
        out[(active + 1) % 3] = 0.0;
        out[(active + 2) % 3] = 0.0;
        return out;
    }
    let non_active: Vec<usize> = (0..3).filter(|&i| i != active).collect();
    let pos_in_order = |i: usize| order.iter().position(|&o| o == i).unwrap_or(0);
    let mut ordered: Vec<usize> = non_active;
    ordered.sort_by_key(|&i| pos_in_order(i));
    let two_vals = [values[ordered[0]], values[ordered[1]], 0.0];
    let result = normalize_lru_3_with_target(two_vals, s);
    let mut out = values;
    out[ordered[0]] = result[0];
    out[ordered[1]] = result[1];
    out
}

/// Fix 3 values so their sum of squares equals target, using LRU strategy.
/// Tries fix 1 (index 0 only), then 2, then 3 (scale all).
/// Caller must pass values in LRU order (index 0 = LRU).
/// Used by normalize_lru_3 (via padding) and normalize_lru_4.
fn normalize_lru_3_with_target(values: [f64; 3], target: f64) -> [f64; 3] {
    let [u_old, v_old, w_old] = [values[0], values[1], values[2]];

    if target <= 0.0 {
        return [0.0, 0.0, 0.0];
    }

    // Try adjusting only u (LRU)
    let u_sq = target - v_old * v_old - w_old * w_old;
    if u_sq >= 0.0 {
        let sqrt_u = u_sq.sqrt();
        let u_new = if (sqrt_u - u_old).abs() <= (-sqrt_u - u_old).abs() {
            sqrt_u
        } else {
            -sqrt_u
        };
        return [u_new, v_old, w_old];
    }

    // Try adjusting u and v
    let t = target - w_old * w_old;
    if t >= 0.0 {
        let denom = u_old * u_old + v_old * v_old;
        let (u_new, v_new) = if denom > 1e-20 {
            let factor = (t / denom).sqrt();
            (u_old * factor, v_old * factor)
        } else {
            (t.sqrt(), 0.0)
        };
        return [u_new, v_new, w_old];
    }

    // Adjust all three
    let denom = u_old * u_old + v_old * v_old + w_old * w_old;
    let factor = if denom > 1e-20 {
        (target / denom).sqrt()
    } else {
        0.0
    };
    [
        u_old * factor,
        v_old * factor,
        w_old * factor,
    ]
}

/// Normalize 4 components (e.g. quaternion) using LRU strategy.
///
/// - `values`: [v0, v1, v2, v3] - the active component was just set by the user
/// - `active`: index (0..4) of the component being dragged
/// - `order`: [oldest, ..., newest] - indices by last use
/// - Returns normalized [v0, v1, v2, v3] with sum of squares = 1
///
/// Delegates the "fix 3 values" step to normalize_lru_3_with_target.
pub fn normalize_lru_4(
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

    let three_vals = [values[ordered[0]], values[ordered[1]], values[ordered[2]]];
    let result_3 = normalize_lru_3_with_target(three_vals, s);

    let mut out = values;
    out[ordered[0]] = result_3[0];
    out[ordered[1]] = result_3[1];
    out[ordered[2]] = result_3[2];
    out
}

/// Move `index` to newest position in order. order is [oldest, ..., newest].
/// Works for any length (3 or 4).
pub fn touch_order(order: &mut [usize], index: usize) {
    let n = order.len();
    if n == 0 {
        return;
    }
    if let Some(p) = order.iter().position(|&o| o == index) {
        for i in p..n - 1 {
            order[i] = order[i + 1];
        }
        order[n - 1] = index;
    }
}
