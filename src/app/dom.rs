//! DOM helpers for extracting values from input events and shared event handlers.

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement};

use super::ActiveInput;

/// Returns a blur handler that clears the active input.
pub fn make_on_blur(active_input: RwSignal<ActiveInput>) -> impl Fn(leptos::web_sys::FocusEvent) {
    move |_| active_input.set(ActiveInput::None)
}

/// Returns a change handler for the angle unit (radians/degrees) select.
pub fn make_on_angle_unit_change(
    active_input: RwSignal<ActiveInput>,
    use_degrees: RwSignal<bool>,
) -> impl Fn(leptos::web_sys::Event) {
    move |ev: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        let value = ev
            .target()
            .unwrap()
            .unchecked_into::<HtmlSelectElement>()
            .value();
        use_degrees.set(value == "degrees");
    }
}

/// Extract the current value from an input element's event target.
pub fn input_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlInputElement>()
        .value()
}

/// Extract the current value from a textarea element's event target.
pub fn textarea_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<HtmlTextAreaElement>()
        .value()
}
