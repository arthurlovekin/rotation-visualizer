//! Rotation vector slider group.
//!
//! **Model**: The rotation library normalizes the rotation vector (norm ∈ [0, 2π), components
//! effectively in the modular ball). The sliders represent the "unsimplified" form (components
//! ∈ [-2π, 2π] radians or [-360°, 360°] degrees) for smooth dragging. The "simplified" form from
//! the rotation is shown as tick marks on the track.
//!
//! **State flow**:
//! - Slider values (rv_x, rv_y, rv_z) → on_change → rotation (source of truth)
//! - rotation → simplified (memo) → Effect syncs back to sliders when rotation changes externally
//! - When the library normalizes (e.g. norm wraps at 2π), we keep slider values for smooth
//!   dragging; ticks show the simplified form.
//!
//! Three sliders for x, y, z components. Radians: [-2π, 2π]. Degrees: [-360°, 360°].

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::app::rotation::{Rotation, RotationVector};
use crate::app::slider_widget::{CustomSlider, CustomSliderConfig};

#[component]
pub fn RotationVectorSliderGroup(
    rotation: RwSignal<Rotation>,
    /// true = degrees [-360°, 360°], false = radians [-2π, 2π]
    use_degrees: RwSignal<bool>,
    /// Slider values (lifted to parent so they can be converted when switching units)
    rv_x: RwSignal<f64>,
    rv_y: RwSignal<f64>,
    rv_z: RwSignal<f64>,
) -> impl IntoView {

    // Skip Effect sync when the change came from our own slider (avoids overwriting
    // unsimplified values with normalized ones during drag).
    let skip_sync = Rc::new(RefCell::new(false));

    let config_rad = CustomSliderConfig::rotation_vector_component();
    let config_deg = CustomSliderConfig::rotation_vector_component_degrees();

    // ─── Derived: simplified form for tick marks (always in same unit as sliders) ─
    let simplified_rad = Memo::new(move |_| {
        let rv = rotation.get().as_rotation_vector();
        (rv.x as f64, rv.y as f64, rv.z as f64)
    });
    let simplified_deg = Memo::new(move |_| {
        let rv = rotation.get().as_rotation_vector().as_degrees();
        (rv.x as f64, rv.y as f64, rv.z as f64)
    });
    let simplified_x_rad = Memo::new(move |_| simplified_rad.get().0);
    let simplified_y_rad = Memo::new(move |_| simplified_rad.get().1);
    let simplified_z_rad = Memo::new(move |_| simplified_rad.get().2);
    let simplified_x_deg = Memo::new(move |_| simplified_deg.get().0);
    let simplified_y_deg = Memo::new(move |_| simplified_deg.get().1);
    let simplified_z_deg = Memo::new(move |_| simplified_deg.get().2);

    // ─── Sync: rotation → sliders (when rotation changes externally) ───────
    Effect::new({
        let skip_sync = skip_sync.clone();
        move || {
            let _ = rotation.get();
            if *skip_sync.borrow() {
                *skip_sync.borrow_mut() = false;
                return;
            }
            let deg = use_degrees.get();
            let (sx, sy, sz) = if deg {
                let (x, y, z) = simplified_deg.get();
                let (lo, hi) = (-360.0, 360.0);
                (x.clamp(lo, hi), y.clamp(lo, hi), z.clamp(lo, hi))
            } else {
                let (x, y, z) = simplified_rad.get();
                let (lo, hi) = (-2.0 * std::f64::consts::PI, 2.0 * std::f64::consts::PI);
                (x.clamp(lo, hi), y.clamp(lo, hi), z.clamp(lo, hi))
            };
            batch(|| {
                rv_x.set(sx);
                rv_y.set(sy);
                rv_z.set(sz);
            });
        }
    });

    // ─── Write: sliders → rotation ────────────────────────────────────────
    let on_change = Rc::new({
        let skip_sync = skip_sync.clone();
        move |_value: f64| {
            *skip_sync.borrow_mut() = true;
            let (x, y, z) = (
                rv_x.get_untracked() as f32,
                rv_y.get_untracked() as f32,
                rv_z.get_untracked() as f32,
            );
            let rv = if use_degrees.get_untracked() {
                RotationVector::from_degrees(x, y, z)
            } else {
                RotationVector::new(x, y, z)
            };
            rotation.set(Rotation::from(rv));
        }
    });

    // Radians sliders
    let sliders_rad = view! {
        <div style:display=move || if use_degrees.get() { "none" } else { "block" }>
            <div class="vector-sliders" style="display: flex; flex-direction: column;">
                <div style="order: 0;">
                    <CustomSlider
                        label="x"
                        config=config_rad.clone()
                        value=rv_x
                        dual_value=simplified_x_rad
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 1;">
                    <CustomSlider
                        label="y"
                        config=config_rad.clone()
                        value=rv_y
                        dual_value=simplified_y_rad
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 2;">
                    <CustomSlider
                        label="z"
                        config=config_rad.clone()
                        value=rv_z
                        dual_value=simplified_z_rad
                        on_value_change=on_change.clone()
                    />
                </div>
            </div>
        </div>
    };

    // Degrees sliders
    let sliders_deg = view! {
        <div style:display=move || if use_degrees.get() { "block" } else { "none" }>
            <div class="vector-sliders" style="display: flex; flex-direction: column;">
                <div style="order: 0;">
                    <CustomSlider
                        label="x"
                        config=config_deg.clone()
                        value=rv_x
                        dual_value=simplified_x_deg
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 1;">
                    <CustomSlider
                        label="y"
                        config=config_deg.clone()
                        value=rv_y
                        dual_value=simplified_y_deg
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 2;">
                    <CustomSlider
                        label="z"
                        config=config_deg.clone()
                        value=rv_z
                        dual_value=simplified_z_deg
                        on_value_change=on_change.clone()
                    />
                </div>
            </div>
        </div>
    };

    view! {
        <>
            {sliders_rad}
            {sliders_deg}
        </>
    }
}
