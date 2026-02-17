//! Quaternion input box with wxyz/xyzw convention selector and slider group.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{Quaternion, Rotation};
use crate::app::slider_widget::CustomSliderConfig;
use super::slider_group::QuaternionSliderGroup;
use crate::app::ActiveInput;

fn input_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlInputElement>()
        .value()
}

#[component]
pub fn QuaternionBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let is_xyzw = RwSignal::new(false);
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0, 1.0]));

    // Reactive effect: reformat whenever the rotation, format, or convention
    // changes — but only if this box is NOT the one the user is typing in.
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        let xyzw = is_xyzw.get();
        if active_input.get() != ActiveInput::Quaternion {
            let q = rot.as_quaternion();
            let values: Vec<f32> = if xyzw {
                vec![q.x as f32, q.y as f32, q.z as f32, q.w as f32]
            } else {
                vec![q.w as f32, q.x as f32, q.y as f32, q.z as f32]
            };
            text.set(fmt.format_vector(&values));
        }
    });

    // Parse user input, update shared format + rotation
    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::Quaternion);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<4>(&value) {
            format.set(detected_fmt);
            let (w, x, y, z) = if is_xyzw.get_untracked() {
                (nums[3] as f32, nums[0] as f32, nums[1] as f32, nums[2] as f32)
            } else {
                (nums[0] as f32, nums[1] as f32, nums[2] as f32, nums[3] as f32)
            };
            if let Ok(q) = Quaternion::try_new(w, x, y, z) {
                rotation.set(Rotation::from(q));
            }
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    let on_convention_change = move |ev: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        let value = ev.target()
            .unwrap()
            .unchecked_into::<HtmlSelectElement>()
            .value();
        is_xyzw.set(value == "xyzw");
    };

    let quat_config = CustomSliderConfig::quaternion_component();

    view! {
        <div class="control-section">
            <h2>"Quaternion"</h2>
            <input
            type="text"
            class="vector-input"
            prop:value=move || text.get()
            on:input=on_input
            on:blur=on_blur
            />
            <div class="convention-row">
                "Convention: "
                <select
                    prop:value=move || if is_xyzw.get() { "xyzw" } else { "wxyz" }
                    on:change=on_convention_change
                >
                    <option value="wxyz">"wxyz (scalar-first)"</option>
                    <option value="xyzw">"xyzw (scalar-last)"</option>
                </select>
            </div>
            <QuaternionSliderGroup rotation=rotation format_config=quat_config is_xyzw=is_xyzw />
        </div>
    }
}
