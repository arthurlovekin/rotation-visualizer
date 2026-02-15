//! Multi-handle slider widget for rotation visualizer.
//!
//! Supports:
//! - N draggable handles per slider (for quaternion dual, angles mod 2π)
//! - Configurable min/max per slider
//! - Annotation markers (e.g., 0, π, 2π)

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::web_sys::PointerEvent;
use wasm_bindgen::JsCast;

/// A marker displayed at a specific value along the slider track.
#[derive(Clone)]
pub struct SliderMarker {
    /// Value at which to show the marker (should be within min..=max).
    pub value: f64,
    /// Label to display (e.g., "0", "π", "2π").
    pub label: String,
}

/// Configuration for a multi-handle slider.
#[derive(Clone)]
pub struct MultiHandleSliderConfig {
    /// Minimum value of the slider.
    pub min: f64,
    /// Maximum value of the slider.
    pub max: f64,
    /// Optional markers to display along the track.
    pub markers: Vec<SliderMarker>,
}

impl Default for MultiHandleSliderConfig {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 1.0,
            markers: Vec::new(),
        }
    }
}

impl MultiHandleSliderConfig {
    /// Create config for an angle slider [0, 2π] with 0, π, 2π markers.
    pub fn angle_2pi() -> Self {
        let pi = std::f64::consts::PI;
        Self {
            min: 0.0,
            max: 2.0 * pi,
            markers: vec![
                SliderMarker { value: 0.0, label: "0".to_string() },
                SliderMarker { value: pi, label: "π".to_string() },
                SliderMarker { value: 2.0 * pi, label: "2π".to_string() },
            ],
        }
    }

    /// Create config for a quaternion component [-1, 1].
    pub fn quaternion_component() -> Self {
        Self {
            min: -1.0,
            max: 1.0,
            markers: vec![
                SliderMarker { value: -1.0, label: "-1".to_string() },
                SliderMarker { value: 0.0, label: "0".to_string() },
                SliderMarker { value: 1.0, label: "1".to_string() },
            ],
        }
    }
}

/// Convert value to fraction [0, 1] for positioning.
fn value_to_fraction(value: f64, min: f64, max: f64) -> f64 {
    let range = max - min;
    if range <= 0.0 {
        return 0.0;
    }
    ((value - min) / range).clamp(0.0, 1.0)
}

/// Convert fraction [0, 1] to value.
fn fraction_to_value(fraction: f64, min: f64, max: f64) -> f64 {
    min + fraction * (max - min)
}

#[component]
pub fn MultiHandleSlider(
    /// Label shown above the slider.
    label: &'static str,
    /// Slider configuration (min, max, markers).
    config: MultiHandleSliderConfig,
    /// One RwSignal per handle. Each handle's value is stored in its signal.
    values: Vec<RwSignal<f64>>,
) -> impl IntoView {
    let track_ref = NodeRef::<leptos::html::Div>::new();
    let min = config.min;
    let max = config.max;
    let markers = config.markers;

    // Clamp initial values
    for value_signal in &values {
        value_signal.update(|v| {
            *v = v.clamp(min, max);
        });
    }

    let handle_count = values.len();

    view! {
        <div class="multi-handle-slider">
            <label class="slider-label">{label}</label>
            <div class="slider-track-container">
                <div class="slider-track" node_ref=track_ref>
                    {markers.iter().map(|m| {
                        let frac = value_to_fraction(m.value, min, max);
                        let left_pct = (frac * 100.0).min(100.0).max(0.0);
                        view! {
                            <div
                                class="slider-marker"
                                style:left=format!("{}%", left_pct)
                            >
                                <span class="slider-marker-tick"></span>
                                <span class="slider-marker-label">{m.label.clone()}</span>
                            </div>
                        }
                    }).collect_view()}
                    {(0..handle_count).map(|i| {
                        let value_signal = values[i];
                        let track_ref = track_ref.clone();
                        let values = values.clone();
                        let min = min;
                        let max = max;
                        let on_pointerdown = move |ev: PointerEvent| {
                            ev.prevent_default();
                            let track = track_ref.get_untracked();
                            let track_el: leptos::web_sys::HtmlElement = match track {
                                Some(el) => el.unchecked_into(),
                                None => return,
                            };
                            let rect = track_el.get_bounding_client_rect();
                            let track_left = rect.left();
                            let track_width = rect.width();
                            let value_signal = values[i];
                            let update_value = |client_x: f64| {
                                if track_width <= 0.0 {
                                    return;
                                }
                                let fraction = ((client_x - track_left) / track_width).clamp(0.0, 1.0);
                                let raw_value = fraction_to_value(fraction, min, max);
                                let new_value = raw_value.clamp(min, max);
                                value_signal.set(new_value);
                            };
                            update_value(ev.client_x() as f64);
                            let document = leptos::web_sys::window()
                                .expect("no window")
                                .document()
                                .expect("no document");
                            let move_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let value_signal = value_signal;
                                let track_left = track_left;
                                let track_width = track_width;
                                let min = min;
                                let max = max;
                                move |ev: leptos::web_sys::PointerEvent| {
                                    let fraction = ((ev.client_x() as f64 - track_left) / track_width)
                                        .clamp(0.0, 1.0);
                                    let raw_value = fraction_to_value(fraction, min, max);
                                    let new_value = raw_value.clamp(min, max);
                                    value_signal.set(new_value);
                                }
                            }) as Box<dyn FnMut(_)>);
                            type ClosureType = wasm_bindgen::closure::Closure<dyn FnMut(leptos::web_sys::PointerEvent)>;
                            let closures_rc = Rc::new(RefCell::new(None::<(ClosureType, ClosureType)>));
                            let up_closure = wasm_bindgen::closure::Closure::wrap(Box::new({
                                let document = document.clone();
                                let closures_rc = closures_rc.clone();
                                move |ev: leptos::web_sys::PointerEvent| {
                                    ev.prevent_default();
                                    if let Some((ref m, ref u)) = *closures_rc.borrow() {
                                        let _ = document.remove_event_listener_with_callback(
                                            "pointermove",
                                            m.as_ref().unchecked_ref(),
                                        );
                                        let _ = document.remove_event_listener_with_callback(
                                            "pointerup",
                                            u.as_ref().unchecked_ref(),
                                        );
                                    }
                                }
                            }) as Box<dyn FnMut(_)>);
                            *closures_rc.borrow_mut() = Some((move_closure, up_closure));
                            {
                                let guard = closures_rc.borrow();
                                let (m, u) = guard.as_ref().unwrap();
                                let _ = document.add_event_listener_with_callback(
                                    "pointermove",
                                    m.as_ref().unchecked_ref(),
                                );
                                let _ = document.add_event_listener_with_callback(
                                    "pointerup",
                                    u.as_ref().unchecked_ref(),
                                );
                            }
                            std::mem::forget(closures_rc);
                        };
                        view! {
                            <div
                                class="slider-handle"
                                style:left=move || {
                                    let v = value_signal.get();
                                    let frac = value_to_fraction(v, min, max);
                                    format!("{}%", (frac * 100.0).min(100.0).max(0.0))
                                }
                                on:pointerdown=on_pointerdown
                                role="slider"
                                tabindex="0"
                            >
                                <span class="slider-handle-value">
                                    {move || format!("{:.3}", value_signal.get())}
                                </span>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
    }
}
