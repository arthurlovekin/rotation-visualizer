//! Rotation vector slider group.
//!
//! Three sliders for x, y, z components, each [-π, π - ε].
//! Range stops short of π to avoid teleport when dragging past the end.

use leptos::prelude::*;

use crate::app::rotation::{Rotation, RotationVector};
use crate::app::slider_group::{SliderSlot, VectorSliderGroup};
use crate::app::slider_widget::CustomSliderConfig;

#[component]
pub fn RotationVectorSliderGroup(
    rotation: RwSignal<Rotation>,
    format_config: CustomSliderConfig,
) -> impl IntoView {
    let rv_x = RwSignal::new(0.0_f64);
    let rv_y = RwSignal::new(0.0_f64);
    let rv_z = RwSignal::new(0.0_f64);

    let slots = vec![
        SliderSlot {
            value: rv_x,
            label: "x",
            dual_value: None,
            on_pointerdown: None,
        },
        SliderSlot {
            value: rv_y,
            label: "y",
            dual_value: None,
            on_pointerdown: None,
        },
        SliderSlot {
            value: rv_z,
            label: "z",
            dual_value: None,
            on_pointerdown: None,
        },
    ];

    let sync_from_rotation: std::rc::Rc<dyn Fn(Rotation) -> Vec<f64>> = std::rc::Rc::new(move |rot| {
        let rv = rot.as_rotation_vector();
        let (mut x, mut y, mut z) = (rv.x as f64, rv.y as f64, rv.z as f64);
        let pi = std::f64::consts::PI;
        let max = pi - 0.001;
        vec![x.clamp(-pi, max), y.clamp(-pi, max), z.clamp(-pi, max)]
    });

    let on_value_change: std::rc::Rc<dyn Fn(usize, f64)> = std::rc::Rc::new({
        let rotation = rotation.clone();
        let rx = rv_x;
        let ry = rv_y;
        let rz = rv_z;

        move |_idx: usize, _value: f64| {
            let x = rx.get_untracked() as f32;
            let y = ry.get_untracked() as f32;
            let z = rz.get_untracked() as f32;
            let rv = RotationVector::new(x, y, z);
            rotation.set(Rotation::from(rv));
        }
    });

    let order_memo = Memo::new(|_| vec![0, 1, 2]);

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
