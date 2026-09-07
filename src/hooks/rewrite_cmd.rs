//! Translates a raw shell command into its RTK-optimized equivalent.

use super::permissions::{check_command, PermissionVerdict};
use crate::discover::registry;
use std::io::Write;

/// Run the `rtk rewrite` command.
///
/// Prints the RTK-rewritten command to stdout and exits with a code that tells
/// the caller how to handle permissions:
///
/// | Exit | Stdout   | Meaning                                                      |
/// |------|----------|--------------------------------------------------------------|
/// | 0    | rewritten| Rewrite allowed — hook may auto-allow the rewritten command. |
/// | 1    | (none)   | No RTK equivalent, or `RTK_DISABLED=1` exported — hook passes through unchanged. |
/// | 2    | (none)   | Deny rule matched — hook defers to Claude Code native deny.  |
/// | 3    | rewritten| Ask rule matched — hook rewrites but lets Claude Code prompt.|
pub fn run(cmd: &str) -> anyhow::Result<()> {
    let disabled_by_env = rewrite_disabled_by_env();
    let (excluded, transparent_prefixes) = crate::core::config::hook_rewrite_params();

    match evaluate(cmd, disabled_by_env, &excluded, &transparent_prefixes) {
        RewriteOutcome::Allow(rewritten) => {
            print!("{}", rewritten);
            let _ = std::io::stdout().flush();
            Ok(())
        }
        RewriteOutcome::Ask(rewritten) => {
            print!("{}", rewritten);
            let _ = std::io::stdout().flush();
            std::process::exit(3);
        }
        RewriteOutcome::Deny => std::process::exit(2),
        RewriteOutcome::Passthrough => std::process::exit(1),
    }
}

#[derive(Debug, PartialEq)]
enum RewriteOutcome {
    Allow(String),
    Passthrough,
    Deny,
    Ask(String),
}

/// `RTK_DISABLED=1` exported in the process environment (#1153, #3791).
///
/// The `RTK_DISABLED=1 <cmd>` prefix, handled per segment by the registry
/// (#345), opts one command out; the exported form opts out every command the
/// process sees, so a parent that sets it once stands the hook down for its
/// whole process tree. Only the exact value `1` counts, like the other `RTK_*`
/// switches, so a child can be re-enabled with `RTK_DISABLED=0`.
fn rewrite_disabled_by_env() -> bool {
    std::env::var("RTK_DISABLED").as_deref() == Ok("1")
}

/// Decide the outcome for `cmd`.
///
/// `disabled_by_env` is [`rewrite_disabled_by_env`], read once by the caller
/// and passed in for the same reason the verdict is a parameter of
/// [`evaluate_with_verdict`]: tests stay off the process-global environment.
/// When set, the answer is a passthrough before the permission settings, the
/// lexer, or the registry are consulted, so a disabled host never has its
/// settings files read or a rewrite computed.
fn evaluate(
    cmd: &str,
    disabled_by_env: bool,
    excluded: &[String],
    transparent_prefixes: &[String],
) -> RewriteOutcome {
    if disabled_by_env {
        return RewriteOutcome::Passthrough;
    }
    evaluate_with_verdict(cmd, check_command(cmd), excluded, transparent_prefixes)
}

