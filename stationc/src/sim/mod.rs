//! Simulator entry points.

pub mod ic10;
pub mod world;

/// Runs the default simulator target.
#[must_use]
pub fn run() -> std::process::ExitCode {
    ic10::run()
}
