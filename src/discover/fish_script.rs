//! Classifies unambiguously-fish command strings and wraps them for explicit fish execution.
//!
//! Hosts that evaluate Bash-tool command strings with a POSIX/zsh layer choke on
//! fish-only syntax (`if … end`, `; and`) before RTK ever runs. The hook decision
//! paths call [`try_wrap`] at the unattestable gate: a script that is provably fish
//! (fish-only marker at command position, no POSIX disambiguator) is rewritten to
//! `rtk run --shell fish -c '<script>'` — a form every host layer parses as one
//! command with one quoted argument. Anything ambiguous keeps the defer behavior.

use super::lexer::{self, ParsedToken, TokenKind};
use super::shell_wrapper::is_shell_wrapper_candidate;

/// Keywords only fish accepts at command position. Every fish block terminator or
/// continuation word here is also in the lexer's unattestable control keywords, so
/// classified scripts always reach the hook's unattestable gate.
const FISH_ONLY_KEYWORDS: &[&str] = &["end", "begin", "switch", "and", "or", "not"];

/// Keywords only POSIX shells accept at command position — their presence vetoes
/// fish classification.
const POSIX_ONLY_KEYWORDS: &[&str] = &["then", "fi", "do", "done", "esac", "elif"];

/// Wrap `cmd` for explicit fish execution when it is unambiguously a fish script.
///
/// Returns `None` (caller keeps its defer behavior) on Windows, when the command
/// already delegates (`rtk …` or an explicit shell `-c` wrapper), when the script
/// is not provably fish, or when no `fish` binary is resolvable.
#[allow(dead_code)] // wired into the hook decision paths in a follow-up commit
pub fn try_wrap(cmd: &str) -> Option<String> {
    try_wrap_gated(cmd, crate::core::utils::resolve_binary("fish").is_ok())
}

/// [`try_wrap`] with the fish-binary probe injected, for deterministic tests.
pub(crate) fn try_wrap_gated(cmd: &str, fish_available: bool) -> Option<String> {
    if cfg!(windows) {
        // cmd/PowerShell host layers do not honor POSIX single quotes, so the
        // wrapped form could be mis-tokenized before reaching rtk.
        return None;
    }

    let script = cmd.trim();
    if script.split_whitespace().next() == Some("rtk") || is_shell_wrapper_candidate(script) {
        return None;
    }
    if !is_unambiguous_fish(script) || !fish_available {
        return None;
    }

    Some(format!(
        "rtk run --shell fish -c '{}'",
        escape_single_quoted(script)
    ))
}

/// True only when the command contains a fish-only marker at command position and
/// nothing a POSIX shell would need to parse it (see module docs for the lists).
pub(crate) fn is_unambiguous_fish(cmd: &str) -> bool {
    if cmd.contains('\0') || lexer::has_unclosed_quote_or_escape(cmd) {
        return false;
    }
    classify_tokens(&lexer::tokenize_with_newlines(cmd))
}

fn classify_tokens(tokens: &[ParsedToken]) -> bool {
    let mut command_position = true;
    let mut has_fish_marker = false;

    for token in tokens {
        match token.kind {
            TokenKind::Operator | TokenKind::Pipe(_) => command_position = true,
            TokenKind::Shellism if token.value == "&" => command_position = true,
            // Heredocs (`<<`, `<<-`, `<<<` all tokenize with a `<<` prefix) do not
            // exist in fish — the script must be POSIX or malformed.
            TokenKind::Redirect if token.value.starts_with("<<") => return false,
            TokenKind::Arg => {
                // `[[ … ]]` is bash/zsh-only wherever it appears.
                if token.value == "[[" {
                    return false;
                }
                if command_position {
                    if POSIX_ONLY_KEYWORDS.contains(&token.value.as_str()) {
                        return false;
                    }
                    if FISH_ONLY_KEYWORDS.contains(&token.value.as_str()) {
                        has_fish_marker = true;
                    }
                    command_position = false;
                }
            }
            _ => {}
        }
    }

    has_fish_marker
}

