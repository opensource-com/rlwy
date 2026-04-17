use crate::api::{DeploymentStatus, Railway};
use crate::commands::watch;
use crate::config;
use crate::ui;
use anyhow::{Result, anyhow};
use colored::Colorize;
use std::time::Duration;

pub async fn run(query: Option<String>, pick: bool, no_watch: bool) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = watch::resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let ctx = api
        .latest_deployment(&service_id)
        .await?
        .ok_or_else(|| {
            anyhow!("service {service_id} has no existing deployment to redeploy")
        })?;

    println!(
        "{} redeploying service {} {}",
        "↻".cyan().bold(),
        service_id.cyan(),
        watch::env_label(&ctx.env_name)
    );
    println!("   from deployment {}", ctx.deployment.id.dimmed());

    let fresh = api.redeploy_deployment(&ctx.deployment.id).await?;

    println!(
        "{} triggered new deployment {}",
        "✓".green().bold(),
        fresh.id.cyan()
    );

    if no_watch {
        return Ok(());
    }

    ui::print_banner();
    println!();

    let pb = ui::make_progress_bar();
    let mut current = fresh;
    ui::update_progress(&pb, &current);

    loop {
        if current.status.is_terminal() {
            ui::finish_progress(&pb, &current);
            break;
        }
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(3)) => {}
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
            println!("{} redeploy succeeded", "✓".green().bold());
            if let Some(url) = &current.static_url {
                println!("  {}", url.underline());
            }
        }
        DeploymentStatus::Failed | DeploymentStatus::Crashed => {
            println!(
                "{} redeploy {}",
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
