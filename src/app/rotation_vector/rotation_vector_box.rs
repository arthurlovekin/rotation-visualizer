//! Rotation vector input box with slider group.

use leptos::prelude::*;

use crate::app::collapsible_section::CollapsibleSection;
use crate::app::dom::input_event_value;
use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{Rotation, RotationVector};
use crate::app::slider_widget::CustomSliderConfig;
use super::slider_group::RotationVectorSliderGroup;
use crate::app::ActiveInput;

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
            let rv = RotationVector::new(nums[0] as f32, nums[1] as f32, nums[2] as f32);
            rotation.set(Rotation::from(rv));
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    let rv_config = CustomSliderConfig::rotation_vector_component();

    view! {
        <CollapsibleSection title="Rotation Vector">
            <input
                type="text"
                class="vector-input vector-input-3"
                prop:value=move || text.get()
                on:input=on_input
                on:blur=on_blur
            />
            <RotationVectorSliderGroup rotation=rotation slider_config=rv_config />
        </CollapsibleSection>
    }
}
