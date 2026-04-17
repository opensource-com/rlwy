use crate::api::{DeploymentStatus, Railway};
use crate::config;
use crate::ui;
use anyhow::{Context, Result, anyhow};
use colored::Colorize;
use std::time::Duration;

pub async fn run(service_id: Option<String>, interval_secs: u64) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = match service_id {
        Some(id) => id,
        None => pick_service_interactively(&api).await?,
    };

    let Some(mut current) = api.latest_deployment(&service_id).await? else {
        println!(
            "{} no deployments found for service {}",
            "!".yellow().bold(),
            service_id.dimmed()
        );
        return Ok(());
    };

    ui::print_banner();
    println!(
        "watching service {}   deployment {}",
        service_id.cyan(),
        current.id.dimmed()
    );
    if let Some(url) = &current.static_url {
        println!("url: {}", url.underline());
    }
    println!();

    let pb = ui::make_progress_bar();
    ui::update_progress(&pb, &current);

    loop {
        if current.status.is_terminal() {
            ui::finish_progress(&pb, &current);
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(interval_secs.max(1))) => {}
            _ = tokio::signal::ctrl_c() => {
                pb.abandon_with_message("interrupted".yellow().to_string());
                return Ok(());
            }
        }
        match api.deployment(&current.id).await {
            Ok(next) => {
                current = next;
                ui::update_progress(&pb, &current);
            }
            Err(err) => {
                pb.println(format!("{} {err}", "warn:".yellow()));
            }
        }
    }

    println!();
    match current.status {
        DeploymentStatus::Success => {
            println!("{} deployment succeeded", "✓".green().bold());
            if let Some(url) = &current.static_url {
                println!("  {}", url.underline());
            }
        }
        DeploymentStatus::Failed | DeploymentStatus::Crashed => {
            println!(
                "{} deployment {}",
                "✗".red().bold(),
                current.status.label().to_lowercase()
            );
            println!(
                "  tail logs with: {}",
                format!("rlwy logs {}", current.id).cyan()
            );
        }
        other => {
            println!("finished in status {}", other.label());
        }
    }

    Ok(())
}

pub async fn logs(deployment_id: String) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    println!("{} build logs", "══".bright_magenta());
    let build = api
        .build_logs(&deployment_id, 500)
        .await
        .context("fetching build logs")?;
    if build.is_empty() {
        println!("  {}", "(no build logs yet)".dimmed());
    } else {
        for line in &build {
            ui::print_log_line(line);
        }
    }

    println!();
    println!("{} deploy logs", "══".bright_magenta());
    let deploy = api
        .deployment_logs(&deployment_id, 500)
        .await
        .context("fetching deploy logs")?;
    if deploy.is_empty() {
        println!("  {}", "(no deploy logs yet)".dimmed());
    } else {
        for line in &deploy {
            ui::print_log_line(line);
        }
    }
    Ok(())
}

async fn pick_service_interactively(api: &Railway) -> Result<String> {
    let projects = api.projects().await?;
    let mut options: Vec<(String, String)> = Vec::new();
    for p in &projects {
        for s in p.services() {
            options.push((
                format!("{}  ›  {}", p.name, s.name),
                s.id.clone(),
            ));
        }
    }
    if options.is_empty() {
        return Err(anyhow!("no services found for this token"));
    }

    println!("pick a service:");
    for (i, (label, _)) in options.iter().enumerate() {
        println!("  {:>2}. {}", i + 1, label);
    }
    eprint!("{} ", "›".cyan().bold());
    let mut buf = String::new();
    std::io::stdin()
        .read_line(&mut buf)
        .context("reading selection")?;
    let idx: usize = buf
        .trim()
        .parse()
        .map_err(|_| anyhow!("enter a number between 1 and {}", options.len()))?;
    let choice = options
        .get(idx.wrapping_sub(1))
        .ok_or_else(|| anyhow!("selection out of range"))?;
    Ok(choice.1.clone())
}
