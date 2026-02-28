//! Axis-angle representation: unit axis (x,y,z) and angle θ.
//!
//! **State flow**: `rotation` (source of truth) → Effect → text/sliders when not editing.
//! Text input and sliders → parse/update → `rotation`.

mod axis_angle_box;
mod slider_group;

pub use axis_angle_box::AxisAngleBox;
