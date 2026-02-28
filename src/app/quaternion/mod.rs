//! Quaternion representation (w,x,y,z).
//!
//! **State flow**: `rotation` (source of truth) → Effect → text/sliders when not editing.
//! Text input and sliders → parse/update → `rotation`.

mod quaternion_box;
mod slider_group;

pub use quaternion_box::QuaternionBox;
