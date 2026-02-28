//! Rotation vector slider group.
//!
//! **Model**: rotation is the single source of truth. Slider values are derived from it and
//! synced via Effect. When the user drags, we write back to rotation.
//!
//! **State flow**:
//! - rotation (source of truth) → Effect → rv_x, rv_y, rv_z (display)
//! - User drag → on_change → rotation
//! - skip_sync prevents Effect from overwriting during drag.

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
) -> impl IntoView {
    let rv_x = RwSignal::new(0.0_f64);
    let rv_y = RwSignal::new(0.0_f64);
    let rv_z = RwSignal::new(0.0_f64);

    let skip_sync = Rc::new(RefCell::new(false));

    let config_rad = CustomSliderConfig::rotation_vector_component();
    let config_deg = CustomSliderConfig::rotation_vector_component_degrees();

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

    // Sync: rotation + use_degrees → sliders. use_degrees.get() first ensures we track it.
    Effect::new({
        let skip_sync = skip_sync.clone();
        move || {
            let deg = use_degrees.get();
            let rv = rotation.get().as_rotation_vector();
            if *skip_sync.borrow() {
                *skip_sync.borrow_mut() = false;
                return;
            }
            let (sx, sy, sz) = if deg {
                let rv_deg = rv.as_degrees();
                let (lo, hi) = (-360.0_f64, 360.0_f64);
                (
                    (rv_deg.x as f64).clamp(lo, hi),
                    (rv_deg.y as f64).clamp(lo, hi),
                    (rv_deg.z as f64).clamp(lo, hi),
                )
            } else {
                let (lo, hi) = (-2.0 * std::f64::consts::PI, 2.0 * std::f64::consts::PI);
                (
                    (rv.x as f64).clamp(lo, hi),
                    (rv.y as f64).clamp(lo, hi),
                    (rv.z as f64).clamp(lo, hi),
                )
            };
            batch(|| {
                rv_x.set(sx);
                rv_y.set(sy);
                rv_z.set(sz);
            });
        }
    });

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

    view! {
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
    }
}
