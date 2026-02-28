//! Axis-angle slider group.
//!
//! **Model**: There is always a "simplified" axis-angle (angle ∈ [0, π], canonical form)
//! derived from the rotation — it drives the tick marks. The sliders represent the
//! "unsimplified" form (angle ∈ [-π, 2π]) for smooth dragging; both represent the
//! same rotation.
//!
//! **State flow**:
//! - Slider values (axis_x/y/z, angle) → update_rotation → rotation (source of truth)
//! - rotation → simplified (memo) → Effect syncs back to sliders when rotation changes externally
//! - On axis flip (simplified axis ≈ -current), we keep slider axes for smooth dragging; ticks show simplified
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

fn make_on_change(update: Rc<dyn Fn(Option<usize>)>, idx: Option<usize>) -> Rc<dyn Fn(f64)> {
    Rc::new(move |_| update(idx))
}

#[component]
pub fn AxisAngleSliderGroup(
    rotation: RwSignal<Rotation>,
    /// true = degrees [-180°, 360°], false = radians [-π, 2π]
    use_degrees: RwSignal<bool>,
) -> impl IntoView {
    // Slider state: unsimplified form (angle ∈ [-π, 2π], axis continuous)
    let axis_x = RwSignal::new(1.0_f64);
    let axis_y = RwSignal::new(0.0_f64);
    let axis_z = RwSignal::new(0.0_f64);
    let angle = RwSignal::new(0.0_f64);

    let order = Rc::new(RefCell::new([0, 1, 2]));
    let slider_did_update = Rc::new(RefCell::new(false));

    let axis_config = CustomSliderConfig::quaternion_component();
    let angle_config_rad = CustomSliderConfig::angle_rad_neg_pi_2pi();
    let angle_config_deg = CustomSliderConfig::angle_deg_neg180_360();

    // Simplified form (angle ∈ [0, π]) drives tick marks. Derived from rotation.
    let simplified = Memo::new(move |_| {
        let aa = rotation.get().as_axis_angle();
        (aa.x as f64, aa.y as f64, aa.z as f64, aa.angle as f64)
    });
    let simplified_axis_x = Memo::new(move |_| simplified.get().0);
    let simplified_axis_y = Memo::new(move |_| simplified.get().1);
    let simplified_axis_z = Memo::new(move |_| simplified.get().2);
    let simplified_angle_rad = Memo::new(move |_| simplified.get().3);
    let simplified_angle_deg = Memo::new(move |_| simplified.get().3.to_degrees());

    // Sync rotation → sliders when rotation changes externally (not from our slider).
    Effect::new({
        let slider_did_update = slider_did_update.clone();
        move || {
            let _ = rotation.get();
            if *slider_did_update.borrow() {
                *slider_did_update.borrow_mut() = false;
                return;
            }
            let (ax, ay, az, a) = simplified.get();
            let angle_val = if use_degrees.get() { a.to_degrees() } else { a };
            let (sx, sy, sz) = (axis_x.get_untracked(), axis_y.get_untracked(), axis_z.get_untracked());

            // When simplified axis ≈ -current (angle ≥ π flip), keep axes for smooth dragging; ticks show simplified.
            let axis_flipped = a.abs() > AXIS_EPSILON && (ax * sx + ay * sy + az * sz) < -0.99;
            batch(|| {
                if a.abs() > AXIS_EPSILON && !axis_flipped {
                    axis_x.set(ax);
                    axis_y.set(ay);
                    axis_z.set(az);
                }
                angle.set(angle_val);
            });
        }
    });

    // Slider → rotation: normalize axis (LRU when axis changed, else simple), update rotation, write back normalized axis.
    let update_from_slider = Rc::new({
        let order = order.clone();
        let slider_did_update = slider_did_update.clone();
        move |changed_idx: Option<usize>| {
            *slider_did_update.borrow_mut() = true;
            let (x, y, z) = (axis_x.get_untracked(), axis_y.get_untracked(), axis_z.get_untracked());
            let a = angle.get_untracked();

            let norm_sq = x * x + y * y + z * z;
            if norm_sq < AXIS_EPSILON {
                rotation.set(Rotation::default());
                return;
            }
            let (nx, ny, nz) = match changed_idx {
                Some(i) => {
                    let ord = *order.borrow();
                    let n = normalize_lru_3([x, y, z], i, &ord);
                    (n[0], n[1], n[2])
                }
                None => {
                    let norm = norm_sq.sqrt();
                    (x / norm, y / norm, z / norm)
                }
            };
            let angle_rad = if use_degrees.get_untracked() {
                a.to_radians() as f32
            } else {
                a as f32
            };
            if let Ok(aa) = AxisAngle::try_new(nx as f32, ny as f32, nz as f32, angle_rad) {
                rotation.set(Rotation::from(aa));
                batch(|| {
                    axis_x.set(nx);
                    axis_y.set(ny);
                    axis_z.set(nz);
                });
            }
        }
    });

    let on_x_change = make_on_change(update_from_slider.clone(), Some(0));
    let on_y_change = make_on_change(update_from_slider.clone(), Some(1));
    let on_z_change = make_on_change(update_from_slider.clone(), Some(2));
    let on_angle_change = make_on_change(update_from_slider.clone(), None);

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
