//! Generic vector slider group for rotation representations.
//!
//! Provides a reusable component that:
//! - Syncs rotation -> slider values when rotation changes
//! - Renders N sliders with configurable labels, dual values, and order
//! - Throttles value changes via RAF (~60fps) during drag
//!
//! Specific representations provide their own sync/transform logic:
//! - Quaternion: 4 components with LRU normalization, dual values, xyzw/wxyz order
//! - RotationVector: 3 components (x,y,z), no normalization, no dual values

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use crate::app::rotation::Rotation;
use crate::app::slider_widget::{CustomSlider, CustomSliderConfig};

/// Configuration for a single slider in the group.
pub struct SliderSlot {
    /// Value signal for this component.
    pub value: RwSignal<f64>,
    /// Label shown for the slider (e.g., "x", "y", "z", "w").
    pub label: &'static str,
    /// Optional dual value (e.g., -value for quaternion) shown as a tick mark.
    pub dual_value: Option<Memo<f64>>,
    /// Optional callback when handle is pressed (e.g., for LRU touch order).
    pub on_pointerdown: Option<Rc<dyn Fn()>>,
}

/// Generic vector slider group.
///
/// - `rotation`: shared rotation signal
/// - `slots`: per-component configuration (value, label, optional dual, optional pointerdown)
/// - `format_config`: slider min/max/markers
/// - `sync_from_rotation`: extracts component values from rotation (for Effect sync)
/// - `on_value_change`: called when user drags; receives (component_index, new_value).
///   Use RAF-throttled; reads other values from slots and updates rotation.
/// - `order`: Memo returning flex order index per slot for layout (e.g., xyzw vs wxyz).
#[component]
pub fn VectorSliderGroup(
    rotation: RwSignal<Rotation>,
    slots: Vec<SliderSlot>,
    format_config: CustomSliderConfig,
    sync_from_rotation: Rc<dyn Fn(Rotation) -> Vec<f64>>,
    on_value_change: Rc<dyn Fn(usize, f64)>,
    order: Memo<Vec<usize>>,
) -> impl IntoView {
    let value_signals: Vec<RwSignal<f64>> = slots.iter().map(|s| s.value).collect();

    // Sync rotation -> sliders when rotation changes (text input or external update).
    Effect::new(move || {
        let rot = rotation.get();
        let values = sync_from_rotation(rot);
        if values.len() != value_signals.len() {
            return;
        }
        batch(|| {
            for (i, sig) in value_signals.iter().enumerate() {
                sig.set(values[i]);
            }
        });
    });

    // RAF-throttled value change handler.
    let handle_value_change: Rc<dyn Fn(usize, f64)> = Rc::new({
        let on_value_change = on_value_change.clone();
        let pending: Rc<RefCell<Option<(usize, f64)>>> = Rc::new(RefCell::new(None));
        let raf_scheduled: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        move |component_index: usize, value: f64| {
            *pending.borrow_mut() = Some((component_index, value));

            if *raf_scheduled.borrow() {
                return;
            }
            *raf_scheduled.borrow_mut() = true;

            let pending = pending.clone();
            let on_value_change = on_value_change.clone();
            let raf_scheduled = raf_scheduled.clone();

            let f = leptos::wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                *raf_scheduled.borrow_mut() = false;
                let Some((idx, v)) = pending.borrow_mut().take() else { return };
                on_value_change(idx, v);
            }) as Box<dyn FnMut()>);

            leptos::web_sys::window()
                .and_then(|w| w.request_animation_frame(f.as_ref().unchecked_ref()).ok())
                .expect("requestAnimationFrame");
            std::mem::forget(f);
        }
    });

    view! {
        <div class="vector-sliders" style="display: flex; flex-direction: column;">
            {slots.into_iter().enumerate().map(|(i, slot)| {
                let order = order.clone();
                let flex_order = move || {
                    let ord = order.get();
                    ord.get(i).copied().unwrap_or(i)
                };
                let on_change: Rc<dyn Fn(f64)> = Rc::new({
                    let h = handle_value_change.clone();
                    move |v: f64| h(i, v)
                });
                let label = slot.label;
                let config = format_config.clone();
                let value = slot.value;
                let dual_value = slot.dual_value;
                let on_pointerdown = slot.on_pointerdown;
                let slot_view = match (dual_value, on_pointerdown) {
                    (Some(dual), Some(pd)) => view! {
                        <CustomSlider
                            label=label
                            config=config.clone()
                            value=value
                            dual_value=dual
                            on_handle_pointerdown=pd
                            on_value_change=on_change.clone()
                        />
                    }.into_view().into_any(),
                    (Some(dual), None) => view! {
                        <CustomSlider
                            label=label
                            config=config.clone()
                            value=value
                            dual_value=dual
                            on_value_change=on_change.clone()
                        />
                    }.into_view().into_any(),
                    (None, Some(pd)) => view! {
                        <CustomSlider
                            label=label
                            config=config.clone()
                            value=value
                            on_handle_pointerdown=pd
                            on_value_change=on_change.clone()
                        />
                    }.into_view().into_any(),
                    (None, None) => view! {
                        <CustomSlider
                            label=label
                            config=config.clone()
                            value=value
                            on_value_change=on_change.clone()
                        />
                    }.into_view().into_any(),
                };
                view! {
                    <div style=move || format!("order: {};", flex_order())>
                        {slot_view}
                    </div>
                }
            }).collect_view()}
        </div>
    }
}
