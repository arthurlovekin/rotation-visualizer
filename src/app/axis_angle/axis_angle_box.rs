//! Axis-angle input box with VectorFormat, degrees/radians dropdown, and slider group.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement};

use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{AxisAngle, Rotation};
use crate::app::ActiveInput;

use super::slider_group::AxisAngleSliderGroup;

fn input_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlInputElement>()
        .value()
}

#[component]
pub fn AxisAngleBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let use_degrees = RwSignal::new(false);
    let text = RwSignal::new(format.get_untracked().format_vector(&[1.0, 0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format/use_degrees changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        let deg = use_degrees.get();
        if active_input.get() != ActiveInput::AxisAngle {
            let aa = rot.as_axis_angle();
            let angle_val = if deg {
                aa.angle.to_degrees() as f32
            } else {
                aa.angle as f32
            };
            let values = vec![aa.x as f32, aa.y as f32, aa.z as f32, angle_val];
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::AxisAngle);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<4>(&value) {
            format.set(detected_fmt);
            let (ax, ay, az, a) = (nums[0] as f32, nums[1] as f32, nums[2] as f32, nums[3] as f32);
            let angle_rad = if use_degrees.get_untracked() {
                a.to_radians()
            } else {
                a
            };
            if let Ok(aa) = AxisAngle::try_new(ax, ay, az, angle_rad) {
                rotation.set(Rotation::from(aa));
            }
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    let on_angle_unit_change = move |ev: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        let value = ev
            .target()
            .unwrap()
            .unchecked_into::<HtmlSelectElement>()
            .value();
        use_degrees.set(value == "degrees");
    };

    view! {
        <div class="control-section">
            <h2>"Axis-Angle"</h2>
            <input
            type="text"
            class="vector-input vector-input-4"
            prop:value=move || text.get()
            on:input=on_input
            on:blur=on_blur
            />
            <div class="convention-row">
                "Angle unit: "
                <select
                    prop:value=move || if use_degrees.get() { "degrees" } else { "radians" }
                    on:change=on_angle_unit_change
                >
                    <option value="radians">"radians"</option>
                    <option value="degrees">"degrees"</option>
                </select>
            </div>
            <AxisAngleSliderGroup rotation=rotation use_degrees=use_degrees />
        </div>
    }
}
