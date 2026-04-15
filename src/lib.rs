#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "prototyping"
)]

pub mod soundscape;
pub mod track;
pub(crate) mod components;
mod vector;

pub use vector::Vector2;
