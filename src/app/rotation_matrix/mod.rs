//! Rotation matrix representation (3×3).
//!
//! **State flow**: `rotation` (source of truth) → Effect → text when not editing.
//! Textarea input → parse → `rotation`.

mod rotation_matrix_box;

pub use rotation_matrix_box::RotationMatrixBox;
