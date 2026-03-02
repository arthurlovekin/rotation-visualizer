use std::cell::RefCell;
use std::rc::Rc;

use leptos::mount::mount_to;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

mod axis_angle;
mod axis_angle_flag;
mod collapsible_section;
mod dom;
mod format;
mod normalize;
mod quaternion;
mod rotation;
mod rotation_matrix;
mod rotation_vector;
mod slider_group;
mod slider_widget;

use axis_angle::AxisAngleBox;
use format::{MatrixFormat, VectorFormat};
use quaternion::QuaternionBox;
use rotation_matrix::RotationMatrixBox;
use rotation_vector::RotationVectorBox;
use rotation::Rotation;
use slider_widget::{CustomSliderConfig, SliderMarker};

/// App-specific slider config constructors. Kept in app.rs so slider_widget remains reusable.
impl CustomSliderConfig {
    fn with_markers(min: f64, max: f64, markers: &[(f64, &str)]) -> Self {
        Self {
            min,
            max,
            markers: markers
                .iter()
                .map(|(v, l)| SliderMarker { value: *v, label: l.to_string() })
                .collect(),
        }
    }

    pub fn angle_2pi() -> Self {
        let pi = std::f64::consts::PI;
        Self::with_markers(0.0, 2.0 * pi, &[(0.0, "0"), (pi, "π"), (2.0 * pi, "2π")])
    }

    pub fn angle_0_pi() -> Self {
        let pi = std::f64::consts::PI;
        Self::with_markers(0.0, pi, &[(0.0, "0"), (pi / 2.0, "π/2"), (pi, "π")])
    }

    pub fn angle_rad_neg_pi_2pi() -> Self {
        let pi = std::f64::consts::PI;
        Self::with_markers(
            -pi,
            2.0 * pi,
            &[(-pi, "-π"), (0.0, "0"), (pi / 2.0, "π/2"), (pi, "π"), (2.0 * pi, "2π")],
        )
    }

    pub fn angle_deg_neg180_360() -> Self {
        Self::with_markers(
            -180.0,
            360.0,
            &[(-180.0, "-180°"), (0.0, "0°"), (90.0, "90°"), (180.0, "180°"), (360.0, "360°")],
        )
    }

    pub fn quaternion_component() -> Self {
        Self::with_markers(-1.0, 1.0, &[(-1.0, "-1"), (0.0, "0"), (1.0, "1")])
    }

    pub fn rotation_vector_component() -> Self {
        let pi = std::f64::consts::PI;
        Self::with_markers(
            -2.0 * pi,
            2.0 * pi,
            &[
                (-2.0 * pi, "-2π"),
                (-pi, "-π"),
                (-pi / 2.0, "-π/2"),
                (0.0, "0"),
                (pi / 2.0, "π/2"),
                (pi, "π"),
                (2.0 * pi, "2π"),
            ],
        )
    }

    pub fn rotation_vector_component_degrees() -> Self {
        Self::with_markers(
            -360.0,
            360.0,
            &[
                (-360.0, "-360°"),
                (-180.0, "-180°"),
                (-90.0, "-90°"),
                (0.0, "0°"),
                (90.0, "90°"),
                (180.0, "180°"),
                (360.0, "360°"),
            ],
        )
    }

    pub fn angle_degrees() -> Self {
        Self::with_markers(
            0.0,
            360.0,
            &[(0.0, "0°"), (90.0, "90°"), (180.0, "180°"), (270.0, "270°"), (360.0, "360°")],
        )
    }

    pub fn angle_degrees_0_180() -> Self {
        Self::with_markers(0.0, 180.0, &[(0.0, "0°"), (90.0, "90°"), (180.0, "180°")])
    }
}

/// Which text box the user is currently editing.
/// While editing, that box's text is driven by the user's keystrokes;
/// all *other* boxes reactively reformat from the shared Rotation.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ActiveInput {
    None,
    Quaternion,
    RotationVector,
    RotationMatrix,
    AxisAngle,
}

/// Callback to request a 3D canvas redraw (used for reactive rendering).
pub type RequestRedraw = Rc<dyn Fn()>;

