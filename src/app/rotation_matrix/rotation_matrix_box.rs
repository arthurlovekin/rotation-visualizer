//! Rotation matrix input box (3x3 textarea).

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::HtmlTextAreaElement;

use crate::app::format::{parse_matrix_and_format, MatrixFormat};
use crate::app::rotation::{Rotation, RotationMatrix};
use crate::app::ActiveInput;

fn textarea_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlTextAreaElement>()
        .value()
}

#[component]
pub fn RotationMatrixBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<MatrixFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let text = RwSignal::new(format.get_untracked().format_matrix(&identity));

    // Reactive effect: reformat when rotation/format changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        if active_input.get() != ActiveInput::RotationMatrix {
            let m = rot.as_rotation_matrix();
            let values = [[m[0][0], m[0][1], m[0][2]], [m[1][0], m[1][1], m[1][2]], [m[2][0], m[2][1], m[2][2]]];
            text.set(fmt.format_matrix(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = textarea_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::RotationMatrix);

        if let Ok((matrix, detected_fmt)) = parse_matrix_and_format::<3, 3>(&value) {
            format.set(detected_fmt);
            let rm = RotationMatrix([
                [matrix[0][0], matrix[0][1], matrix[0][2]],
                [matrix[1][0], matrix[1][1], matrix[1][2]],
                [matrix[2][0], matrix[2][1], matrix[2][2]],
            ]);
            rotation.set(Rotation::from(rm));
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    view! {
        <div class="control-section">
            <h2>"Rotation Matrix"</h2>
            <textarea
                rows=3
                class="vector-input matrix-input"
                prop:value=move || text.get()
                on:input=on_input
                on:blur=on_blur
            />
        </div>
    }
}
