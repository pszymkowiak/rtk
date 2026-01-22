use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum ContainerCmd {
    DockerPs,
    DockerImages,
    DockerLogs,
    KubectlPods,
    KubectlServices,
    KubectlLogs,
}

pub fn run(cmd: ContainerCmd, args: &[String], verbose: u8) -> Result<()> {
    match cmd {
        ContainerCmd::DockerPs => docker_ps(verbose),
        ContainerCmd::DockerImages => docker_images(verbose),
        ContainerCmd::DockerLogs => docker_logs(args, verbose),
        ContainerCmd::KubectlPods => kubectl_pods(args, verbose),
        ContainerCmd::KubectlServices => kubectl_services(args, verbose),
        ContainerCmd::KubectlLogs => kubectl_logs(args, verbose),
    }
}

fn docker_ps(_verbose: u8) -> Result<()> {
    let output = Command::new("docker")
        .args(["ps", "--format", "{{.Names}}\t{{.Status}}\t{{.Image}}\t{{.Ports}}"])
        .output()
        .context("Failed to run docker ps")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() {
        println!("🐳 No running containers");
        return Ok(());
    }

    println!("🐳 Running Containers:");
    println!("{:<25} {:<15} {:<30} {}", "NAME", "STATUS", "IMAGE", "PORTS");
    println!("{}", "-".repeat(80));

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let name = truncate(parts[0], 24);
            let status = truncate(parts.get(1).unwrap_or(&""), 14);
            let image = truncate(parts.get(2).unwrap_or(&""), 29);
            let ports = compact_ports(parts.get(3).unwrap_or(&""));
            println!("{:<25} {:<15} {:<30} {}", name, status, image, ports);
        }
    }

    Ok(())
}

fn docker_images(_verbose: u8) -> Result<()> {
    let output = Command::new("docker")
        .args(["images", "--format", "{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}"])
        .output()
        .context("Failed to run docker images")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("🐳 Docker Images:");
    println!("{:<50} {:<10} {}", "IMAGE", "SIZE", "CREATED");
    println!("{}", "-".repeat(75));

    let mut count = 0;
    for line in stdout.lines().take(20) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let image = truncate(parts[0], 49);
            let size = parts.get(1).unwrap_or(&"");
            let created = parts.get(2).unwrap_or(&"");
            println!("{:<50} {:<10} {}", image, size, created);
            count += 1;
        }
    }

    let total: usize = stdout.lines().count();
    if total > 20 {
        println!("... +{} more images", total - 20);
    }

    Ok(())
}

fn docker_logs(args: &[String], _verbose: u8) -> Result<()> {
    let container = args.first().map(|s| s.as_str()).unwrap_or("");
    if container.is_empty() {
        println!("Usage: rtk docker logs <container>");
        return Ok(());
    }

    let output = Command::new("docker")
        .args(["logs", "--tail", "100", container])
        .output()
        .context("Failed to run docker logs")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    // Use log deduplication
    let analyzed = crate::log_cmd::run_stdin_str(&combined);
    println!("🐳 Logs for {}:", container);
    println!("{}", analyzed);

    Ok(())
}

fn kubectl_pods(args: &[String], _verbose: u8) -> Result<()> {
    let mut cmd = Command::new("kubectl");
    cmd.args(["get", "pods", "-o", "wide"]);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().context("Failed to run kubectl get pods")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("☸️  Kubernetes Pods:");

    let lines: Vec<&str> = stdout.lines().collect();
    if lines.is_empty() {
        println!("No pods found");
        return Ok(());
    }

    // Header
    if let Some(header) = lines.first() {
        println!("{}", header);
        println!("{}", "-".repeat(80));
    }

    // Count by status
    let mut running = 0;
    let mut pending = 0;
    let mut failed = 0;
    let mut other = 0;

    for line in lines.iter().skip(1).take(20) {
        println!("{}", truncate(line, 100));

        if line.contains("Running") {
            running += 1;
        } else if line.contains("Pending") {
            pending += 1;
        } else if line.contains("Failed") || line.contains("Error") || line.contains("CrashLoop") {
            failed += 1;
        } else {
            other += 1;
        }
    }

    if lines.len() > 21 {
        println!("... +{} more pods", lines.len() - 21);
    }

    println!();
    println!("📊 Summary: {} running, {} pending, {} failed, {} other", running, pending, failed, other);

    Ok(())
}

fn kubectl_services(args: &[String], _verbose: u8) -> Result<()> {
    let mut cmd = Command::new("kubectl");
    cmd.args(["get", "services", "-o", "wide"]);

    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().context("Failed to run kubectl get services")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("☸️  Kubernetes Services:");

    for (i, line) in stdout.lines().enumerate() {
        if i == 0 {
            println!("{}", line);
            println!("{}", "-".repeat(80));
        } else if i <= 15 {
            println!("{}", truncate(line, 100));
        }
    }

    let total = stdout.lines().count();
    if total > 16 {
        println!("... +{} more services", total - 16);
    }

    Ok(())
}

fn kubectl_logs(args: &[String], _verbose: u8) -> Result<()> {
    let pod = args.first().map(|s| s.as_str()).unwrap_or("");
    if pod.is_empty() {
        println!("Usage: rtk kubectl logs <pod>");
        return Ok(());
    }

    let mut cmd = Command::new("kubectl");
    cmd.args(["logs", "--tail", "100", pod]);

    // Add remaining args (like container name, -c, etc.)
    for arg in args.iter().skip(1) {
        cmd.arg(arg);
    }

    let output = cmd.output().context("Failed to run kubectl logs")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let analyzed = crate::log_cmd::run_stdin_str(&stdout);
    println!("☸️  Logs for {}:", pod);
    println!("{}", analyzed);

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

fn compact_ports(ports: &str) -> String {
    if ports.is_empty() {
        return "-".to_string();
    }

    // Extract just the port numbers
    let port_nums: Vec<&str> = ports
        .split(',')
        .filter_map(|p| {
            p.split("->")
                .next()
                .and_then(|s| s.split(':').last())
        })
        .collect();

    if port_nums.len() <= 3 {
        port_nums.join(", ")
    } else {
        format!("{}, ... +{}", port_nums[..2].join(", "), port_nums.len() - 2)
    }
}
