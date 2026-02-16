//! Rotation vector input box with slider group.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlInputElement;

use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{AxisAngle, Rotation};
use crate::app::slider_widget::CustomSliderConfig;
use super::slider_group::RotationVectorSliderGroup;
use crate::app::ActiveInput;

fn input_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlInputElement>()
        .value()
}

#[component]
pub fn RotationVectorBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        if active_input.get() != ActiveInput::RotationVector {
            let rv = rot.as_rotation_vector();
            let values = vec![
                rv.x as f32,
                rv.y as f32,
                rv.z as f32,
            ];
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::RotationVector);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<3>(&value) {
            format.set(detected_fmt);
            let (ax, ay, az) = (nums[0] as f32, nums[1] as f32, nums[2] as f32);
            let angle = (ax * ax + ay * ay + az * az).sqrt();
            if angle > 1e-10 {
                let aa = AxisAngle::new(ax / angle, ay / angle, az / angle, angle);
                rotation.set(Rotation::from(aa));
            } else {
                rotation.set(Rotation::default());
            }
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    let rv_config = CustomSliderConfig::rotation_vector_component();

    view! {
        <div class="control-section">
            <h2>"Rotation Vector"</h2>
            <input
                type="text"
                class="vector-input vector-input-3"
                prop:value=move || text.get()
                on:input=on_input
                on:blur=on_blur
            />
            <RotationVectorSliderGroup rotation=rotation format_config=rv_config />
        </div>
    }
}