/// Escape for single-quoted embedding: `'` → `'\''`. The idiom evaluates back to
/// the original script under sh/bash/zsh (quote-close, escaped quote, quote-open)
/// and under fish (`\'` is a literal quote; adjacent strings concatenate).
/// Newlines pass through untouched inside the quotes.
fn escape_single_quoted(script: &str) -> String {
    script.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_unambiguous_fish: fish-only markers ------------------------------

    #[test]
    fn test_multiline_if_else_end_is_fish() {
        assert!(is_unambiguous_fish(
            "if test -d src\n  git status\nelse\n  echo missing\nend"
        ));
    }

    #[test]
    fn test_single_line_and_chain_is_fish() {
        assert!(is_unambiguous_fish("test -d src; and git status"));
        assert!(is_unambiguous_fish("test -d src; or echo missing"));
    }

    #[test]
    fn test_for_loop_with_fish_substitution_is_fish() {
        assert!(is_unambiguous_fish("for f in (ls)\n  echo $f\nend"));
    }

    #[test]
    fn test_begin_block_is_fish() {
        assert!(is_unambiguous_fish("begin; echo hi; end"));
    }

    #[test]
    fn test_switch_block_is_fish() {
        assert!(is_unambiguous_fish("switch $x\ncase a\necho a\nend"));
    }

    #[test]
    fn test_not_at_command_position_is_fish() {
        assert!(is_unambiguous_fish("not grep -q pattern file"));
    }

    #[test]
    fn test_function_block_detected_via_end() {
        assert!(is_unambiguous_fish(
            "function greet\n  echo hello $argv\nend"
        ));
    }

    // --- is_unambiguous_fish: POSIX vetoes -----------------------------------

    #[test]
    fn test_posix_if_then_fi_is_not_fish() {
        assert!(!is_unambiguous_fish("if [ -d src ]; then git status; fi"));
    }

    #[test]
    fn test_posix_for_do_done_is_not_fish() {
        assert!(!is_unambiguous_fish("for f in *.rs\ndo\n  echo $f\ndone"));
    }

    #[test]
    fn test_posix_case_esac_is_not_fish() {
        assert!(!is_unambiguous_fish(
            "case $x in\na) echo one ;;\nesac\nend"
        ));
    }

    #[test]
    fn test_heredoc_vetoes_fish() {
        assert!(!is_unambiguous_fish(
            "if test -d src\ncat <<EOF\nx\nEOF\nend"
        ));
    }

    #[test]
    fn test_double_bracket_vetoes_fish() {
        assert!(!is_unambiguous_fish("[[ -d src ]]\nend"));
    }

    // --- is_unambiguous_fish: ambiguous stays unclassified -------------------

    #[test]
    fn test_shared_keywords_without_marker_are_ambiguous() {
        assert!(!is_unambiguous_fish("if test -d src"));
        assert!(!is_unambiguous_fish("git status && cargo build"));
        assert!(!is_unambiguous_fish("git status"));
        assert!(!is_unambiguous_fish(""));
    }

    #[test]
    fn test_fish_keyword_as_argument_is_not_marker() {
        assert!(!is_unambiguous_fish("rg end src/"));
        assert!(!is_unambiguous_fish("printf '%s\\n' and"));
    }

    #[test]
    fn test_quoted_fish_syntax_is_not_marker() {
        assert!(!is_unambiguous_fish("echo 'if x; and y; end'"));
        assert!(!is_unambiguous_fish("echo \"begin; end\""));
    }

    #[test]
    fn test_incomplete_quoting_is_never_classified() {
        assert!(!is_unambiguous_fish("if test -d src\necho 'unclosed\nend"));
        assert!(!is_unambiguous_fish("test -d src; and git status \\"));
    }

    #[test]
    fn test_nul_byte_is_never_classified() {
        assert!(!is_unambiguous_fish("test -d src; and\0 git status; end"));
    }

    // --- escape_single_quoted -------------------------------------------------

    #[test]
    fn test_escape_plain_script_unchanged() {
        assert_eq!(escape_single_quoted("echo hi"), "echo hi");
    }

    #[test]
    fn test_escape_single_quotes() {
        assert_eq!(escape_single_quoted("echo 'a b'"), "echo '\\''a b'\\''");
    }

    #[test]
    fn test_escape_preserves_newlines_and_unicode() {
        assert_eq!(
            escape_single_quoted("echo 日本語\necho 'なか'"),
            "echo 日本語\necho '\\''なか'\\''"
        );
    }

    // --- try_wrap_gated --------------------------------------------------------

    #[cfg(not(windows))]
    #[test]
    fn test_wraps_multiline_fish_block() {
        assert_eq!(
            try_wrap_gated(
                "if test -d src\n  git status\nelse\n  echo missing\nend",
                true
            )
            .as_deref(),
            Some(
                "rtk run --shell fish -c 'if test -d src\n  git status\nelse\n  echo missing\nend'"
            )
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn test_wraps_script_containing_single_quotes() {
        assert_eq!(
            try_wrap_gated("echo 'a b'; and echo done", true).as_deref(),
            Some("rtk run --shell fish -c 'echo '\\''a b'\\''; and echo done'")
        );
    }

    #[test]
    fn test_posix_and_ambiguous_scripts_are_not_wrapped() {
        assert!(try_wrap_gated("if [ -d src ]; then git status; fi", true).is_none());
        assert!(try_wrap_gated("git status && cargo build", true).is_none());
    }

    #[test]
    fn test_missing_fish_binary_defers() {
        assert!(try_wrap_gated("test -d src; and git status", false).is_none());
    }

    #[test]
    fn test_rtk_prefixed_command_is_not_wrapped() {
        // `and` at command position would classify without the rtk gate.
        assert!(try_wrap_gated("rtk git status; and echo ok", true).is_none());
    }

    #[test]
    fn test_explicit_shell_wrapper_is_not_wrapped() {
        assert!(try_wrap_gated("fish -c 'if x; end'; and echo ok", true).is_none());
        assert!(try_wrap_gated("fish -c 'if x; end'", true).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn test_windows_never_wraps() {
        assert!(try_wrap_gated("test -d src; and git status", true).is_none());
    }
}
