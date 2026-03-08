//! Euler angles slider group with no-teleport logic.
//!
//! **No-teleport**: when the user drags a slider past π into the [-3π/2, 3π/2] range,
//! the slider handle stays at its position. The `dual_value` tick mark shows the canonical
//! position (from atan2/asin/acos, range [-π, π]).
//!
//! **Sequence/unit change**: snaps sliders to new canonical values immediately.

use std::cell::RefCell;
use std::rc::Rc;

use leptos::prelude::*;

use crate::app::rotation::{EulerAngles, EulerSequence, Rotation};
use crate::app::slider_widget::{CustomSlider, CustomSliderConfig};

const WRAP_THRESHOLD: f64 = 0.05;

/// Returns θ-prefixed axis labels [a, b, c] for the given sequence.
fn euler_axis_labels(seq: EulerSequence) -> [&'static str; 3] {
    match seq {
        EulerSequence::XYZ_zyx => ["θX", "θY", "θZ"],
        EulerSequence::XZY_yzx => ["θX", "θZ", "θY"],
        EulerSequence::YXZ_zxy => ["θY", "θX", "θZ"],
        EulerSequence::YZX_xzy => ["θY", "θZ", "θX"],
        EulerSequence::ZXY_yxz => ["θZ", "θX", "θY"],
        EulerSequence::ZYX_xyz => ["θZ", "θY", "θX"],
        EulerSequence::XYX_xyx => ["θX", "θY", "θX"],
        EulerSequence::XZX_xzx => ["θX", "θZ", "θX"],
        EulerSequence::YXY_yxy => ["θY", "θX", "θY"],
        EulerSequence::YZY_yzy => ["θY", "θZ", "θY"],
        EulerSequence::ZXZ_zxz => ["θZ", "θX", "θZ"],
        EulerSequence::ZYZ_zyz => ["θZ", "θY", "θZ"],
    }
}

fn is_wrapped_2pi(simplified_rad: f64, slider: f64, use_deg: bool) -> bool {
    let slider_rad = if use_deg { slider.to_radians() } else { slider };
    let diff = simplified_rad - slider_rad;
    (diff.abs() - 2.0 * std::f64::consts::PI).abs() < WRAP_THRESHOLD
}

