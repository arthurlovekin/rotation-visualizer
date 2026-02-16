//! Axis-angle representation: unit axis (x,y,z) and angle θ.
//!
//! Input box with VectorFormat, 4 sliders (x, y, z, θ), and degrees/radians dropdown.

mod axis_angle_box;
mod slider_group;

pub use axis_angle_box::AxisAngleBox;
