//! Rotation vector representation (x,y,z with norm = angle).
//!
//! **State flow**: `rotation` (source of truth) → Effect → text/sliders when not editing.
//! Text input and sliders → parse/update → `rotation`.

mod rotation_vector_box;
mod slider_group;

pub use rotation_vector_box::RotationVectorBox;