#[cfg(target_arch = "wasm32")]
fn setup_resize_handle(request_redraw: RequestRedraw) {
    use leptos::wasm_bindgen::closure::Closure;
    use leptos::web_sys::{Element, HtmlElement, MouseEvent};

    let window = leptos::web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let container = document
        .query_selector(".container")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlElement>().ok());
    let handle = document
        .get_element_by_id("resize-handle")
        .and_then(|el| el.dyn_into::<Element>().ok());

    let (container, handle) = match (container, handle) {
        (Some(c), Some(h)) => (c, h),
        _ => return,
    };

    // Restore saved width
    if let Ok(Some(storage)) = window.local_storage() {
        if let Ok(Some(saved)) = storage.get_item("rotation-visualizer-panel-width") {
            if let Ok(pct) = saved.parse::<f64>() {
                let clamped = pct.clamp(20.0, 80.0);
                let _ = container.style().set_property("--panel-left-width", &format!("{}%", clamped));
                let _ = storage.set_item("rotation-visualizer-panel-width", &clamped.to_string());
            }
        }
    }

    let is_dragging = Rc::new(RefCell::new(false));

    let down_closure = Closure::wrap(Box::new({
        let is_dragging = is_dragging.clone();
        move |ev: MouseEvent| {
            if ev.button() != 0 {
                return;
            }
            ev.prevent_default();
            *is_dragging.borrow_mut() = true;
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    let move_closure = Closure::wrap(Box::new({
        let container = container.clone();
        let is_dragging = is_dragging.clone();
        let request_redraw = request_redraw.clone();
        move |ev: MouseEvent| {
            if !*is_dragging.borrow() {
                return;
            }
            let rect = container.get_bounding_client_rect();
            let pct = ((ev.client_x() as f64 - rect.left()) / rect.width()) * 100.0;
            let clamped = pct.clamp(20.0, 80.0);
            let _ = container.style().set_property("--panel-left-width", &format!("{}%", clamped));
            if let Some(w) = leptos::web_sys::window() {
                if let Ok(Some(storage)) = w.local_storage() {
                    let _ = storage.set_item("rotation-visualizer-panel-width", &clamped.to_string());
                }
            }
            request_redraw();
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    let up_closure = Closure::wrap(Box::new({
        let is_dragging = is_dragging.clone();
        let request_redraw = request_redraw.clone();
        move |_ev: MouseEvent| {
            if *is_dragging.borrow() {
                request_redraw();
            }
            *is_dragging.borrow_mut() = false;
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    let leave_closure = Closure::wrap(Box::new({
        let is_dragging = is_dragging.clone();
        move |_ev: MouseEvent| {
            *is_dragging.borrow_mut() = false;
        }
    }) as Box<dyn FnMut(MouseEvent)>);

    let _ = handle.add_event_listener_with_callback("mousedown", down_closure.as_ref().unchecked_ref());
    let _ = document.add_event_listener_with_callback("mousemove", move_closure.as_ref().unchecked_ref());
    let _ = document.add_event_listener_with_callback("mouseup", up_closure.as_ref().unchecked_ref());
    let _ = document.add_event_listener_with_callback("mouseleave", leave_closure.as_ref().unchecked_ref());

    down_closure.forget();
    move_closure.forget();
    up_closure.forget();
    leave_closure.forget();
}

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
    let mut matrix_fmt = MatrixFormat::default();
    matrix_fmt.row_delimiter = '\n';
    let matrix_format = RwSignal::new(matrix_fmt);
    let active_input = RwSignal::new(ActiveInput::None);

    // Sync rotation to the three-d renderer and request redraw when it changes
    if let Some(shared) = rotation_for_renderer {
        let redraw = request_redraw.clone();
        Effect::new(move || {
            let rot = rotation.get();
            *shared.borrow_mut() = rot;
            if let Some(ref r) = redraw {
                r();
            }
        });
    }

    // Set up draggable resize handle (WASM only, runs once on mount)
    #[cfg(target_arch = "wasm32")]
    if let Some(redraw) = request_redraw.clone() {
        wasm_bindgen_futures::spawn_local(async move {
            setup_resize_handle(redraw);
        });
    }

    view! {
        <h1>"Rotation Visualizer"</h1>
        <AxisAngleBox rotation=rotation format=format active_input=active_input />
        <RotationVectorBox rotation=rotation format=format active_input=active_input />
        <QuaternionBox rotation=rotation format=format active_input=active_input />
        <RotationMatrixBox rotation=rotation format=matrix_format active_input=active_input />
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

/// Base instance transforms for axes: X (identity), Y (rotate 90° about Z), Z (rotate -90° about Y).
fn axes_base_instances() -> three_d::Instances {
    use three_d::*;
    Instances {
        transformations: vec![
            Mat4::identity(),
            Mat4::from_angle_z(degrees(90.0)),
            Mat4::from_angle_y(degrees(-90.0)),
        ],
        texture_transformations: None,
        colors: Some(vec![Srgba::RED, Srgba::GREEN, Srgba::BLUE]),
    }
}

/// Build Instances for body-fixed axes: rot_mat * base_transform for each axis.
fn body_axes_instances(rot_mat: three_d::Mat4) -> three_d::Instances {
    use three_d::*;
    let base = axes_base_instances();
    Instances {
        transformations: base
            .transformations
            .iter()
            .map(|t| rot_mat * *t)
            .collect(),
        texture_transformations: base.texture_transformations,
        colors: base.colors,
    }
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
#[cfg(target_arch = "wasm32")]
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
        let mut axes_body = Axes::new(&gl, 0.08, 1.5);
        let light0 = DirectionalLight::new(&gl, 1.0, Srgba::WHITE, vec3(0.0, -0.5, -0.5));
        let light1 = DirectionalLight::new(&gl, 1.0, Srgba::WHITE, vec3(0.0, 0.5, 0.5));

        let mut axis_angle_flag = axis_angle_flag::AxisAngleFlag::new(&gl);

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

                    let rot = rotation_for_renderer.borrow();
                    let rot_mat = rotation_to_mat4(&rot);
                    axes_body.geometry.set_transformation(Mat4::identity());
                    axes_body.geometry.set_instances(&body_axes_instances(rot_mat));

                    axis_angle_flag.update(&rot.as_axis_angle());

                    let objects: Vec<&dyn Object> = match &mut mesh_objects {
                        Some((model_unrotated, model_rotated)) => {
                            model_rotated.geometry.set_transformation(rot_mat);
                            vec![
                                &*model_unrotated,
                                &*model_rotated,
                                &axes,
                                &axes_body,
                                axis_angle_flag.pole(),
                                axis_angle_flag.flag(),
                            ]
                        }
                        None => vec![
                            &axes,
                            &axes_body,
                            axis_angle_flag.pole(),
                            axis_angle_flag.flag(),
                        ],
                    };
                    frame_input
                        .screen()
                        .clear(ClearState::color_and_depth(0.051, 0.051, 0.094, 1.0, 1.0))
                        .render(&camera, objects, &[&light0, &light1]);

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
    let rotation_for_renderer = Rc::new(RefCell::new(Rotation::default()));
    let leptos_root = leptos::tachys::dom::document()
        .get_element_by_id("leptos-app")
        .expect("should find #leptos-app element")
        .unchecked_into::<leptos::web_sys::HtmlElement>();

    #[cfg(target_arch = "wasm32")]
    {
        use winit::event_loop::EventLoop;

        let event_loop = EventLoop::new();
        let redraw_proxy = event_loop.create_proxy();
        let request_redraw: RequestRedraw = Rc::new(move || { let _ = redraw_proxy.send_event(()); });
        let rotation_for_app = rotation_for_renderer.clone();
        let request_redraw_for_app = request_redraw.clone();

        mount_to(leptos_root, move || {
            view! { <App rotation_for_renderer=rotation_for_app.clone() request_redraw=request_redraw_for_app.clone() /> }
        })
        .forget();

        run_three_d(rotation_for_renderer, request_redraw, event_loop);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let rotation_for_app = rotation_for_renderer.clone();
        mount_to(leptos_root, move || {
            view! { <App rotation_for_renderer=rotation_for_app.clone() /> }
        })
        .forget();

        run_three_d(rotation_for_renderer);
    }
}
