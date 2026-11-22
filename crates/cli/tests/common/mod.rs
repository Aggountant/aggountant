//! Helpers shared by the CLI integration tests.

use std::io::{ErrorKind, Write};
use std::process::Child;

/// Writes `input` to the child's piped stdin and closes it.
///
/// A command that refuses before reading stdin (input given in flags, a
/// validation error, a conflicting positional argument) may exit before the
/// write; the write then fails with `BrokenPipe`. That is not a test failure:
/// the caller asserts on the child's exit code and output. Any other write
/// error still panics.
#[allow(dead_code)]
pub fn feed_stdin(child: &mut Child, input: &[u8]) {
    let mut stdin = child.stdin.take().expect("child stdin is piped");
    if let Err(error) = stdin.write_all(input) {
        assert_eq!(
            error.kind(),
            ErrorKind::BrokenPipe,
            "writing child stdin: {error}"
        );
    }
}

/// A temporary directory holding a fresh company home set up with `init`.
/// Every command except `init` refuses a directory that is not a home.
#[allow(dead_code)]
pub fn initialized_home() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temporary home");
    init_home(dir.path());
    dir
}

/// Set up a company home at `home` with `init`.
#[allow(dead_code)]
pub fn init_home(home: &std::path::Path) {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_aggountant"))
        .arg("--home")
        .arg(home)
        .args(["--json", "init"])
        .output()
        .expect("spawn aggountant init");
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
