//! Rotation vector slider group.
//!
//! **Model**: The rotation library normalizes the rotation vector (norm ∈ [0, 2π), components
//! effectively in the modular ball). The sliders represent the "unsimplified" form (components
//! ∈ [-2π, 2π]) for smooth dragging. The "simplified" form from the rotation is shown as tick
//! marks on the track.
//!
//! **State flow**:
//! - Slider values (rv_x, rv_y, rv_z) → on_change → rotation (source of truth)
//! - rotation → simplified (memo) → Effect syncs back to sliders when rotation changes externally
//! - When the library normalizes (e.g. norm wraps at 2π), we keep slider values for smooth
//!   dragging; ticks show the simplified form.
//!
//! Three sliders for x, y, z components, each [-2π, 2π].

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::app::rotation::{Rotation, RotationVector};
use crate::app::slider_widget::{CustomSlider, CustomSliderConfig};

#[component]
pub fn RotationVectorSliderGroup(
    rotation: RwSignal<Rotation>,
    format_config: CustomSliderConfig,
) -> impl IntoView {
    // ─── Slider state (unsimplified, ∈ [-2π, 2π]) ─────────────────────────
    let rv_x = RwSignal::new(0.0_f64);
    let rv_y = RwSignal::new(0.0_f64);
    let rv_z = RwSignal::new(0.0_f64);

    // Skip Effect sync when the change came from our own slider (avoids overwriting
    // unsimplified values with normalized ones during drag).
    let skip_next_sync = Rc::new(RefCell::new(false));

    // ─── Derived: simplified form for tick marks ─────────────────────────
    let simplified = Memo::new(move |_| {
        let rv = rotation.get().as_rotation_vector();
        (rv.x as f64, rv.y as f64, rv.z as f64)
    });
    let simplified_x = Memo::new(move |_| simplified.get().0);
    let simplified_y = Memo::new(move |_| simplified.get().1);
    let simplified_z = Memo::new(move |_| simplified.get().2);

    // ─── Sync: rotation → sliders (when rotation changes externally) ───────
    Effect::new({
        let skip_next_sync = skip_next_sync.clone();
        move || {
            let _ = rotation.get();
            if *skip_next_sync.borrow() {
                *skip_next_sync.borrow_mut() = false;
                return;
            }
            let (sx, sy, sz) = simplified.get();
            let (lo, hi) = (-2.0 * std::f64::consts::PI, 2.0 * std::f64::consts::PI);
            batch(|| {
                rv_x.set(sx.clamp(lo, hi));
                rv_y.set(sy.clamp(lo, hi));
                rv_z.set(sz.clamp(lo, hi));
            });
        }
    });

    // ─── Write: sliders → rotation ────────────────────────────────────────
    let on_change = Rc::new({
        let skip_next_sync = skip_next_sync.clone();
        move |_value: f64| {
            *skip_next_sync.borrow_mut() = true;
            let rv = RotationVector::new(
                rv_x.get_untracked() as f32,
                rv_y.get_untracked() as f32,
                rv_z.get_untracked() as f32,
            );
            rotation.set(Rotation::from(rv));
        }
    });

    let config = format_config.clone();

    view! {
        <div class="vector-sliders" style="display: flex; flex-direction: column;">
            <div style="order: 0;">
                <CustomSlider
                    label="x"
                    config=config.clone()
                    value=rv_x
                    dual_value=simplified_x
                    on_value_change=on_change.clone()
                />
            </div>
            <div style="order: 1;">
                <CustomSlider
                    label="y"
                    config=config.clone()
                    value=rv_y
                    dual_value=simplified_y
                    on_value_change=on_change.clone()
                />
            </div>
            <div style="order: 2;">
                <CustomSlider
                    label="z"
                    config=config.clone()
                    value=rv_z
                    dual_value=simplified_z
                    on_value_change=on_change.clone()
                />
            </div>
        </div>
    }
}
