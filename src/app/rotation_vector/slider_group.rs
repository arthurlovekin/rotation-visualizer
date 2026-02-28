//! Rotation vector slider group.
//!
//! **Model**: The rotation library normalizes the rotation vector (norm ∈ [0, 2π), components
//! effectively in the modular ball). The sliders represent the "unsimplified" form (components
//! ∈ [-2π, 2π]) for smooth dragging. The "simplified" form from the rotation is shown as tick
//! marks on the track.
//!
//! **State flow**:
//! - Slider values (rv_x, rv_y, rv_z) → update rotation → rotation (source of truth)
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

fn make_on_change(update: Rc<dyn Fn(usize)>, idx: usize) -> Rc<dyn Fn(f64)> {
    Rc::new(move |_| update(idx))
}

#[component]
pub fn RotationVectorSliderGroup(
    rotation: RwSignal<Rotation>,
    format_config: CustomSliderConfig,
) -> impl IntoView {
    // Slider state: unsimplified form (components ∈ [-2π, 2π])
    let rv_x = RwSignal::new(0.0_f64);
    let rv_y = RwSignal::new(0.0_f64);
    let rv_z = RwSignal::new(0.0_f64);

    let slider_did_update = Rc::new(RefCell::new(false));

    // Simplified form (from rotation) drives tick marks.
    let simplified = Memo::new(move |_| {
        let rv = rotation.get().as_rotation_vector();
        (rv.x as f64, rv.y as f64, rv.z as f64)
    });
    let simplified_x = Memo::new(move |_| simplified.get().0);
    let simplified_y = Memo::new(move |_| simplified.get().1);
    let simplified_z = Memo::new(move |_| simplified.get().2);

    // Sync rotation → sliders when rotation changes externally (not from our slider).
    Effect::new({
        let slider_did_update = slider_did_update.clone();
        move || {
            let _ = rotation.get();
            if *slider_did_update.borrow() {
                *slider_did_update.borrow_mut() = false;
                return;
            }
            let (sx, sy, sz) = simplified.get();
            let pi = std::f64::consts::PI;
            batch(|| {
                rv_x.set(sx.clamp(-2.0 * pi, 2.0 * pi));
                rv_y.set(sy.clamp(-2.0 * pi, 2.0 * pi));
                rv_z.set(sz.clamp(-2.0 * pi, 2.0 * pi));
            });
        }
    });

    // Slider → rotation: pass raw values; library normalizes internally.
    let update_from_slider = Rc::new({
        let slider_did_update = slider_did_update.clone();
        move |_idx: usize| {
            *slider_did_update.borrow_mut() = true;
            let x = rv_x.get_untracked() as f32;
            let y = rv_y.get_untracked() as f32;
            let z = rv_z.get_untracked() as f32;
            let rv = RotationVector::new(x, y, z);
            rotation.set(Rotation::from(rv));
        }
    });

    let on_x_change = make_on_change(update_from_slider.clone(), 0);
    let on_y_change = make_on_change(update_from_slider.clone(), 1);
    let on_z_change = make_on_change(update_from_slider.clone(), 2);

    let config = format_config.clone();

    view! {
        <div class="vector-sliders" style="display: flex; flex-direction: column;">
            <div style="order: 0;">
                <CustomSlider
                    label="x"
                    config=config.clone()
                    value=rv_x
                    dual_value=simplified_x
                    on_value_change=on_x_change
                />
            </div>
            <div style="order: 1;">
                <CustomSlider
                    label="y"
                    config=config.clone()
                    value=rv_y
                    dual_value=simplified_y
                    on_value_change=on_y_change
                />
            </div>
            <div style="order: 2;">
                <CustomSlider
                    label="z"
                    config=config.clone()
                    value=rv_z
                    dual_value=simplified_z
                    on_value_change=on_z_change
                />
            </div>
        </div>
    }
}
