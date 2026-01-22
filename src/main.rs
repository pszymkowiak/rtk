mod container;
mod deps;
mod diff_cmd;
mod env_cmd;
mod filter;
mod find_cmd;
mod git;
mod json_cmd;
mod local_llm;
mod log_cmd;
mod ls;
mod read;
mod runner;
mod summary;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rtk",
    version,
    about = "Rust Token Killer - Minimize LLM token consumption",
    long_about = "A high-performance CLI proxy designed to filter and summarize system outputs before they reach your LLM context."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// List directory contents in ultra-dense, token-optimized format
    Ls {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(short, long, default_value = "10")]
        depth: usize,
        #[arg(short = 'a', long)]
        all: bool,
        #[arg(short, long, default_value = "tree")]
        format: ls::OutputFormat,
    },

    /// Read file with intelligent filtering
    Read {
        file: PathBuf,
        #[arg(short, long, default_value = "minimal")]
        level: filter::FilterLevel,
        #[arg(short, long)]
        max_lines: Option<usize>,
        #[arg(short = 'n', long)]
        line_numbers: bool,
    },

    /// Generate 2-line technical summary (heuristic-based)
    Smart {
        file: PathBuf,
        #[arg(short, long, default_value = "heuristic")]
        model: String,
        #[arg(long)]
        force_download: bool,
    },

    /// Git commands with compact output
    Git {
        #[command(subcommand)]
        command: GitCommands,
    },

    /// Run command and show only errors/warnings
    Err {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Run tests and show only failures
    Test {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },

    /// Show JSON structure without values
    Json {
        file: PathBuf,
        #[arg(short, long, default_value = "5")]
        depth: usize,
    },

    /// Summarize project dependencies
    Deps {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show environment variables (filtered, sensitive masked)
    Env {
        #[arg(short, long)]
        filter: Option<String>,
        #[arg(long)]
        show_all: bool,
    },

    /// Find files with compact tree output
    Find {
        /// Pattern to search (glob)
        pattern: String,
        /// Path to search in
        #[arg(default_value = ".")]
        path: String,
        /// Maximum results to show
        #[arg(short, long, default_value = "50")]
        max: usize,
    },

    /// Ultra-condensed diff (only changed lines)
    Diff {
        /// First file or - for stdin (unified diff)
        file1: PathBuf,
        /// Second file (optional if stdin)
        file2: Option<PathBuf>,
    },

    /// Filter and deduplicate log output
    Log {
        /// Log file (omit for stdin)
        file: Option<PathBuf>,
    },

    /// Docker commands with compact output
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },

    /// Kubectl commands with compact output
    Kubectl {
        #[command(subcommand)]
        command: KubectlCommands,
    },

    /// Run command and show heuristic summary
    Summary {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
enum GitCommands {
    Diff {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        #[arg(short, long)]
        max_lines: Option<usize>,
    },
    Log {
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
    },
    Status,
}

#[derive(Subcommand)]
enum DockerCommands {
    /// List running containers
    Ps,
    /// List images
    Images,
    /// Show container logs (deduplicated)
    Logs {
        container: String,
    },
}

#[derive(Subcommand)]
enum KubectlCommands {
    /// List pods
    Pods {
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// List services
    Services {
        #[arg(short, long)]
        namespace: Option<String>,
    },
    /// Show pod logs (deduplicated)
    Logs {
        pod: String,
        #[arg(short, long)]
        container: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ls { path, depth, all, format } => {
            ls::run(&path, depth, all, format, cli.verbose)?;
        }

        Commands::Read { file, level, max_lines, line_numbers } => {
            read::run(&file, level, max_lines, line_numbers, cli.verbose)?;
        }

        Commands::Smart { file, model, force_download } => {
            local_llm::run(&file, &model, force_download, cli.verbose)?;
        }

        Commands::Git { command } => match command {
            GitCommands::Diff { args, max_lines } => {
                git::run(git::GitCommand::Diff, &args, max_lines, cli.verbose)?;
            }
            GitCommands::Log { args, count } => {
                git::run(git::GitCommand::Log, &args, Some(count), cli.verbose)?;
            }
            GitCommands::Status => {
                git::run(git::GitCommand::Status, &[], None, cli.verbose)?;
            }
        },

        Commands::Err { command } => {
            let cmd = command.join(" ");
            runner::run_err(&cmd, cli.verbose)?;
        }

        Commands::Test { command } => {
            let cmd = command.join(" ");
            runner::run_test(&cmd, cli.verbose)?;
        }

        Commands::Json { file, depth } => {
            json_cmd::run(&file, depth, cli.verbose)?;
        }

        Commands::Deps { path } => {
            deps::run(&path, cli.verbose)?;
        }

        Commands::Env { filter, show_all } => {
            env_cmd::run(filter.as_deref(), show_all, cli.verbose)?;
        }

        Commands::Find { pattern, path, max } => {
            find_cmd::run(&pattern, &path, max, cli.verbose)?;
        }

        Commands::Diff { file1, file2 } => {
            if let Some(f2) = file2 {
                diff_cmd::run(&file1, &f2, cli.verbose)?;
            } else {
                diff_cmd::run_stdin(cli.verbose)?;
            }
        }

        Commands::Log { file } => {
            if let Some(f) = file {
                log_cmd::run_file(&f, cli.verbose)?;
            } else {
                log_cmd::run_stdin(cli.verbose)?;
            }
        }

        Commands::Docker { command } => match command {
            DockerCommands::Ps => {
                container::run(container::ContainerCmd::DockerPs, &[], cli.verbose)?;
            }
            DockerCommands::Images => {
                container::run(container::ContainerCmd::DockerImages, &[], cli.verbose)?;
            }
            DockerCommands::Logs { container: c } => {
                container::run(container::ContainerCmd::DockerLogs, &[c], cli.verbose)?;
            }
        },

        Commands::Kubectl { command } => match command {
            KubectlCommands::Pods { namespace } => {
                let args: Vec<String> = namespace.map(|n| vec!["-n".to_string(), n]).unwrap_or_default();
                container::run(container::ContainerCmd::KubectlPods, &args, cli.verbose)?;
            }
            KubectlCommands::Services { namespace } => {
                let args: Vec<String> = namespace.map(|n| vec!["-n".to_string(), n]).unwrap_or_default();
                container::run(container::ContainerCmd::KubectlServices, &args, cli.verbose)?;
            }
            KubectlCommands::Logs { pod, container: c } => {
                let mut args = vec![pod];
                if let Some(cont) = c {
                    args.push("-c".to_string());
                    args.push(cont);
                }
                container::run(container::ContainerCmd::KubectlLogs, &args, cli.verbose)?;
            }
        },

        Commands::Summary { command } => {
            let cmd = command.join(" ");
            summary::run(&cmd, cli.verbose)?;
        }
    }

    Ok(())
}
