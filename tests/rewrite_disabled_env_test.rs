//! End-to-end check of `RTK_DISABLED=1` exported in the environment for
//! `rtk rewrite` (#1153): the exported form is a passthrough — exit 1 and
//! nothing on stdout — decided before any registry work, and only the exact
//! value `1` counts.

use std::process::{Command, Output, Stdio};

fn rtk_rewrite(cmd: &str, rtk_disabled: Option<&str>) -> std::io::Result<Output> {
    let mut rtk = Command::new(env!("CARGO_BIN_EXE_rtk"));
    rtk.args(["rewrite", cmd])
        // A developer's own `export RTK_DISABLED=1` must not reach the child.
        .env_remove("RTK_DISABLED")
        .env("RTK_TELEMETRY_DISABLED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(value) = rtk_disabled {
        rtk.env("RTK_DISABLED", value);
    }
    rtk.output()
}

#[test]
fn rtk_disabled_in_env_is_a_passthrough() -> std::io::Result<()> {
    let out = rtk_rewrite("git status", Some("1"))?;

    assert_eq!(out.status.code(), Some(1), "passthrough exit code");
    assert!(
        out.stdout.is_empty(),
        "no rewrite may be printed, got: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    Ok(())
}

/// Only the exact value `1` disables the rewrite, so a parent that exported
/// `RTK_DISABLED=1` can hand a child `RTK_DISABLED=0` to re-enable it.
/// `rtk git status` is already RTK, so the rewrite is its own input whatever
/// permission rules or `exclude_commands` the host running the tests has.
#[test]
fn rtk_disabled_other_value_still_rewrites() -> std::io::Result<()> {
    let out = rtk_rewrite("rtk git status", Some("0"))?;

    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "rtk git status"
    );
    assert!(
        matches!(out.status.code(), Some(0) | Some(3)),
        "allow or ask exit code expected, got {:?}",
        out.status.code()
    );
    Ok(())
}
