use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::process::Command;

/// Compact grep - strips whitespace, truncates, groups by file
pub fn run(
    pattern: &str,
    path: &str,
    max_line_len: usize,
    max_results: usize,
    context_only: bool,
    verbose: u8,
) -> Result<()> {
    if verbose > 0 {
        eprintln!("Searching: '{}' in {}", pattern, path);
    }

    // Use ripgrep if available, otherwise grep
    let output = Command::new("rg")
        .args(["-n", "--no-heading", pattern, path])
        .output()
        .or_else(|_| {
            Command::new("grep")
                .args(["-rn", pattern, path])
                .output()
        })
        .context("Failed to run grep/rg")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() {
        println!("No matches for '{}'", pattern);
        return Ok(());
    }

    // Parse and group by file
    let mut by_file: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    let mut total_matches = 0;

    for line in stdout.lines() {
        // Parse format: file:line:content or line:content
        let parts: Vec<&str> = line.splitn(3, ':').collect();

        let (file, line_num, content) = if parts.len() == 3 {
            // file:line:content
            let ln = parts[1].parse().unwrap_or(0);
            (parts[0].to_string(), ln, parts[2])
        } else if parts.len() == 2 {
            // line:content (single file mode)
            let ln = parts[0].parse().unwrap_or(0);
            (path.to_string(), ln, parts[1])
        } else {
            continue;
        };

        total_matches += 1;

        // Clean and truncate content
        let cleaned = clean_line(content, max_line_len, context_only, pattern);

        by_file
            .entry(file)
            .or_default()
            .push((line_num, cleaned));
    }

    // Print results
    println!("🔍 {} matches in {} files:", total_matches, by_file.len());
    println!();

    let mut shown = 0;
    let mut files: Vec<_> = by_file.iter().collect();
    files.sort_by_key(|(f, _)| *f);

    for (file, matches) in files {
        if shown >= max_results {
            break;
        }

        // Compact file path
        let file_display = compact_path(file);
        println!("📄 {} ({}):", file_display, matches.len());

        for (line_num, content) in matches.iter().take(10) {
            println!("  {:>4}: {}", line_num, content);
            shown += 1;
            if shown >= max_results {
                break;
            }
        }

        if matches.len() > 10 {
            println!("  ... +{} more in this file", matches.len() - 10);
        }
        println!();
    }

    if total_matches > shown {
        println!("... +{} more matches (use -m to show more)", total_matches - shown);
    }

    Ok(())
}

fn clean_line(line: &str, max_len: usize, context_only: bool, pattern: &str) -> String {
    // Strip leading/trailing whitespace
    let trimmed = line.trim();

    if context_only {
        // Try to extract just the match with surrounding context
        if let Ok(re) = Regex::new(&format!("(?i).{{0,20}}{}.*", regex::escape(pattern))) {
            if let Some(m) = re.find(trimmed) {
                let matched = m.as_str();
                if matched.len() <= max_len {
                    return matched.to_string();
                }
            }
        }
    }

    // Truncate if needed
    if trimmed.len() <= max_len {
        trimmed.to_string()
    } else {
        // Try to keep the match visible
        let lower = trimmed.to_lowercase();
        let pattern_lower = pattern.to_lowercase();

        if let Some(pos) = lower.find(&pattern_lower) {
            // Center around the match
            let start = pos.saturating_sub(max_len / 3);
            let end = (start + max_len).min(trimmed.len());
            let start = if end == trimmed.len() {
                end.saturating_sub(max_len)
            } else {
                start
            };

            let slice = &trimmed[start..end];
            if start > 0 && end < trimmed.len() {
                format!("...{}...", slice)
            } else if start > 0 {
                format!("...{}", slice)
            } else {
                format!("{}...", slice)
            }
        } else {
            format!("{}...", &trimmed[..max_len - 3])
        }
    }
}

fn compact_path(path: &str) -> String {
    // Shorten long paths
    if path.len() <= 50 {
        return path.to_string();
    }

    // Try to show meaningful parts
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() <= 3 {
        return path.to_string();
    }

    // Keep first, ..., and last 2 parts
    format!(
        "{}/.../{}/{}",
        parts[0],
        parts[parts.len() - 2],
        parts[parts.len() - 1]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_line() {
        let line = "            const result = someFunction();";
        let cleaned = clean_line(line, 50, false, "result");
        assert!(!cleaned.starts_with(' '));
        assert!(cleaned.len() <= 50);
    }

    #[test]
    fn test_compact_path() {
        let path = "/Users/patrick/dev/project/src/components/Button.tsx";
        let compact = compact_path(path);
        assert!(compact.len() <= 60);
    }
}
