use leptos::mount::mount_to;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

mod format;
mod rotation;

use format::VectorFormat;
use rotation::{AxisAngle, Quaternion, Rotation};

/// Which text box the user is currently editing.
/// While editing, that box's text is driven by the user's keystrokes;
/// all *other* boxes reactively reformat from the shared Rotation.
#[derive(Clone, Copy, PartialEq)]
enum ActiveInput {
    None,
    Quaternion,
    AxisAngle3D,
}

// ---------------------------------------------------------------------------
// Helper: pull the string value out of an <input> event
// ---------------------------------------------------------------------------
fn input_event_value(ev: &leptos::web_sys::Event) -> String {
    ev.target()
        .unwrap()
        .unchecked_into::<leptos::web_sys::HtmlInputElement>()
        .value()
}

// ---------------------------------------------------------------------------
// QuaternionBox
// ---------------------------------------------------------------------------
#[component]
fn QuaternionBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let (is_xyzw, set_is_xyzw) = signal(true);
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0, 1.0]));

    // Reactive effect: reformat whenever the rotation, format, or convention
    // changes — but only if this box is NOT the one the user is typing in.
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        let xyzw = is_xyzw.get();
        if active_input.get() != ActiveInput::Quaternion {
            let q = rot.as_quaternion();
            let values: Vec<f64> = if xyzw {
                vec![q.x as f64, q.y as f64, q.z as f64, q.w as f64]
            } else {
                vec![q.w as f64, q.x as f64, q.y as f64, q.z as f64]
            };
            text.set(fmt.format_vector(&values));
        }
    });

    // Parse user input, update shared format + rotation
    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::Quaternion);

        if let Ok((detected_fmt, nums)) = VectorFormat::detect_and_parse(&value, 4) {
            format.set(detected_fmt);
            let (w, x, y, z) = if is_xyzw.get_untracked() {
                (nums[3] as f32, nums[0] as f32, nums[1] as f32, nums[2] as f32)
            } else {
                (nums[0] as f32, nums[1] as f32, nums[2] as f32, nums[3] as f32)
            };
            if let Ok(q) = Quaternion::try_new(w, x, y, z) {
                rotation.set(Rotation::from(q));
            }
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    // Convention radio buttons also reset active_input so the effect reformats.
    let set_xyzw = move |_: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        set_is_xyzw.set(true);
    };
    let set_wxyz = move |_: leptos::web_sys::Event| {
        active_input.set(ActiveInput::None);
        set_is_xyzw.set(false);
    };

    view! {
        <div>
            <h2>"Quaternion"</h2>
            <div>
                "Convention: "
                <label>
                    <input type="radio" name="quat-convention"
                        prop:checked=move || is_xyzw.get()
                        on:change=set_xyzw
                    /> "xyzw"
                </label>
                <label>
                    <input type="radio" name="quat-convention"
                        prop:checked=move || !is_xyzw.get()
                        on:change=set_wxyz
                    /> "wxyz"
                </label>
            </div>
            <input
                type="text"
                prop:value=move || text.get()
                on:input=on_input
                on:blur=on_blur
            />
        </div>
    }
}

// ---------------------------------------------------------------------------
// AxisAngle3DBox
// ---------------------------------------------------------------------------
#[component]
fn AxisAngle3DBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        if active_input.get() != ActiveInput::AxisAngle3D {
            let aa = rot.as_axis_angle();
            let values = vec![
                (aa.x * aa.angle) as f64,
                (aa.y * aa.angle) as f64,
                (aa.z * aa.angle) as f64,
            ];
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::AxisAngle3D);

        if let Ok((detected_fmt, nums)) = VectorFormat::detect_and_parse(&value, 3) {
            format.set(detected_fmt);
            let (ax, ay, az) = (nums[0] as f32, nums[1] as f32, nums[2] as f32);
            let angle = (ax * ax + ay * ay + az * az).sqrt();
            if angle > 1e-10 {
                let aa = AxisAngle::new(ax / angle, ay / angle, az / angle, angle);
                rotation.set(Rotation::from(aa));
            } else {
                rotation.set(Rotation::default());
            }
        }
    };

    let on_blur = move |_: leptos::web_sys::FocusEvent| {
        active_input.set(ActiveInput::None);
    };

    view! {
        <div>
            <h2>"Axis Angle (3d)"</h2>
            <input
                type="text"
                prop:value=move || text.get()
                on:input=on_input
                on:blur=on_blur
            />
        </div>
    }
}

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------
#[component]
fn App() -> impl IntoView {
    let rotation = RwSignal::new(Rotation::default());
    let format = RwSignal::new(VectorFormat::default());
    let active_input = RwSignal::new(ActiveInput::None);

    view! {
        <h1>"Rotation Visualizer"</h1>
        <QuaternionBox rotation=rotation format=format active_input=active_input />
        <AxisAngle3DBox rotation=rotation format=format active_input=active_input />
    }
}

