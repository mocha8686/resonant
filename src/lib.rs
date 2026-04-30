#![feature(file_buffered, iterator_try_collect)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "prototyping"
)]

mod app;
pub(crate) mod audio_cache;
pub(crate) mod components;
pub mod scene;
pub mod soundscape;
pub mod track;
mod vector;

use std::sync::LazyLock;

pub use app::App;
use directories::ProjectDirs;
pub use vector::Vector2;

pub(crate) static PROJECT_DIRS: LazyLock<ProjectDirs> = LazyLock::new(|| {
    ProjectDirs::from("com.github", "mocha8686", env!("CARGO_PKG_NAME"))
        .expect("current user should have a home directory")
});
