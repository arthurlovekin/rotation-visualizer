//! Euler angles input box with sequence and unit dropdowns.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlSelectElement;

use crate::app::collapsible_section::CollapsibleSection;
use crate::app::dom::input_event_value;
use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{DEFAULT_EULER_SEQUENCE, EulerAngles, EulerSequence, Rotation};
use super::slider_group::EulerAnglesSliderGroup;
use crate::app::ActiveInput;

const ALL_SEQUENCES: [EulerSequence; 12] = [
    EulerSequence::XYZ_zyx,
    EulerSequence::XZY_yzx,
    EulerSequence::YXZ_zxy,
    EulerSequence::YZX_xzy,
    EulerSequence::ZXY_yxz,
    EulerSequence::ZYX_xyz,
    EulerSequence::XYX_xyx,
    EulerSequence::XZX_xzx,
    EulerSequence::YXY_yxy,
    EulerSequence::YZY_yzy,
    EulerSequence::ZXZ_zxz,
    EulerSequence::ZYZ_zyz,
];

#[component]
pub fn EulerAnglesBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let use_degrees = RwSignal::new(false);
    let sequence = RwSignal::new(DEFAULT_EULER_SEQUENCE);
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format/use_degrees/sequence changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        let deg = use_degrees.get();
        let seq = sequence.get();
        if active_input.get() != ActiveInput::EulerAngles {
            let ea = rot.as_euler_angles(seq);
            let values = if deg {
                let (a, b, c) = ea.as_degrees();
                vec![a, b, c]
            } else {
                vec![ea.a, ea.b, ea.c]
            };
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::EulerAngles);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<3>(&value) {
            format.set(detected_fmt);
            let seq = sequence.get_untracked();
            let ea = if use_degrees.get_untracked() {
                EulerAngles::from_degrees(nums[0] as f32, nums[1] as f32, nums[2] as f32, seq)
            } else {
                EulerAngles::new(nums[0] as f32, nums[1] as f32, nums[2] as f32, seq)
            };
            rotation.set(Rotation::from(ea));
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

    let on_sequence_change = move |ev: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        let value = ev
            .target()
            .unwrap()
            .unchecked_into::<HtmlSelectElement>()
            .value();
        if let Some(seq) = ALL_SEQUENCES.iter().find(|s| s.display_name() == value.as_str()).copied() {
            sequence.set(seq);
        }
    };

    view! {
        <CollapsibleSection title="Euler Angles">
            <input
                type="text"
                class="vector-input vector-input-3"
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
            <div class="convention-row">
                "Sequence: "
                <select
                    prop:value=move || sequence.get().display_name()
                    on:change=on_sequence_change
                >
                    {ALL_SEQUENCES.iter().map(|seq| view! {
                        <option value=seq.display_name()>{seq.display_name()}</option>
                    }).collect::<Vec<_>>()}
                </select>
            </div>
            <EulerAnglesSliderGroup rotation=rotation use_degrees=use_degrees sequence=sequence />
        </CollapsibleSection>
    }
}
