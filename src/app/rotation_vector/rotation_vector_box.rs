//! Rotation vector input box with slider group.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlSelectElement;

use crate::app::collapsible_section::CollapsibleSection;
use crate::app::dom::input_event_value;
use crate::app::format::{parse_vector_and_format, VectorFormat};
use crate::app::rotation::{Rotation, RotationVector};
use super::slider_group::RotationVectorSliderGroup;
use crate::app::ActiveInput;

#[component]
pub fn RotationVectorBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let use_degrees = RwSignal::new(false);
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format/use_degrees changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        let deg = use_degrees.get();
        if active_input.get() != ActiveInput::RotationVector {
            let rv = rot.as_rotation_vector();
            let values = if deg {
                let rv_deg = rv.as_degrees();
                vec![rv_deg.x as f32, rv_deg.y as f32, rv_deg.z as f32]
            } else {
                vec![rv.x as f32, rv.y as f32, rv.z as f32]
            };
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::RotationVector);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<3>(&value) {
            format.set(detected_fmt);
            let rv = if use_degrees.get_untracked() {
                RotationVector::from_degrees(nums[0] as f32, nums[1] as f32, nums[2] as f32)
            } else {
                RotationVector::new(nums[0] as f32, nums[1] as f32, nums[2] as f32)
            };
            rotation.set(Rotation::from(rv));
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    // Slider values lifted to parent so we can convert them when switching units
    let rv_x = RwSignal::new(0.0_f64);
    let rv_y = RwSignal::new(0.0_f64);
    let rv_z = RwSignal::new(0.0_f64);

    let on_angle_unit_change = move |ev: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        let value = ev
            .target()
            .unwrap()
            .unchecked_into::<HtmlSelectElement>()
            .value();
        let new_use_degrees = value == "degrees";
        // Convert slider values to the new unit before switching, so the sliders
        // display correctly (e.g. 90° → π/2 rad, not 90 on a rad scale)
        let rv = rotation.get().as_rotation_vector();
        if new_use_degrees {
            let rv_deg = rv.as_degrees();
            let (lo, hi) = (-360.0_f64, 360.0_f64);
            batch(|| {
                rv_x.set((rv_deg.x as f64).clamp(lo, hi));
                rv_y.set((rv_deg.y as f64).clamp(lo, hi));
                rv_z.set((rv_deg.z as f64).clamp(lo, hi));
            });
        } else {
            let (lo, hi) = (-2.0 * std::f64::consts::PI, 2.0 * std::f64::consts::PI);
            batch(|| {
                rv_x.set((rv.x as f64).clamp(lo, hi));
                rv_y.set((rv.y as f64).clamp(lo, hi));
                rv_z.set((rv.z as f64).clamp(lo, hi));
            });
        }
        use_degrees.set(new_use_degrees);
    };

    view! {
        <CollapsibleSection title="Rotation Vector">
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
            <RotationVectorSliderGroup
                rotation=rotation
                use_degrees=use_degrees
                rv_x=rv_x
                rv_y=rv_y
                rv_z=rv_z
            />
        </CollapsibleSection>
    }
}
