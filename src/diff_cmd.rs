use anyhow::Result;
use std::fs;
use std::path::Path;

/// Ultra-condensed diff - only changed lines, no context
pub fn run(file1: &Path, file2: &Path, verbose: u8) -> Result<()> {
    if verbose > 0 {
        eprintln!("Comparing: {} vs {}", file1.display(), file2.display());
    }

    let content1 = fs::read_to_string(file1)?;
    let content2 = fs::read_to_string(file2)?;

    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();

    let diff = compute_diff(&lines1, &lines2);

    if diff.added == 0 && diff.removed == 0 {
        println!("✅ Files are identical");
        return Ok(());
    }

    // Print summary
    println!("📊 {} → {}", file1.display(), file2.display());
    println!("   +{} added, -{} removed, ~{} modified", diff.added, diff.removed, diff.modified);
    println!();

    // Print changes (condensed)
    for change in diff.changes.iter().take(50) {
        match change {
            DiffChange::Added(line_num, content) => {
                println!("+{:4} {}", line_num, truncate(content, 80));
            }
            DiffChange::Removed(line_num, content) => {
                println!("-{:4} {}", line_num, truncate(content, 80));
            }
            DiffChange::Modified(line_num, old, new) => {
                println!("~{:4} {} → {}", line_num, truncate(old, 35), truncate(new, 35));
            }
        }
    }

    if diff.changes.len() > 50 {
        println!("... +{} more changes", diff.changes.len() - 50);
    }

    Ok(())
}

/// Run diff from stdin (piped command output)
pub fn run_stdin(_verbose: u8) -> Result<()> {
    use std::io::{self, Read};

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse unified diff format
    let condensed = condense_unified_diff(&input);
    println!("{}", condensed);

    Ok(())
}

#[derive(Debug)]
enum DiffChange {
    Added(usize, String),
    Removed(usize, String),
    Modified(usize, String, String),
}

struct DiffResult {
    added: usize,
    removed: usize,
    modified: usize,
    changes: Vec<DiffChange>,
}

fn compute_diff(lines1: &[&str], lines2: &[&str]) -> DiffResult {
    let mut changes = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;

    // Simple line-by-line comparison (not optimal but fast)
    let max_len = lines1.len().max(lines2.len());

    for i in 0..max_len {
        let l1 = lines1.get(i).copied();
        let l2 = lines2.get(i).copied();

        match (l1, l2) {
            (Some(a), Some(b)) if a != b => {
                // Check if it's similar (modification) or completely different
                if similarity(a, b) > 0.5 {
                    changes.push(DiffChange::Modified(i + 1, a.to_string(), b.to_string()));
                    modified += 1;
                } else {
                    changes.push(DiffChange::Removed(i + 1, a.to_string()));
                    changes.push(DiffChange::Added(i + 1, b.to_string()));
                    removed += 1;
                    added += 1;
                }
            }
            (Some(a), None) => {
                changes.push(DiffChange::Removed(i + 1, a.to_string()));
                removed += 1;
            }
            (None, Some(b)) => {
                changes.push(DiffChange::Added(i + 1, b.to_string()));
                added += 1;
            }
            _ => {}
        }
    }

    DiffResult { added, removed, modified, changes }
}

fn similarity(a: &str, b: &str) -> f64 {
    let a_chars: std::collections::HashSet<char> = a.chars().collect();
    let b_chars: std::collections::HashSet<char> = b.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn condense_unified_diff(diff: &str) -> String {
    let mut result = Vec::new();
    let mut current_file = String::new();
    let mut added = 0;
    let mut removed = 0;
    let mut changes = Vec::new();

    for line in diff.lines() {
        if line.starts_with("diff --git") || line.starts_with("--- ") || line.starts_with("+++ ") {
            // File header
            if line.starts_with("+++ ") {
                if !current_file.is_empty() && (added > 0 || removed > 0) {
                    result.push(format!("📄 {} (+{} -{})", current_file, added, removed));
                    for c in changes.iter().take(10) {
                        result.push(format!("  {}", c));
                    }
                    if changes.len() > 10 {
                        result.push(format!("  ... +{} more", changes.len() - 10));
                    }
                }
                current_file = line.trim_start_matches("+++ ").trim_start_matches("b/").to_string();
                added = 0;
                removed = 0;
                changes.clear();
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
            if changes.len() < 15 {
                changes.push(truncate(line, 70));
            }
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
            if changes.len() < 15 {
                changes.push(truncate(line, 70));
            }
        }
    }

    // Last file
    if !current_file.is_empty() && (added > 0 || removed > 0) {
        result.push(format!("📄 {} (+{} -{})", current_file, added, removed));
        for c in changes.iter().take(10) {
            result.push(format!("  {}", c));
        }
        if changes.len() > 10 {
            result.push(format!("  ... +{} more", changes.len() - 10));
        }
    }

    result.join("\n")
}