#[component]
pub fn EulerAnglesSliderGroup(
    rotation: RwSignal<Rotation>,
    /// true = degrees [-270°, 270°], false = radians [-3π/2, 3π/2]
    use_degrees: RwSignal<bool>,
    sequence: RwSignal<EulerSequence>,
) -> impl IntoView {
    let angle_a = RwSignal::new(0.0_f64);
    let angle_b = RwSignal::new(0.0_f64);
    let angle_c = RwSignal::new(0.0_f64);

    let skip_sync = Rc::new(RefCell::new(false));

    let config_rad = CustomSliderConfig::euler_angle_rad();
    let config_deg = CustomSliderConfig::euler_angle_deg();

    // Canonical values from current rotation + sequence (always in radians).
    let simplified = Memo::new(move |_| {
        let seq = sequence.get();
        let ea = rotation.get().as_euler_angles(seq);
        (ea.a as f64, ea.b as f64, ea.c as f64)
    });

    let simplified_a_rad = Memo::new(move |_| simplified.get().0);
    let simplified_b_rad = Memo::new(move |_| simplified.get().1);
    let simplified_c_rad = Memo::new(move |_| simplified.get().2);
    let simplified_a_deg = Memo::new(move |_| simplified_a_rad.get().to_degrees());
    let simplified_b_deg = Memo::new(move |_| simplified_b_rad.get().to_degrees());
    let simplified_c_deg = Memo::new(move |_| simplified_c_rad.get().to_degrees());

    // Previous values for change detection.
    let prev_sequence = Rc::new(RefCell::new(sequence.get_untracked()));
    let prev_use_degrees = Rc::new(RefCell::new(use_degrees.get_untracked()));

    // Sync: rotation + sequence + use_degrees → sliders (with no-teleport for rotation-only changes).
    Effect::new({
        let skip_sync = skip_sync.clone();
        let prev_sequence = prev_sequence.clone();
        let prev_use_degrees = prev_use_degrees.clone();
        move || {
            let deg = use_degrees.get();
            let seq = sequence.get();
            let (sa_rad, sb_rad, sc_rad) = simplified.get();

            if *skip_sync.borrow() {
                *skip_sync.borrow_mut() = false;
                return;
            }

            let seq_changed = seq != *prev_sequence.borrow();
            let deg_changed = deg != *prev_use_degrees.borrow();
            *prev_sequence.borrow_mut() = seq;
            *prev_use_degrees.borrow_mut() = deg;

            let lo_deg = -270.0_f64;
            let hi_deg = 270.0_f64;
            let lo_rad = -3.0 * std::f64::consts::FRAC_PI_2;
            let hi_rad = 3.0 * std::f64::consts::FRAC_PI_2;

            if seq_changed || deg_changed {
                // Snap sliders to new canonical values.
                let (na, nb, nc) = if deg {
                    (
                        sa_rad.to_degrees().clamp(lo_deg, hi_deg),
                        sb_rad.to_degrees().clamp(lo_deg, hi_deg),
                        sc_rad.to_degrees().clamp(lo_deg, hi_deg),
                    )
                } else {
                    (
                        sa_rad.clamp(lo_rad, hi_rad),
                        sb_rad.clamp(lo_rad, hi_rad),
                        sc_rad.clamp(lo_rad, hi_rad),
                    )
                };
                batch(|| {
                    angle_a.set(na);
                    angle_b.set(nb);
                    angle_c.set(nc);
                });
            } else {
                // No-teleport: only update slider if not wrapped by 2π.
                let a_wrapped = is_wrapped_2pi(sa_rad, angle_a.get_untracked(), deg);
                let b_wrapped = is_wrapped_2pi(sb_rad, angle_b.get_untracked(), deg);
                let c_wrapped = is_wrapped_2pi(sc_rad, angle_c.get_untracked(), deg);

                batch(|| {
                    if !a_wrapped {
                        let new_a = if deg {
                            sa_rad.to_degrees().clamp(lo_deg, hi_deg)
                        } else {
                            sa_rad.clamp(lo_rad, hi_rad)
                        };
                        angle_a.set(new_a);
                    }
                    if !b_wrapped {
                        let new_b = if deg {
                            sb_rad.to_degrees().clamp(lo_deg, hi_deg)
                        } else {
                            sb_rad.clamp(lo_rad, hi_rad)
                        };
                        angle_b.set(new_b);
                    }
                    if !c_wrapped {
                        let new_c = if deg {
                            sc_rad.to_degrees().clamp(lo_deg, hi_deg)
                        } else {
                            sc_rad.clamp(lo_rad, hi_rad)
                        };
                        angle_c.set(new_c);
                    }
                });
            }
        }
    });

    let on_change = Rc::new({
        let skip_sync = skip_sync.clone();
        move |_value: f64| {
            *skip_sync.borrow_mut() = true;
            let (a, b, c) = (
                angle_a.get_untracked() as f32,
                angle_b.get_untracked() as f32,
                angle_c.get_untracked() as f32,
            );
            let seq = sequence.get_untracked();
            let ea = if use_degrees.get_untracked() {
                EulerAngles::from_degrees(a, b, c, seq)
            } else {
                EulerAngles::new(a, b, c, seq)
            };
            rotation.set(Rotation::from(ea));
        }
    });

    view! {
        <div style:display=move || if use_degrees.get() { "none" } else { "block" }>
            <div class="vector-sliders" style="display: flex; flex-direction: column;">
                <div style="order: 0;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[0]
                        config=config_rad.clone()
                        value=angle_a
                        dual_value=simplified_a_rad
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 1;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[1]
                        config=config_rad.clone()
                        value=angle_b
                        dual_value=simplified_b_rad
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 2;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[2]
                        config=config_rad.clone()
                        value=angle_c
                        dual_value=simplified_c_rad
                        on_value_change=on_change.clone()
                    />
                </div>
            </div>
        </div>
        <div style:display=move || if use_degrees.get() { "block" } else { "none" }>
            <div class="vector-sliders" style="display: flex; flex-direction: column;">
                <div style="order: 0;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[0]
                        config=config_deg.clone()
                        value=angle_a
                        dual_value=simplified_a_deg
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 1;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[1]
                        config=config_deg.clone()
                        value=angle_b
                        dual_value=simplified_b_deg
                        on_value_change=on_change.clone()
                    />
                </div>
                <div style="order: 2;">
                    <CustomSlider
                        label=move || euler_axis_labels(sequence.get())[2]
                        config=config_deg.clone()
                        value=angle_c
                        dual_value=simplified_c_deg
                        on_value_change=on_change.clone()
                    />
                </div>
            </div>
        </div>
    }
}