// ---------------------------------------------------------------------------
// three-d renderer + Leptos mount
// ---------------------------------------------------------------------------

pub fn main() {
    use three_d::*;

    // Mount Leptos to the specific container element
    let leptos_root = leptos::tachys::dom::document()
        .get_element_by_id("leptos-app")
        .expect("should find #leptos-app element")
        .unchecked_into::<leptos::web_sys::HtmlElement>();
    mount_to(leptos_root, App).forget(); // Keep the view mounted permanently

    // Configure three-d to use the specific canvas element
    #[cfg(target_arch = "wasm32")]
    let canvas_element = {
        leptos::tachys::dom::document()
            .get_element_by_id("three-canvas")
            .expect("should find #three-canvas element")
            .unchecked_into::<leptos::web_sys::HtmlCanvasElement>()
    };

    // Sync canvas buffer size with CSS display size to prevent distortion
    #[cfg(target_arch = "wasm32")]
    {
        let dpr = leptos::web_sys::window().unwrap().device_pixel_ratio();
        let css_width = canvas_element.client_width() as f64;
        let css_height = canvas_element.client_height() as f64;
        canvas_element.set_width((css_width * dpr) as u32);
        canvas_element.set_height((css_height * dpr) as u32);
    }

    let window = Window::new(WindowSettings {
        title: "Rotation Visualizer".to_string(),
        #[cfg(target_arch = "wasm32")]
        canvas: Some(canvas_element),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();

    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5.0, 3.0, 2.5),
        vec3(0.0, 0.0, 0.0),
        vec3(0.0, 0.0, 1.0),
        degrees(45.0),
        0.1,
        1000.0,
    );
    let mut control = OrbitControl::new(camera.target(), 1.0, 100.0);

    let axes = Axes::new(&context, 0.1, 2.0);

    let light0 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
    let light1 = DirectionalLight::new(&context, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

    window.render_loop(move |mut frame_input| {
        // Sync canvas buffer size with CSS display size on each frame (handles resize)
        // and create a viewport based on actual canvas dimensions
        #[cfg(target_arch = "wasm32")]
        let canvas_viewport = {
            let canvas = leptos::tachys::dom::document()
                .get_element_by_id("three-canvas")
                .unwrap()
                .unchecked_into::<leptos::web_sys::HtmlCanvasElement>();
            let dpr = leptos::web_sys::window().unwrap().device_pixel_ratio();
            let css_width = canvas.client_width() as f64;
            let css_height = canvas.client_height() as f64;
            let buffer_width = (css_width * dpr) as u32;
            let buffer_height = (css_height * dpr) as u32;
            if canvas.width() != buffer_width || canvas.height() != buffer_height {
                canvas.set_width(buffer_width);
                canvas.set_height(buffer_height);
            }
            Viewport {
                x: 0,
                y: 0,
                width: buffer_width,
                height: buffer_height,
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        let canvas_viewport = frame_input.viewport;

        camera.set_viewport(canvas_viewport);
        control.handle_events(&mut camera, &mut frame_input.events);

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
            .render(&camera, &axes, &[&light0, &light1]);

        FrameOutput::default()
    });
}
