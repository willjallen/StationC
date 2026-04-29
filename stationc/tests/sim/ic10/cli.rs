use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

#[test]
fn cli_runs_ic10_file_successfully() -> TestResult {
    let source_path = write_temp_source(
        "\
move r0 1
move r1 2
add r2 r0 r1
yield
",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_stationc"))
        .arg("sim")
        .arg("ic10")
        .arg(&source_path)
        .arg("--trace")
        .output();
    remove_temp_source(&source_path)?;

    let output = output?;
    if !output.status.success() {
        return Err(test_error(format!(
            "expected CLI success, stderr was `{}`",
            String::from_utf8(output.stderr)?
        )));
    }

    Ok(())
}

#[test]
fn cli_reports_missing_source_path() -> TestResult {
    let output = Command::new(env!("CARGO_BIN_EXE_stationc"))
        .arg("sim")
        .arg("ic10")
        .output()?;

    if output.status.success() {
        Err(test_error("expected CLI failure"))
    } else {
        Ok(())
    }
}

fn write_temp_source(source: &str) -> TestResult<PathBuf> {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let source_path = std::env::temp_dir().join(format!(
        "stationc-ic10-cli-test-{}-{sequence}.ic10",
        std::process::id()
    ));
    fs::write(&source_path, source)?;
    Ok(source_path)
}

fn remove_temp_source(source_path: &Path) -> TestResult {
    fs::remove_file(source_path).map_err(|error| {
        test_error(format!(
            "failed to remove {}: {error}",
            source_path.display()
        ))
    })
}

fn test_error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::other(message.into()))
}
