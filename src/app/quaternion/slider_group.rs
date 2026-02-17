//! Quaternion slider group with Least-Recently-Used normalization.
//!
//! Uses the generic VectorSliderGroup with quaternion-specific:
//! - 4 components (x, y, z, w) with dual values (-q)
//! - Least-Recently-Used touch order for normalization
//! - xyzw vs wxyz layout order

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::app::normalize::{normalize_lru_4, touch_order};
use crate::app::rotation::{Quaternion, Rotation};
use crate::app::slider_group::{SliderSlot, VectorSliderGroup};
use crate::app::slider_widget::CustomSliderConfig;

#[component]
pub fn QuaternionSliderGroup(
    rotation: RwSignal<Rotation>,
    format_config: CustomSliderConfig,
    /// true = xyzw (x,y,z,w), false = wxyz (w,x,y,z)
    is_xyzw: RwSignal<bool>,
) -> impl IntoView {
    let quat_x = RwSignal::new(0.0_f64);
    let quat_y = RwSignal::new(0.0_f64);
    let quat_z = RwSignal::new(0.0_f64);
    let quat_w = RwSignal::new(1.0_f64);

    let dual_x = Memo::new(move |_| -quat_x.get());
    let dual_y = Memo::new(move |_| -quat_y.get());
    let dual_z = Memo::new(move |_| -quat_z.get());
    let dual_w = Memo::new(move |_| -quat_w.get());

    let order = Rc::new(RefCell::new([0, 1, 2, 3]));

    let slots = vec![
        SliderSlot {
            value: quat_x,
            label: "x",
            dual_value: Some(dual_x),
            on_pointerdown: Some(Rc::new({
                let order = order.clone();
                move || touch_order(order.borrow_mut().as_mut(), 0)
            })),
        },
        SliderSlot {
            value: quat_y,
            label: "y",
            dual_value: Some(dual_y),
            on_pointerdown: Some(Rc::new({
                let order = order.clone();
                move || touch_order(order.borrow_mut().as_mut(), 1)
            })),
        },
        SliderSlot {
            value: quat_z,
            label: "z",
            dual_value: Some(dual_z),
            on_pointerdown: Some(Rc::new({
                let order = order.clone();
                move || touch_order(order.borrow_mut().as_mut(), 2)
            })),
        },
        SliderSlot {
            value: quat_w,
            label: "w",
            dual_value: Some(dual_w),
            on_pointerdown: Some(Rc::new({
                let order = order.clone();
                move || touch_order(order.borrow_mut().as_mut(), 3)
            })),
        },
    ];

    let sync_from_rotation: Rc<dyn Fn(Rotation) -> Vec<f64>> = Rc::new(move |rot| {
        let q = rot.as_quaternion();
        vec![q.x as f64, q.y as f64, q.z as f64, q.w as f64]
    });

    let on_value_change: Rc<dyn Fn(usize, f64)> = Rc::new({
        let order = order.clone();
        let rotation = rotation.clone();
        let qx = quat_x;
        let qy = quat_y;
        let qz = quat_z;
        let qw = quat_w;

        move |idx: usize, _value: f64| {
            let values = [
                qx.get_untracked(),
                qy.get_untracked(),
                qz.get_untracked(),
                qw.get_untracked(),
            ];
            let ord = *order.borrow();
            let normalized = normalize_lru_4(values, idx, &ord);
            let new_rot = match Quaternion::try_new(
                normalized[3] as f32,
                normalized[0] as f32,
                normalized[1] as f32,
                normalized[2] as f32,
            ) {
                Ok(q) => Rotation::from(q),
                Err(_) => Rotation::default(),
            };
            rotation.set(new_rot);
        }
    });

    let order_memo = Memo::new(move |_| {
        if is_xyzw.get() {
            vec![0, 1, 2, 3] // x, y, z, w
        } else {
            vec![1, 2, 3, 0] // w, x, y, z
        }
    });

    view! {
        <VectorSliderGroup
            rotation=rotation
            slots=slots
            format_config=format_config
            sync_from_rotation=sync_from_rotation
            on_value_change=on_value_change
            order=order_memo
        />
    }
}
