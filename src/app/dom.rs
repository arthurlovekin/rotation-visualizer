//! DOM helpers for extracting values from input events.

use leptos::wasm_bindgen::JsCast;
use leptos::web_sys::{HtmlInputElement, HtmlTextAreaElement};

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
