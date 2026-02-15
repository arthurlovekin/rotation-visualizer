use std::cell::RefCell;
use std::rc::Rc;

use leptos::mount::mount_to;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

mod format;
mod quaternion;
mod quaternion_slider_group;
mod rotation;
mod slider_widget;

use format::{parse_vector_and_format, VectorFormat};
use quaternion_slider_group::QuaternionSliderGroup;
use rotation::{AxisAngle, Quaternion, Rotation};
use slider_widget::{MultiHandleSliderConfig, SliderMarker};

/// App-specific slider config constructors. Kept in app.rs so slider_widget remains reusable.
impl MultiHandleSliderConfig {
    /// Angle slider [0, 2π] with 0, π, 2π markers.
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

    /// Quaternion component slider [-1, 1].
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

/// Which text box the user is currently editing.
/// While editing, that box's text is driven by the user's keystrokes;
/// all *other* boxes reactively reformat from the shared Rotation.
#[derive(Clone, Copy, PartialEq)]
enum ActiveInput {
    None,
    Quaternion,
    RotationVector,
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

    let quat_config = MultiHandleSliderConfig::quaternion_component();

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
            <QuaternionSliderGroup rotation=rotation format_config=quat_config />
        </div>
    }
}

// ---------------------------------------------------------------------------
// RotationVectorBox
// ---------------------------------------------------------------------------
#[component]
fn RotationVectorBox(
    rotation: RwSignal<Rotation>,
    format: RwSignal<VectorFormat>,
    active_input: RwSignal<ActiveInput>,
) -> impl IntoView {
    let text = RwSignal::new(format.get_untracked().format_vector(&[0.0, 0.0, 0.0]));

    // Reactive effect: reformat when rotation/format changes (if not editing).
    Effect::new(move || {
        let rot = rotation.get();
        let fmt = format.get();
        if active_input.get() != ActiveInput::RotationVector {
            let rv = rot.as_rotation_vector();
            let values = vec![
                rv.x as f32,
                rv.y as f32,
                rv.z as f32,
            ];
            text.set(fmt.format_vector(&values));
        }
    });

    let on_input = move |ev: leptos::web_sys::Event| {
        let value = input_event_value(&ev);
        text.set(value.clone());
        active_input.set(ActiveInput::RotationVector);

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

/// Callback to request a 3D canvas redraw (used for reactive rendering).
pub type RequestRedraw = Rc<dyn Fn()>;

// ---------------------------------------------------------------------------
// App root
// ---------------------------------------------------------------------------
#[component]
fn App(
    #[prop(optional)] rotation_for_renderer: Option<Rc<RefCell<Rotation>>>,
    #[prop(optional)] request_redraw: Option<RequestRedraw>,
) -> impl IntoView {
    let rotation = RwSignal::new(Rotation::default());
    let format = RwSignal::new(VectorFormat::default());
    let active_input = RwSignal::new(ActiveInput::None);

    // Sync rotation to the three-d renderer and request redraw when it changes
    if let Some(shared) = rotation_for_renderer {
        let redraw = request_redraw;
        Effect::new(move || {
            let rot = rotation.get();
            *shared.borrow_mut() = rot;
            if let Some(ref r) = redraw {
                r();
            }
        });
    }

    view! {
        <h1>"Rotation Visualizer"</h1>
        <QuaternionBox rotation=rotation format=format active_input=active_input />
        <RotationVectorBox rotation=rotation format=format active_input=active_input />
    }
}

// ---------------------------------------------------------------------------
// three-d renderer + Leptos mount
// ---------------------------------------------------------------------------

/// Build edge transformations for wireframe rendering (cylinders along each mesh edge).
fn edge_transformations(cpu_mesh: &three_d::CpuMesh) -> three_d::Instances {
    use three_d::*;
    let indices = cpu_mesh.indices.to_u32().unwrap();
    let positions = cpu_mesh.positions.to_f32();
    let mut transformations = Vec::new();
    for f in 0..indices.len() / 3 {
        let i1 = indices[3 * f] as usize;
        let i2 = indices[3 * f + 1] as usize;
        let i3 = indices[3 * f + 2] as usize;
        if i1 < i2 {
            transformations.push(edge_transform(positions[i1], positions[i2]));
        }
        if i2 < i3 {
            transformations.push(edge_transform(positions[i2], positions[i3]));
        }
        if i3 < i1 {
            transformations.push(edge_transform(positions[i3], positions[i1]));
        }
    }
    Instances {
        transformations,
        ..Default::default()
    }
}

fn edge_transform(p1: three_d::Vec3, p2: three_d::Vec3) -> three_d::Mat4 {
    use three_d::*;
    Mat4::from_translation(p1)
        * Mat4::from(Quat::from_arc(
            vec3(1.0, 0.0, 0.0),
            (p2 - p1).normalize(),
            None,
        ))
        * Mat4::from_nonuniform_scale((p2 - p1).magnitude(), 1.0, 1.0)
}

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

/// Returns true if the window event should trigger a redraw (user interaction with 3D view).
fn window_event_needs_redraw(event: &winit::event::WindowEvent) -> bool {
    use winit::event::WindowEvent;
    matches!(
        event,
        WindowEvent::CursorMoved { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Touch(_)
            | WindowEvent::TouchpadMagnify { .. }
            | WindowEvent::TouchpadRotate { .. }
            | WindowEvent::Resized(_)
            | WindowEvent::ScaleFactorChanged { .. }
    )
}

#[cfg(target_arch = "wasm32")]
fn run_three_d(
    rotation_for_renderer: Rc<RefCell<Rotation>>,
    request_redraw: RequestRedraw,
    event_loop: winit::event_loop::EventLoop<()>,
) {
    use three_d::*;
    use winit::event::{Event, WindowEvent};
    use winit::event_loop::ControlFlow;
    use winit::platform::web::WindowBuilderExtWebSys;
    use winit::window::WindowBuilder;

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

        let inner_size = winit::dpi::LogicalSize::new(css_width, css_height);
        let window = WindowBuilder::new()
            .with_title("Rotation Visualizer".to_string())
            .with_canvas(Some(canvas_element))
            .with_inner_size(inner_size)
            .with_prevent_default(true)
            .build(&event_loop)
            .expect("failed to create window");
        window.focus_window();

        let surface_settings = SurfaceSettings::default();
        let gl = WindowedContext::from_winit_window(&window, surface_settings)
            .or_else(|_| {
                let mut fallback = surface_settings;
                fallback.multisamples = 0;
                WindowedContext::from_winit_window(&window, fallback)
            })
            .expect("failed to create WebGL context");

        let mut frame_input_generator = FrameInputGenerator::from_winit_window(&window);

        // Load suzanne_monkey mesh (async)
        let mut mesh_objects: Option<(
            Gm<InstancedMesh, PhysicalMaterial>,
            Gm<Mesh, PhysicalMaterial>,
        )> = match load_assets_wasm(&["assets/suzanne_monkey.obj", "assets/suzanne_monkey.mtl"])
            .await
        {
            Ok(mut loaded) => {
                match loaded.deserialize::<three_d::CpuMesh>("assets/suzanne_monkey.obj") {
                    Ok(mut cpu_mesh) => {
                        let scale = 1.5;
                        if let Err(e) = cpu_mesh.transform(three_d::Mat4::from_scale(scale)) {
                            log::warn!("Mesh transform failed: {:?}", e);
                        }

                        let mut wireframe_material = PhysicalMaterial::new_transparent(
                            &gl,
                            &CpuMaterial {
                                albedo: Srgba::new(153, 153, 153, 128),
                                roughness: 0.7,
                                metallic: 0.3,
                                ..Default::default()
                            },
                        );
                        wireframe_material.render_states.cull = Cull::Back;

                        let mut cylinder = CpuMesh::cylinder(10);
                        cylinder
                            .transform(three_d::Mat4::from_nonuniform_scale(1.0, 0.007, 0.007))
                            .expect("cylinder transform");
                        let wireframe_unrotated = Gm::new(
                            InstancedMesh::new(&gl, &edge_transformations(&cpu_mesh), &cylinder),
                            wireframe_material,
                        );

                        let mut white_material = PhysicalMaterial::new_opaque(
                            &gl,
                            &CpuMaterial {
                                albedo: Srgba::new_opaque(220, 220, 220),
                                roughness: 0.7,
                                metallic: 0.3,
                                ..Default::default()
                            },
                        );
                        white_material.render_states.cull = Cull::Back;

                        let mut mesh_rotated = Mesh::new(&gl, &cpu_mesh);
                        mesh_rotated.set_transformation(three_d::Mat4::identity());

                        Some((wireframe_unrotated, Gm::new(mesh_rotated, white_material)))
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

        let (w, h): (u32, u32) = window.inner_size().into();
        let viewport = Viewport::new_at_origo(w, h);
        let mut camera = Camera::new_perspective(
            viewport,
            vec3(5.0, 3.0, 2.5),
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 1.0),
            degrees(45.0),
            0.1,
            1000.0,
        );
        let mut control = OrbitControl::new(camera.target(), 1.0, 100.0);

        let axes = Axes::new(&gl, 0.1, 2.0);
        let light0 = DirectionalLight::new(&gl, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&gl, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

        // Request initial render (rotation Effect will also trigger on mount)
        request_redraw();

        event_loop.run(move |event, _, control_flow| {
            match &event {
                Event::UserEvent(()) => {
                    // Rotation changed from Leptos - request a redraw
                    window.request_redraw();
                }
                Event::MainEventsCleared => {
                    // Reactive loop: do NOT request redraw here. We only redraw on
                    // UserEvent (rotation change) or WindowEvent (user interaction).
                }
                Event::RedrawRequested(_) => {
                    #[cfg(target_arch = "wasm32")]
                    {
                        use winit::platform::web::WindowExtWebSys;
                        let html_canvas = window.canvas();
                        let browser_window = html_canvas
                            .owner_document()
                            .and_then(|doc| doc.default_view())
                            .or_else(leptos::web_sys::window)
                            .unwrap();
                        window.set_inner_size(winit::dpi::LogicalSize {
                            width: browser_window.inner_width().unwrap().as_f64().unwrap(),
                            height: browser_window.inner_height().unwrap().as_f64().unwrap(),
                        });
                    }

                    let mut frame_input = frame_input_generator.generate(&gl);
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

                    if option_env!("THREE_D_SCREENSHOT").is_none() {
                        let _ = gl.swap_buffers();
                    }

                    // Reactive: wait for next event instead of continuous 60 FPS
                    *control_flow = ControlFlow::Wait;
                }
                Event::WindowEvent { event, .. } => {
                    frame_input_generator.handle_winit_window_event(event);
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            gl.resize(*physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            gl.resize(**new_inner_size);
                        }
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                    if window_event_needs_redraw(event) {
                        window.request_redraw();
                    }
                }
                _ => {}
            }
        });
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn run_three_d(_rotation_for_renderer: Rc<RefCell<Rotation>>) {
    // Native build: synchronous loading, no spawn_local needed
    unimplemented!("suzanne_monkey visualization is WASM-only for now; use trunk serve");
}

pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        use winit::event_loop::EventLoop;

        let rotation_for_renderer = Rc::new(RefCell::new(Rotation::default()));
        let rotation_for_app = rotation_for_renderer.clone();

        let event_loop = EventLoop::new();
        let redraw_proxy = event_loop.create_proxy();
        let request_redraw: RequestRedraw = Rc::new(move || {
            let _ = redraw_proxy.send_event(());
        });
        let request_redraw_for_app = request_redraw.clone();

        let leptos_root = leptos::tachys::dom::document()
            .get_element_by_id("leptos-app")
            .expect("should find #leptos-app element")
            .unchecked_into::<leptos::web_sys::HtmlElement>();

        mount_to(leptos_root, move || {
            view! {
                <App rotation_for_renderer=rotation_for_app.clone() request_redraw=request_redraw_for_app.clone() />
            }
        })
        .forget();

        run_three_d(rotation_for_renderer, request_redraw, event_loop);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
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
}
