//! Quaternion slider group with LRU-based normalization.

mod normalize;

pub use normalize::{normalize_lru, touch_order, X, Y, Z, W};
