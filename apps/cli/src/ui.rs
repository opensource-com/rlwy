use crate::api::{Deployment, DeploymentStatus, LogLine};
use colored::{ColoredString, Colorize};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub fn color_status(status: DeploymentStatus) -> ColoredString {
    let label = status.label();
    match status {
        DeploymentStatus::Success => label.green().bold(),
        DeploymentStatus::Failed | DeploymentStatus::Crashed => label.red().bold(),
        DeploymentStatus::Building | DeploymentStatus::Deploying => label.cyan().bold(),
        DeploymentStatus::Queued
        | DeploymentStatus::Initializing
        | DeploymentStatus::Waiting => label.yellow().bold(),
        DeploymentStatus::Removed | DeploymentStatus::Removing | DeploymentStatus::Skipped => {
            label.dimmed()
        }
        DeploymentStatus::Unknown => label.normal(),
    }
}

pub fn print_banner() {
    println!(
        "{} {}",
        "▲".bright_magenta().bold(),
        "rlwy — Railway deployment watcher".bold()
    );
}

pub fn make_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.cyan} [{elapsed_precise}] [{bar:30.cyan/blue}] {percent:>3}%  {msg}",
        )
        .expect("valid template")
        .progress_chars("██░")
        .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

pub fn update_progress(pb: &ProgressBar, dep: &Deployment) {
    let pct = (dep.status.progress_fraction() * 100.0).round() as u64;
    pb.set_position(pct);
    pb.set_message(format!(
        "{}  {}",
        color_status(dep.status),
        dep.id.dimmed()
    ));
}

pub fn finish_progress(pb: &ProgressBar, dep: &Deployment) {
    pb.set_position(100);
    let line = format!("{}  {}", color_status(dep.status), dep.id.dimmed());
    match dep.status {
        DeploymentStatus::Success => pb.finish_with_message(line),
        _ => pb.abandon_with_message(line),
    }
}

pub fn print_log_line(line: &LogLine) {
    let ts = line.timestamp.as_deref().unwrap_or("");
    let severity = line.severity.as_deref().unwrap_or("");
    let sev_colored: ColoredString = match severity.to_ascii_lowercase().as_str() {
        "error" | "err" => severity.red().bold(),
        "warn" | "warning" => severity.yellow().bold(),
        "info" => severity.cyan(),
        "debug" => severity.dimmed(),
        _ => severity.normal(),
    };
    if severity.is_empty() {
        println!("{}  {}", ts.dimmed(), line.message);
    } else {
        println!("{}  {}  {}", ts.dimmed(), sev_colored, line.message);
    }
}
