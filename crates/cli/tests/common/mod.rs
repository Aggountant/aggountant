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
