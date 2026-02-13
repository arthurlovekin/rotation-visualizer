use std::cell::RefCell;
use std::rc::Rc;

use leptos::mount::mount_to;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

mod format;
mod rotation;

use format::{parse_vector_and_format, VectorFormat};
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
            let values: Vec<f32> = if xyzw {
                vec![q.x as f32, q.y as f32, q.z as f32, q.w as f32]
            } else {
                vec![q.w as f32, q.x as f32, q.y as f32, q.z as f32]
            };
            text.set(fmt.format_vector(&values));
        }
    });

    // Parse user input, update shared format + rotation
    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::Quaternion);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<4>(&value) {
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
                (aa.x * aa.angle) as f32,
                (aa.y * aa.angle) as f32,
                (aa.z * aa.angle) as f32,
            ];
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::AxisAngle3D);

        if let Ok((nums, detected_fmt)) = parse_vector_and_format::<3>(&value) {
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
fn App(
    #[prop(optional)] rotation_for_renderer: Option<Rc<RefCell<Rotation>>>,
) -> impl IntoView {
    let rotation = RwSignal::new(Rotation::default());
    let format = RwSignal::new(VectorFormat::default());
    let active_input = RwSignal::new(ActiveInput::None);

    // Sync rotation to the three-d renderer each time it changes
    if let Some(shared) = rotation_for_renderer {
        Effect::new(move || {
            let rot = rotation.get();
            *shared.borrow_mut() = rot;
        });
    }

    view! {
        <h1>"Rotation Visualizer"</h1>
        <QuaternionBox rotation=rotation format=format active_input=active_input />
        <AxisAngle3DBox rotation=rotation format=format active_input=active_input />
    }
}

// ---------------------------------------------------------------------------
// three-d renderer + Leptos mount
// ---------------------------------------------------------------------------

/// Converts our Rotation to a three-d Mat4 (4x4 rotation matrix, column-major).
fn rotation_to_mat4(rot: &Rotation) -> three_d::Mat4 {
    use three_d::*;
    let m = rot.as_rotation_matrix();
    // cgmath Mat4::new is column-major: c0r0, c0r1, c0r2, c0r3, c1r0, ...
    Mat4::new(
        m.0[0][0], m.0[1][0], m.0[2][0], 0.0,
        m.0[0][1], m.0[1][1], m.0[2][1], 0.0,
        m.0[0][2], m.0[1][2], m.0[2][2], 0.0,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// Load assets in WASM using gloo-net fetch (three-d-asset's load_async uses reqwest which doesn't work on WASM).
#[cfg(target_arch = "wasm32")]
async fn load_assets_wasm(
    paths: &[&str],
) -> Result<three_d_asset::io::RawAssets, String> {
    use three_d_asset::io::RawAssets;

    let mut raw = RawAssets::new();
    for path in paths {
        let response = gloo_net::http::Request::get(*path)
            .send()
            .await
            .map_err(|e| format!("{}: {}", path, e))?;
        if !response.ok() {
            return Err(format!("{}: HTTP status {}", path, response.status()));
        }
        let bytes: Vec<u8> = response
            .binary()
            .await
            .map_err(|e| format!("{}: {}", path, e))?;
        raw.insert(path, bytes);
    }
    Ok(raw)
}

#[cfg(target_arch = "wasm32")]
fn run_three_d(rotation_for_renderer: Rc<RefCell<Rotation>>) {
    use three_d::*;

    wasm_bindgen_futures::spawn_local(async move {
        let canvas_element = leptos::tachys::dom::document()
            .get_element_by_id("three-canvas")
            .expect("should find #three-canvas element")
            .unchecked_into::<leptos::web_sys::HtmlCanvasElement>();

        let dpr = leptos::web_sys::window().unwrap().device_pixel_ratio();
        let css_width = canvas_element.client_width() as f64;
        let css_height = canvas_element.client_height() as f64;
        canvas_element.set_width((css_width * dpr) as u32);
        canvas_element.set_height((css_height * dpr) as u32);

        let window = Window::new(WindowSettings {
            title: "Rotation Visualizer".to_string(),
            canvas: Some(canvas_element),
            ..Default::default()
        })
        .unwrap();
        let context = window.gl();

        // Load suzanne_monkey mesh (async)
        // three-d-asset's load_async uses reqwest which doesn't work on WASM, so we fetch manually
        let mut mesh_objects: Option<(Gm<Mesh, PhysicalMaterial>, Gm<Mesh, PhysicalMaterial>)> =
            match load_assets_wasm(&["assets/suzanne_monkey.obj", "assets/suzanne_monkey.mtl"]).await
            {
                Ok(mut loaded) => {
                    match loaded.deserialize::<three_d::CpuMesh>("assets/suzanne_monkey.obj") {
                        Ok(mut cpu_mesh) => {
                            let scale = 1.5;
                            if let Err(e) = cpu_mesh.transform(three_d::Mat4::from_scale(scale)) {
                                log::warn!("Mesh transform failed: {:?}", e);
                            }

                            let mut gray_material = PhysicalMaterial::new_opaque(
                                &context,
                                &CpuMaterial {
                                    albedo: Srgba::new_opaque(100, 100, 100),
                                    roughness: 0.7,
                                    metallic: 0.3,
                                    ..Default::default()
                                },
                            );
                            gray_material.render_states.cull = Cull::Back;

                            let mut white_material = PhysicalMaterial::new_opaque(
                                &context,
                                &CpuMaterial {
                                    albedo: Srgba::new_opaque(220, 220, 220),
                                    roughness: 0.7,
                                    metallic: 0.3,
                                    ..Default::default()
                                },
                            );
                            white_material.render_states.cull = Cull::Back;

                            let mut mesh_unrotated = Mesh::new(&context, &cpu_mesh);
                            mesh_unrotated.set_transformation(three_d::Mat4::identity());

                            let mut mesh_rotated = Mesh::new(&context, &cpu_mesh);
                            mesh_rotated.set_transformation(three_d::Mat4::identity());

                            Some((
                                Gm::new(mesh_unrotated, gray_material),
                                Gm::new(mesh_rotated, white_material),
                            ))
                        }
                        Err(e) => {
                            log::error!("Failed to deserialize suzanne_monkey.obj: {:?}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::error!("Failed to load assets (check assets/ in dist): {:?}", e);
                    None
                }
            };

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

            camera.set_viewport(canvas_viewport);
            control.handle_events(&mut camera, &mut frame_input.events);

            match &mut mesh_objects {
                Some((model_unrotated, model_rotated)) => {
                    let rot = rotation_for_renderer.borrow();
                    model_rotated.geometry.set_transformation(rotation_to_mat4(&rot));
                    frame_input
                        .screen()
                        .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
                        .render(
                            &camera,
                            (&*model_unrotated)
                                .into_iter()
                                .chain(&*model_rotated)
                                .chain(&axes),
                            &[&light0, &light1],
                        );
                }
                None => {
                    frame_input
                        .screen()
                        .clear(ClearState::color_and_depth(0.0, 0.0, 0.0, 1.0, 1.0))
                        .render(&camera, &axes, &[&light0, &light1]);
                }
            }

            FrameOutput {
                swap_buffers: true,
                ..Default::default()
            }
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn run_three_d(rotation_for_renderer: Rc<RefCell<Rotation>>) {
    // Native build: synchronous loading, no spawn_local needed
    unimplemented!("suzanne_monkey visualization is WASM-only for now; use trunk serve");
}

pub fn main() {
    let rotation_for_renderer = Rc::new(RefCell::new(Rotation::default()));
    let rotation_for_app = rotation_for_renderer.clone();

    let leptos_root = leptos::tachys::dom::document()
        .get_element_by_id("leptos-app")
        .expect("should find #leptos-app element")
        .unchecked_into::<leptos::web_sys::HtmlElement>();

    mount_to(leptos_root, move || {
        view! {
            <App rotation_for_renderer=rotation_for_app.clone() />
        }
    })
    .forget();

    run_three_d(rotation_for_renderer);
}
