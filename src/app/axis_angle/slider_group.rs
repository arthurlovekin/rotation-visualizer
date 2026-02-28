//! Axis-angle slider group.
//!
//! **Model**: There is always a "simplified" axis-angle (angle ∈ [0, π], canonical form)
//! derived from the rotation — it drives the tick marks. The sliders represent the
//! "unsimplified" form (angle ∈ [-π, 2π]) for smooth dragging; both represent the
//! same rotation.
//!
//! Four sliders: unit vector x, y, z (each [-1, 1]) and angle θ.
//! Uses Least-Recently-Used normalization for the axis components.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::app::normalize::{normalize_lru_3, touch_order};
use crate::app::rotation::{AxisAngle, Rotation};
use crate::app::slider_widget::{CustomSlider, CustomSliderConfig};

const AXIS_EPSILON: f64 = 1e-10;

#[component]
pub fn AxisAngleSliderGroup(
    rotation: RwSignal<Rotation>,
    /// true = degrees [-180°, 360°], false = radians [-π, 2π]
    use_degrees: RwSignal<bool>,
) -> impl IntoView {
    // Unsimplified form: slider state. Angle in [-π, 2π], axis continuous.
    let axis_x = RwSignal::new(1.0_f64);
    let axis_y = RwSignal::new(0.0_f64);
    let axis_z = RwSignal::new(0.0_f64);
    let angle = RwSignal::new(0.0_f64);

    let order = Rc::new(RefCell::new([0, 1, 2]));
    let order_for_update = order.clone();
    let slider_did_update = Rc::new(RefCell::new(false));

    let axis_config = CustomSliderConfig::quaternion_component();
    let angle_config_rad = CustomSliderConfig::angle_rad_neg_pi_2pi();
    let angle_config_deg = CustomSliderConfig::angle_deg_neg180_360();

    // Simplified form: always exists, drives tick marks. Derived from rotation.
    let simplified = Memo::new(move |_| {
        let aa = rotation.get().as_axis_angle();
        (aa.x as f64, aa.y as f64, aa.z as f64, aa.angle as f64)
    });
    let simplified_axis_x = Memo::new(move |_| simplified.get().0);
    let simplified_axis_y = Memo::new(move |_| simplified.get().1);
    let simplified_axis_z = Memo::new(move |_| simplified.get().2);
    let simplified_angle_rad = Memo::new(move |_| simplified.get().3);
    let simplified_angle_deg = Memo::new(move |_| simplified.get().3.to_degrees());

    // Sync rotation → sliders when rotation changes externally. Skip when we just updated from slider.
    let slider_did_update_for_effect = slider_did_update.clone();
    Effect::new(move || {
        let _ = rotation.get();
        if *slider_did_update_for_effect.borrow() {
            *slider_did_update_for_effect.borrow_mut() = false;
            return;
        }
        let (ax, ay, az, a) = simplified.get();
        let deg = use_degrees.get();
        let angle_val = if deg { a.to_degrees() } else { a };
        let (sx, sy, sz) = (axis_x.get_untracked(), axis_y.get_untracked(), axis_z.get_untracked());
        // If simplified axis is the negation of current (angle ≥ π flip), keep axes where they are;
        // ticks still show simplified form. Otherwise sync axis from simplified.
        let dot = ax * sx + ay * sy + az * sz;
        let skip_axis_sync = a.abs() > AXIS_EPSILON && dot < -0.99;
        batch(|| {
            if a.abs() > AXIS_EPSILON && !skip_axis_sync {
                axis_x.set(ax);
                axis_y.set(ay);
                axis_z.set(az);
            }
            angle.set(angle_val);
        });
    });

    // Update rotation from unsimplified slider values. changed_idx = which axis changed (for LRU).
    // Always writes normalized axis back to sliders so they stay normalized.
    let update_rotation = Rc::new({
        let order_for_update = order_for_update;
        move |ax: f64, ay: f64, az: f64, a: f64, changed_idx: Option<usize>| -> Option<(f64, f64, f64)> {
            let norm_sq = ax * ax + ay * ay + az * az;
            if norm_sq < AXIS_EPSILON {
                rotation.set(Rotation::default());
                return None;
            }
            let (nx, ny, nz) = match changed_idx {
                Some(i) => {
                    let ord = *order_for_update.borrow();
                    let n = normalize_lru_3([ax, ay, az], i, &ord);
                    (n[0], n[1], n[2])
                }
                None => {
                    let norm = norm_sq.sqrt();
                    (ax / norm, ay / norm, az / norm)
                }
            };
            let angle_rad = if use_degrees.get_untracked() {
                a.to_radians() as f32
            } else {
                a as f32
            };
            if let Ok(aa) = AxisAngle::try_new(nx as f32, ny as f32, nz as f32, angle_rad) {
                rotation.set(Rotation::from(aa));
                Some((nx, ny, nz))
            } else {
                None
            }
        }
    });

    let update_from_slider = Rc::new({
        let slider_did_update = slider_did_update.clone();
        let update_rotation = update_rotation.clone();
        move |changed_idx: Option<usize>| {
            *slider_did_update.borrow_mut() = true;
            let (x, y, z) = (axis_x.get_untracked(), axis_y.get_untracked(), axis_z.get_untracked());
            let a = angle.get_untracked();
            if let Some((nx, ny, nz)) = update_rotation(x, y, z, a, changed_idx) {
                batch(|| {
                    axis_x.set(nx);
                    axis_y.set(ny);
                    axis_z.set(nz);
                });
            }
        }
    });

    let on_x_change = Rc::new({
        let u = update_from_slider.clone();
        move |_v: f64| u(Some(0))
    });
    let on_y_change = Rc::new({
        let u = update_from_slider.clone();
        move |_v: f64| u(Some(1))
    });
    let on_z_change = Rc::new({
        let u = update_from_slider.clone();
        move |_v: f64| u(Some(2))
    });
    let on_angle_change = Rc::new({
        let u = update_from_slider.clone();
        move |_v: f64| u(None)
    });

    let on_x_pd = Rc::new({
        let order = order.clone();
        move || touch_order(order.borrow_mut().as_mut(), 0)
    });
    let on_y_pd = Rc::new({
        let order = order.clone();
        move || touch_order(order.borrow_mut().as_mut(), 1)
    });
    let on_z_pd = Rc::new({
        let order = order.clone();
        move || touch_order(order.borrow_mut().as_mut(), 2)
    });

    // Angle slider: use degrees or radians config. We render both and hide one via CSS
    // to avoid Send/Sync issues with conditional closures capturing Rc.
    // dual_value shows the simplified form [0, π] / [0°, 180°] as a tick mark.
    let angle_slider_rad = view! {
        <div style:display=move || if use_degrees.get() { "none" } else { "block" }>
            <CustomSlider
                label="θ"
                config=angle_config_rad.clone()
                value=angle
                dual_value=simplified_angle_rad
                on_value_change=on_angle_change.clone()
            />
        </div>
    };
    let angle_slider_deg = view! {
        <div style:display=move || if use_degrees.get() { "block" } else { "none" }>
            <CustomSlider
                label="θ"
                config=angle_config_deg.clone()
                value=angle
                dual_value=simplified_angle_deg
                on_value_change=on_angle_change.clone()
            />
        </div>
    };

    view! {
        <div class="vector-sliders" style="display: flex; flex-direction: column;">
            <div style="order: 0;">
                <CustomSlider
                    label="x"
                    config=axis_config.clone()
                    value=axis_x
                    dual_value=simplified_axis_x
                    on_handle_pointerdown=on_x_pd
                    on_value_change=on_x_change
                />
            </div>
            <div style="order: 1;">
                <CustomSlider
                    label="y"
                    config=axis_config.clone()
                    value=axis_y
                    dual_value=simplified_axis_y
                    on_handle_pointerdown=on_y_pd
                    on_value_change=on_y_change
                />
            </div>
            <div style="order: 2;">
                <CustomSlider
                    label="z"
                    config=axis_config.clone()
                    value=axis_z
                    dual_value=simplified_axis_z
                    on_handle_pointerdown=on_z_pd
                    on_value_change=on_z_change
                />
            </div>
            <div style="order: 3;">
                {angle_slider_rad}
                {angle_slider_deg}
            </div>
        </div>
    }
}
