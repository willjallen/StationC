//! Command-line entry point for `StationC`.

use std::process::ExitCode;

fn main() -> ExitCode {
    stationc::sim::run()
}
