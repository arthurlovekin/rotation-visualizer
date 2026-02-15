//! Multi-handle slider widget for rotation visualizer.
//!
//! Supports:
//! - N draggable handles per slider (for quaternion dual, angles mod 2π)
//! - Configurable min/max per slider
//! - Annotation markers (e.g., 0, π, 2π)
//!
//! This module is self-contained: it injects its own CSS when first used.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use leptos::prelude::*;
use leptos::web_sys::PointerEvent;
use wasm_bindgen::JsCast;

static SLIDER_STYLES_INJECTED: AtomicBool = AtomicBool::new(false);

const SLIDER_CSS: &str = r#"
.multi-handle-slider {
  margin: 1em 0;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: 1em;
}
.slider-label {
  flex: 0 0 auto;
  font-size: 0.9em;
  color: #aaa;
  min-width: 1.5em;
}
.slider-track-container {
  position: relative;
  flex: 1;
  min-width: 0;
  height: 2em;
}
.slider-track {
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  transform: translateY(-50%);
  height: 0.35em;
  background: #1e1e24;
  border: none;
  outline: none;
  border-radius: 2px;
  box-shadow: inset 0 0 0 1px rgba(80,80,90,0.5),
              inset 0 1px 2px rgba(0,0,0,0.3);
}
.slider-marker {
  position: absolute;
  top: 100%;
  transform: translateX(-50%);
  margin-top: 0.2em;
  display: flex;
  flex-direction: column;
  align-items: center;
  pointer-events: none;
}
.slider-marker-tick {
  width: 1px;
  height: 0.25em;
  background: #333;
  margin-bottom: 0.1em;
}
.slider-marker-label {
  font-size: 0.65em;
  color: #555;
}
.slider-handle {
  position: absolute;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 2.2em;
  height: 1.1em;
  background: rgba(100, 200, 255, 0.85);
  border-radius: 1px;
  cursor: grab;
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2;
  pointer-events: auto;
}
.slider-handle:active {
  cursor: grabbing;
}
.slider-handle-value {
  font-size: 0.65em;
  font-family: monospace;
  color: rgba(200, 235, 255, 0.95);
  width: 4ch;
  text-align: center;
}
"#;

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

/// Format value to 3 significant figures, 4 characters wide.
fn format_value_4ch_3sig(v: f64) -> String {
    if v == 0.0 {
        return "0.00".to_string();
    }
    let abs = v.abs();
    let s = if abs >= 100.0 {
        format!("{:.0}", v)
    } else if abs >= 10.0 {
        format!("{:.1}", v)
    } else if abs >= 1.0 {
        format!("{:.2}", v)
    } else if abs >= 0.1 {
        format!("{:.2}", v)
    } else if abs >= 0.01 {
        format!("{:.3}", v)
    } else {
        format!("{:.3}", v)
    };
    s.chars().take(4).collect()
}

#[component]
pub fn MultiHandleSlider<F, C>(
    /// Label shown above the slider.
    label: &'static str,
    /// Slider configuration (min, max, markers).
    config: MultiHandleSliderConfig,
    /// One RwSignal per handle. Each handle's value is stored in its signal.
    values: Vec<RwSignal<f64>>,
    /// Callback invoked when a handle is pressed (receives handle index).
    on_handle_pointerdown: F,
    /// Callback invoked when a handle's value changes during drag (handle_index, new_value).
    on_value_change: C,
) -> impl IntoView
where
    F: Fn(usize) + Clone + 'static,
    C: Fn(usize, f64) + Clone + 'static,
{
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
    let inject_styles = !SLIDER_STYLES_INJECTED.swap(true, Ordering::SeqCst);

    view! {
        <>
            {inject_styles.then(|| view! { <style>{SLIDER_CSS}</style> })}
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
                        let on_handle_pointerdown = on_handle_pointerdown.clone();
                        let on_value_change = on_value_change.clone();
                        let on_pointerdown = move |ev: PointerEvent| {
                            on_handle_pointerdown(i);
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
                                on_value_change(i, new_value);
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
                                let on_value_change = on_value_change.clone();
                                move |ev: leptos::web_sys::PointerEvent| {
                                    let fraction = ((ev.client_x() as f64 - track_left) / track_width)
                                        .clamp(0.0, 1.0);
                                    let raw_value = fraction_to_value(fraction, min, max);
                                    let new_value = raw_value.clamp(min, max);
                                    value_signal.set(new_value);
                                    on_value_change(i, new_value);
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
                                    {move || format_value_4ch_3sig(value_signal.get())}
                                </span>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </div>
        </>
    }
}
