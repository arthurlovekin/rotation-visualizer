//! Axis-angle slider group.
//!
//! Four sliders: unit vector x, y, z (each [-1, 1]) and angle θ.
//! Uses Least-Recently-Used normalization for the axis components.
//! Angle slider uses radians [0, π] or degrees [0, 180] based on use_degrees.

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
    /// true = degrees [0, 180], false = radians [0, π]
    use_degrees: RwSignal<bool>,
) -> impl IntoView {
    let axis_x = RwSignal::new(0.0_f64);
    let axis_y = RwSignal::new(0.0_f64);
    let axis_z = RwSignal::new(0.0_f64);
    let angle = RwSignal::new(0.0_f64);

    let order = Rc::new(RefCell::new([0, 1, 2]));
    let order_for_update = order.clone();

    let axis_config = CustomSliderConfig::quaternion_component();
    let angle_config_rad = CustomSliderConfig::angle_0_pi();
    let angle_config_deg = CustomSliderConfig::angle_degrees_0_180();

    // Sync rotation -> sliders when rotation changes.
    Effect::new(move || {
        let rot = rotation.get();
        let deg = use_degrees.get();
        let aa = rot.as_axis_angle();
        let (ax, ay, az, a) = (aa.x as f64, aa.y as f64, aa.z as f64, aa.angle as f64);
        let angle_val = if deg { a.to_degrees() } else { a };
        batch(|| {
            axis_x.set(ax);
            axis_y.set(ay);
            axis_z.set(az);
            angle.set(angle_val);
        });
    });

    let update_rotation_from_axis = Rc::new({
        let order_for_update = order_for_update;
        move |ax: f64, ay: f64, az: f64, a: f64, changed_idx: usize| {
            let norm_sq = ax * ax + ay * ay + az * az;
            if norm_sq < AXIS_EPSILON {
                rotation.set(Rotation::default());
                return;
            }
            let values = [ax, ay, az];
            let ord = *order_for_update.borrow();
            let normalized = normalize_lru_3(values, changed_idx, &ord);
            let nx = normalized[0] as f32;
            let ny = normalized[1] as f32;
            let nz = normalized[2] as f32;
            let angle_rad = if use_degrees.get_untracked() {
                a.to_radians() as f32
            } else {
                a as f32
            };
            if let Ok(aa) = AxisAngle::try_new(nx, ny, nz, angle_rad) {
                rotation.set(Rotation::from(aa));
            }
        }
    });

    let update_rotation_from_angle = move |ax: f64, ay: f64, az: f64, a: f64| {
        let norm_sq = ax * ax + ay * ay + az * az;
        if norm_sq < AXIS_EPSILON {
            rotation.set(Rotation::default());
            return;
        }
        let norm = norm_sq.sqrt();
        let nx = (ax / norm) as f32;
        let ny = (ay / norm) as f32;
        let nz = (az / norm) as f32;
        let angle_rad = if use_degrees.get_untracked() {
            a.to_radians() as f32
        } else {
            a as f32
        };
        if let Ok(aa) = AxisAngle::try_new(nx, ny, nz, angle_rad) {
            rotation.set(Rotation::from(aa));
        }
    };

    let on_x_change = Rc::new({
        let ax = axis_x;
        let ay = axis_y;
        let az = axis_z;
        let ang = angle;
        let update = update_rotation_from_axis.clone();
        move |_v: f64| {
            let (x, y, z) = (ax.get_untracked(), ay.get_untracked(), az.get_untracked());
            let a = ang.get_untracked();
            update(x, y, z, a, 0);
        }
    });
    let on_y_change = Rc::new({
        let ax = axis_x;
        let ay = axis_y;
        let az = axis_z;
        let ang = angle;
        let update = update_rotation_from_axis.clone();
        move |_v: f64| {
            let (x, y, z) = (ax.get_untracked(), ay.get_untracked(), az.get_untracked());
            let a = ang.get_untracked();
            update(x, y, z, a, 1);
        }
    });
    let on_z_change = Rc::new({
        let ax = axis_x;
        let ay = axis_y;
        let az = axis_z;
        let ang = angle;
        let update = update_rotation_from_axis.clone();
        move |_v: f64| {
            let (x, y, z) = (ax.get_untracked(), ay.get_untracked(), az.get_untracked());
            let a = ang.get_untracked();
            update(x, y, z, a, 2);
        }
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

    let on_angle_change = Rc::new({
        let ax = axis_x;
        let ay = axis_y;
        let az = axis_z;
        move |v: f64| {
            let (x, y, z) = (ax.get_untracked(), ay.get_untracked(), az.get_untracked());
            update_rotation_from_angle(x, y, z, v);
        }
    });

    // Angle slider: use degrees or radians config. We render both and hide one via CSS
    // to avoid Send/Sync issues with conditional closures capturing Rc.
    let angle_slider_rad = view! {
        <div style:display=move || if use_degrees.get() { "none" } else { "block" }>
            <CustomSlider
                label="θ"
                config=angle_config_rad.clone()
                value=angle
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
                    on_handle_pointerdown=on_x_pd
                    on_value_change=on_x_change
                />
            </div>
            <div style="order: 1;">
                <CustomSlider
                    label="y"
                    config=axis_config.clone()
                    value=axis_y
                    on_handle_pointerdown=on_y_pd
                    on_value_change=on_y_change
                />
            </div>
            <div style="order: 2;">
                <CustomSlider
                    label="z"
                    config=axis_config.clone()
                    value=axis_z
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
