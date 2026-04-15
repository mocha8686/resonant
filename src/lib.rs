#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "prototyping"
)]

pub(crate) mod components;
pub mod soundscape;
pub mod track;
mod id;
mod vector;

pub use id::Id;
pub use vector::Vector2;
