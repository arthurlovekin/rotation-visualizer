//! Quaternion slider group with LRU-based normalization.
//!
//! Uses event-driven updates (on_value_change) instead of reactive effects
//! during drag, avoiding cascade lag. Rotation->sliders sync only when
//! rotation changes from text input or on pointerup.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

use super::normalize::{normalize_lru, touch_order, X, Y, Z, W};
use crate::app::rotation::{Quaternion, Rotation};
use crate::app::slider_widget::{MultiHandleSlider, MultiHandleSliderConfig};


#[component]
pub fn QuaternionSliderGroup(
    rotation: RwSignal<Rotation>,
    format_config: MultiHandleSliderConfig,
    /// true = xyzw (x,y,z,w), false = wxyz (w,x,y,z)
    is_xyzw: RwSignal<bool>,
) -> impl IntoView {
    let quat_x = RwSignal::new(0.0_f64);
    let quat_y = RwSignal::new(0.0_f64);
    let quat_z = RwSignal::new(0.0_f64);
    let quat_w = RwSignal::new(1.0_f64);

    let dual_x = Memo::new(move |_| -quat_x.get());
    let dual_y = Memo::new(move |_| -quat_y.get());
    let dual_z = Memo::new(move |_| -quat_z.get());
    let dual_w = Memo::new(move |_| -quat_w.get());

    let order = Rc::new(RefCell::new([X, Y, Z, W]));
    let active_slider: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));

    // Sync rotation -> sliders when rotation changes (text input) or on pointerup.
    Effect::new(move || {
        let rot = rotation.get();
        let q = rot.as_quaternion();
        batch(move || {
            quat_x.set(q.x as f64);
            quat_y.set(q.y as f64);
            quat_z.set(q.z as f64);
            quat_w.set(q.w as f64);
        });
    });

    // Event-driven: called when user drags. Throttled to ~60fps via RAF.
    let handle_value_change: Rc<dyn Fn(usize, f64)> = Rc::new({
        let order = order.clone();
        let rotation = rotation.clone();
        let pending: Rc<RefCell<Option<(usize, f64)>>> = Rc::new(RefCell::new(None));
        let raf_scheduled: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));

        move |component_index: usize, value: f64| {
            *pending.borrow_mut() = Some((component_index, value));

            if *raf_scheduled.borrow() {
                return;
            }
            *raf_scheduled.borrow_mut() = true;

            let pending = pending.clone();
            let order = order.clone();
            let rotation = rotation.clone();
            let raf_scheduled = raf_scheduled.clone();
            let qx = quat_x;
            let qy = quat_y;
            let qz = quat_z;
            let qw = quat_w;

            let f = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                *raf_scheduled.borrow_mut() = false;
                let Some((idx, _)) = pending.borrow_mut().take() else { return };
                let values = [
                    qx.get_untracked(),
                    qy.get_untracked(),
                    qz.get_untracked(),
                    qw.get_untracked(),
                ];
                let ord = *order.borrow();
                let normalized = normalize_lru(values, idx, &ord);
                let new_rot = match Quaternion::try_new(
                    normalized[W] as f32,
                    normalized[X] as f32,
                    normalized[Y] as f32,
                    normalized[Z] as f32,
                ) {
                    Ok(q) => Rotation::from(q),
                    Err(_) => Rotation::default(),
                };
                rotation.set(new_rot);
            }) as Box<dyn FnMut()>);

            leptos::web_sys::window()
                .and_then(|w| w.request_animation_frame(f.as_ref().unchecked_ref()).ok())
                .expect("requestAnimationFrame");
            std::mem::forget(f);
        }
    });

    let on_x = {
        let order = order.clone();
        let active_slider = active_slider.clone();
        move |_| {
            touch_order(&mut order.borrow_mut(), X);
            *active_slider.borrow_mut() = Some(X);
        }
    };
    let on_y = {
        let order = order.clone();
        let active_slider = active_slider.clone();
        move |_| {
            touch_order(&mut order.borrow_mut(), Y);
            *active_slider.borrow_mut() = Some(Y);
        }
    };
    let on_z = {
        let order = order.clone();
        let active_slider = active_slider.clone();
        move |_| {
            touch_order(&mut order.borrow_mut(), Z);
            *active_slider.borrow_mut() = Some(Z);
        }
    };
    let on_w = {
        let order = order.clone();
        let active_slider = active_slider.clone();
        move |_| {
            touch_order(&mut order.borrow_mut(), W);
            *active_slider.borrow_mut() = Some(W);
        }
    };

    // Clear active on pointerup so rotation->sliders effect can sync from text input
    #[cfg(target_arch = "wasm32")]
    Effect::new(move || {
        let document = leptos::web_sys::window()
            .expect("window")
            .document()
            .expect("document");
        let active_slider = active_slider.clone();
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(
            move |_ev: leptos::web_sys::PointerEvent| {
                *active_slider.borrow_mut() = None;
            },
        ) as Box<dyn FnMut(_)>);
        document
            .add_event_listener_with_callback("pointerup", closure.as_ref().unchecked_ref())
            .expect("add pointerup listener");
        std::mem::forget(closure);
    });

    let on_change_x = {
        let h = handle_value_change.clone();
        move |_: usize, v: f64| h(X, v)
    };
    let on_change_y = {
        let h = handle_value_change.clone();
        move |_: usize, v: f64| h(Y, v)
    };
    let on_change_z = {
        let h = handle_value_change.clone();
        move |_: usize, v: f64| h(Z, v)
    };
    let on_change_w = {
        let h = handle_value_change.clone();
        move |_: usize, v: f64| h(W, v)
    };

    // Render sliders in order matching convention: xyzw = x,y,z,w; wxyz = w,x,y,z
    // Use flexbox order to reorder without changing the DOM structure (avoids closure type mismatch)
    view! {
        <div class="quaternion-sliders" style="display: flex; flex-direction: column;">
            <div style=move || format!("order: {};", if is_xyzw.get() { 0 } else { 1 })>
                <MultiHandleSlider label="x" config=format_config.clone() values=vec![quat_x] dual_values=vec![dual_x] on_handle_pointerdown=on_x on_value_change=on_change_x />
            </div>
            <div style=move || format!("order: {};", if is_xyzw.get() { 1 } else { 2 })>
                <MultiHandleSlider label="y" config=format_config.clone() values=vec![quat_y] dual_values=vec![dual_y] on_handle_pointerdown=on_y on_value_change=on_change_y />
            </div>
            <div style=move || format!("order: {};", if is_xyzw.get() { 2 } else { 3 })>
                <MultiHandleSlider label="z" config=format_config.clone() values=vec![quat_z] dual_values=vec![dual_z] on_handle_pointerdown=on_z on_value_change=on_change_z />
            </div>
            <div style=move || format!("order: {};", if is_xyzw.get() { 3 } else { 0 })>
                <MultiHandleSlider label="w" config=format_config.clone() values=vec![quat_w] dual_values=vec![dual_w] on_handle_pointerdown=on_w on_value_change=on_change_w />
            </div>
        </div>
    }
}
