use leptos::mount::mount_to;
use leptos::prelude::*;
mod rotation;

#[component]
fn QuaternionBox() -> impl IntoView {
    // Convention state: true = xyzw, false = wxyz
    let (is_xyzw, set_is_xyzw) = signal(true);

    view! {
        <div>
            <h2>"Quaternion"</h2>
            <div>
                "Convention: "
                <label>
                    <input type="radio" name="quat-convention"
                        checked={is_xyzw.get()}
                        on:change=move |_| set_is_xyzw.set(true)
                    /> "xyzw"
                </label>
                <label>
                    <input type="radio" name="quat-convention"
                        checked={!is_xyzw.get()}
                        on:change=move |_| set_is_xyzw.set(false)
                    /> "wxyz"
                </label>
            </div>
            <input
                type="text"
                value="[0.0,0.0,0.0,1.0]"
            />
        </div>
    }
}

#[component]
fn AxisAngle3DBox() -> impl IntoView {
    view! {
        <div>
            <h2>"Axis Angle (3d)"</h2>
        </div>
        <input
            type="text"
            value="[0.0,0.0,0.0]"
        />
    }
}

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = signal(0);    
    view! {
        <h1>"Rotation Visualizer"</h1>

        <QuaternionBox />
        <AxisAngle3DBox />
    }
}

use three_d::*;

pub fn main() {
    // Mount Leptos to the specific container element
    use leptos::wasm_bindgen::JsCast;
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
            .render(
                &camera,
                &axes,
                &[&light0, &light1],
            );

        FrameOutput::default()
    });
}
