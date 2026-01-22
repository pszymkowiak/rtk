use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

/// Run find/glob and show compact tree summary
pub fn run(pattern: &str, path: &str, max_results: usize, verbose: u8) -> Result<()> {
    if verbose > 0 {
        eprintln!("Finding: {} in {}", pattern, path);
    }

    // Use fd if available, otherwise find
    let output = Command::new("fd")
        .args([pattern, path, "--type", "f"])
        .output()
        .or_else(|_| {
            Command::new("find")
                .args([path, "-name", pattern, "-type", "f"])
                .output()
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = stdout.lines().collect();

    if files.is_empty() {
        println!("No files found matching '{}'", pattern);
        return Ok(());
    }

    // Group by directory
    let mut by_dir: HashMap<String, Vec<String>> = HashMap::new();

    for file in &files {
        let parts: Vec<&str> = file.rsplitn(2, '/').collect();
        let (filename, dir) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1].to_string())
        } else {
            (parts[0].to_string(), ".".to_string())
        };
        by_dir.entry(dir).or_default().push(filename);
    }

    // Sort directories
    let mut dirs: Vec<_> = by_dir.keys().collect();
    dirs.sort();

    // Print compact tree
    println!("📁 Found {} files in {} directories:", files.len(), dirs.len());
    println!();

    let mut shown = 0;
    for dir in dirs {
        if shown >= max_results {
            println!("... +{} more results", files.len() - shown);
            break;
        }

        let files_in_dir = &by_dir[dir];
        let dir_display = if dir.len() > 50 {
            format!("...{}", &dir[dir.len()-47..])
        } else {
            dir.clone()
        };

        if files_in_dir.len() <= 3 {
            // Show individual files
            println!("{}/ ({})", dir_display, files_in_dir.len());
            for f in files_in_dir {
                println!("  └─ {}", f);
                shown += 1;
            }
        } else {
            // Show summary
            println!("{}/ ({} files)", dir_display, files_in_dir.len());
            for f in files_in_dir.iter().take(2) {
                println!("  ├─ {}", f);
                shown += 1;
            }
            println!("  └─ ... +{} more", files_in_dir.len() - 2);
            shown += files_in_dir.len() - 2;
        }
    }

    // Extension summary
    let mut by_ext: HashMap<String, usize> = HashMap::new();
    for file in &files {
        let ext = file.rsplit('.').next().unwrap_or("(no ext)");
        *by_ext.entry(ext.to_string()).or_default() += 1;
    }

    if by_ext.len() > 1 {
        println!();
        print!("📊 Extensions: ");
        let mut exts: Vec<_> = by_ext.iter().collect();
        exts.sort_by(|a, b| b.1.cmp(a.1));
        let ext_str: Vec<String> = exts.iter().take(5).map(|(e, c)| format!(".{} ({})", e, c)).collect();
        println!("{}", ext_str.join(", "));
    }

    Ok(())
}