/// Decision logic for [`evaluate`] with the permission verdict supplied by the
/// caller, mirroring [`check_command_with_rules`](super::permissions::check_command_with_rules).
///
/// `check_command` reads the machine's Claude Code settings files, so tests that
/// call [`evaluate`] directly would change verdict with the developer's local
/// `settings.local.json`. Taking the verdict as a parameter keeps the rewrite
/// logic under test independent of the host configuration (#3146).
fn evaluate_with_verdict(
    cmd: &str,
    verdict: PermissionVerdict,
    excluded: &[String],
    transparent_prefixes: &[String],
) -> RewriteOutcome {
    if verdict == PermissionVerdict::Deny {
        return RewriteOutcome::Deny;
    }

    if crate::discover::lexer::contains_unattestable_construct(cmd) {
        return RewriteOutcome::Passthrough;
    }

    match registry::rewrite_command(cmd, excluded, transparent_prefixes) {
        Some(rewritten) => match verdict {
            PermissionVerdict::Allow => RewriteOutcome::Allow(rewritten),
            _ => RewriteOutcome::Ask(rewritten),
        },
        None => RewriteOutcome::Passthrough,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite_command_no_prefixes(cmd: &str) -> Option<String> {
        registry::rewrite_command(cmd, &[], &[])
    }

    #[test]
    fn test_run_supported_command_succeeds() {
        assert!(rewrite_command_no_prefixes("git status").is_some());
    }

    #[test]
    fn test_run_unsupported_returns_none() {
        assert!(rewrite_command_no_prefixes("htop").is_none());
    }

    #[test]
    fn test_run_already_rtk_returns_some() {
        assert_eq!(
            rewrite_command_no_prefixes("rtk git status"),
            Some("rtk git status".into())
        );
    }

    /// The verdict still drives the outcome: an allow rule yields `Allow`.
    /// Pinning both directions keeps the mapping covered without depending
    /// on which rules the developer happens to have configured.
    #[test]
    fn test_allow_verdict_yields_allow() {
        assert!(matches!(
            evaluate_with_verdict("git status", PermissionVerdict::Allow, &[], &[]),
            RewriteOutcome::Allow(_)
        ));
    }

    #[test]
    fn test_deny_verdict_yields_deny() {
        assert_eq!(
            evaluate_with_verdict("git status", PermissionVerdict::Deny, &[], &[]),
            RewriteOutcome::Deny
        );
    }

    /// Commands with an unattestable construct are always a passthrough,
    /// regardless of permission verdict.
    ///
    /// The verdict is pinned to `Default` rather than going through `evaluate`,
    /// which reads the developer's own `.claude/settings.local.json`: a
    /// `Bash(git *)` allow rule turns the expected `Ask` into `Allow` and these
    /// tests fail on that machine only (#3146). Pinning the verdict keeps the
    /// assertions about the rewrite logic and nothing about the host.
    mod unattestable_passthrough {
        use super::super::{evaluate_with_verdict, RewriteOutcome};
        use crate::hooks::permissions::PermissionVerdict;

        #[test]
        fn test_backtick_substitution_passthrough() {
            assert_eq!(
                evaluate_with_verdict(
                    "git status `rm -rf /tmp/x`",
                    PermissionVerdict::Default,
                    &[],
                    &[]
                ),
                RewriteOutcome::Passthrough
            );
        }

        #[test]
        fn test_dollar_substitution_passthrough() {
            assert_eq!(
                evaluate_with_verdict(
                    "git status $(rm -rf /tmp/x)",
                    PermissionVerdict::Default,
                    &[],
                    &[]
                ),
                RewriteOutcome::Passthrough
            );
        }

        #[test]
        fn test_double_quoted_substitution_passthrough() {
            assert_eq!(
                evaluate_with_verdict(
                    "git log --pretty=\"$(rm -rf /tmp/x)\"",
                    PermissionVerdict::Default,
                    &[],
                    &[]
                ),
                RewriteOutcome::Passthrough
            );
        }

        #[test]
        fn test_file_redirect_passthrough() {
            assert_eq!(
                evaluate_with_verdict(
                    "git log > /tmp/out.txt",
                    PermissionVerdict::Default,
                    &[],
                    &[]
                ),
                RewriteOutcome::Passthrough
            );
        }

        #[test]
        fn test_fd_dup_redirect_still_rewrites() {
            assert!(matches!(
                evaluate_with_verdict("git status 2>&1", PermissionVerdict::Default, &[], &[]),
                RewriteOutcome::Ask(_)
            ));
        }

        #[test]
        fn test_plain_command_still_rewrites() {
            assert!(matches!(
                evaluate_with_verdict("git status", PermissionVerdict::Default, &[], &[]),
                RewriteOutcome::Ask(_)
            ));
        }
    }

    /// `RTK_DISABLED=1` exported in the environment is a passthrough for every
    /// command (#1153), decided before the permission settings are read:
    /// `evaluate` never reaches `check_command` when the flag is set, so the
    /// disabled cases are independent of the developer's own Claude Code
    /// settings.
    mod disabled_by_env {
        use super::super::{evaluate, evaluate_with_verdict, RewriteOutcome};
        use crate::hooks::permissions::PermissionVerdict;

        #[test]
        fn test_disabled_by_env_passes_through_rewritable_command() {
            assert_eq!(
                evaluate("git status", true, &[], &[]),
                RewriteOutcome::Passthrough
            );
        }

        #[test]
        fn test_disabled_by_env_passes_through_already_rtk_command() {
            assert_eq!(
                evaluate("rtk git status", true, &[], &[]),
                RewriteOutcome::Passthrough
            );
        }

        /// With the flag off, `git status` still rewrites. Which of
        /// `Allow`/`Ask`/`Deny` comes back depends on the developer's own
        /// Claude Code settings (#3146); only `Passthrough` would mean the
        /// flag leaked into the enabled path.
        #[test]
        fn test_enabled_env_still_rewrites() {
            assert_ne!(
                evaluate("git status", false, &[], &[]),
                RewriteOutcome::Passthrough
            );
        }

        /// The in-command prefix (#345) is unchanged: with the flag off it is
        /// still the registry's decision.
        #[test]
        fn test_prefix_form_still_passes_through_without_env() {
            assert_eq!(
                evaluate_with_verdict(
                    "RTK_DISABLED=1 git status",
                    PermissionVerdict::Default,
                    &[],
                    &[]
                ),
                RewriteOutcome::Passthrough
            );
        }
    }

    /// SECURITY: Verify the exit code protocol for permission verdicts.
    ///
    /// The bash hook (.claude/hooks/rtk-rewrite.sh) interprets exit codes as:
    ///   0 → auto-allow (sets permissionDecision: "allow")
    ///   1 → passthrough (no RTK equivalent)
    ///   2 → deny (let Claude Code handle natively)
    ///   3 → ask (rewrite but omit permissionDecision, forcing user prompt)
    ///
    /// CRITICAL: PermissionVerdict::Default MUST map to exit 3 (ask), NOT exit 0.
    /// If Default were mapped to exit 0, any command without an explicit permission
    /// rule would be auto-allowed — bypassing Claude Code's least-privilege default.
    /// See: https://github.com/rtk-ai/rtk/issues/1155
    mod exit_code_protocol {
        use super::registry;
        use crate::hooks::permissions::{check_command_with_rules, PermissionVerdict};

        /// Exit code that `run()` returns for each verdict:
        ///   Allow  → 0 (exit Ok(()))
        ///   Ask    → 3 (process::exit(3))
        ///   Default→ 3 (process::exit(3)) — grouped with Ask
        ///   Deny   → 2 (process::exit(2)) — handled before rewrite match
        fn expected_exit_code(verdict: &PermissionVerdict) -> i32 {
            match verdict {
                PermissionVerdict::Allow => 0,
                PermissionVerdict::Deny => 2,
                PermissionVerdict::Ask => 3,
                PermissionVerdict::Default => 3, // MUST be 3, not 0!
            }
        }

        #[test]
        fn test_default_verdict_maps_to_ask_exit_code() {
            // When no rules match, verdict is Default → exit code must be 3 (ask).
            let verdict = check_command_with_rules("git status", &[], &[], &[]);
            assert_eq!(verdict, PermissionVerdict::Default);
            assert_eq!(
                expected_exit_code(&verdict),
                3,
                "Default verdict MUST exit with code 3 (ask), not 0 (allow)"
            );
        }

        #[test]
        fn test_allow_verdict_maps_to_allow_exit_code() {
            let allow = vec!["git *".to_string()];
            let verdict = check_command_with_rules("git status", &[], &[], &allow);
            assert_eq!(verdict, PermissionVerdict::Allow);
            assert_eq!(expected_exit_code(&verdict), 0);
        }

        #[test]
        fn test_ask_verdict_maps_to_ask_exit_code() {
            let ask = vec!["git push".to_string()];
            let verdict = check_command_with_rules("git push origin main", &[], &ask, &[]);
            assert_eq!(verdict, PermissionVerdict::Ask);
            assert_eq!(expected_exit_code(&verdict), 3);
        }

        #[test]
        fn test_deny_verdict_maps_to_deny_exit_code() {
            let deny = vec!["rm -rf".to_string()];
            let verdict = check_command_with_rules("rm -rf /tmp/test", &deny, &[], &[]);
            assert_eq!(verdict, PermissionVerdict::Deny);
            assert_eq!(expected_exit_code(&verdict), 2);
        }

        #[test]
        fn test_no_auto_allow_bypass_for_unrecognized_commands() {
            // SECURITY: A command with no permission rules and no matching allow rule
            // must NOT be auto-allowed. This is the core of issue #1155.
            // Even though `git status` can be rewritten to `rtk git status`,
            // the absence of an allow rule means Default → exit 3 → ask.
            let verdict = check_command_with_rules("git status", &[], &[], &[]);
            assert_eq!(verdict, PermissionVerdict::Default);

            // Verify the rewrite exists (so the hook would output it),
            // but the exit code forces user confirmation.
            assert!(registry::rewrite_command("git status", &[], &[]).is_some());
            assert_eq!(expected_exit_code(&verdict), 3);
        }

        #[test]
        fn test_default_never_equals_allow() {
            // Sentinel: ensure Default and Allow are distinct enum variants.
            // If this ever fails, the entire permission model is broken.
            assert_ne!(PermissionVerdict::Default, PermissionVerdict::Allow);
        }
    }
}
