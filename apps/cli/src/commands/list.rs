use crate::api::Railway;
use crate::config;
use crate::ui;
use anyhow::Result;
use colored::Colorize;

pub async fn run() -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;
    let projects = api.projects().await?;

    if projects.is_empty() {
        println!(
            "{} no projects found for this token",
            "!".yellow().bold()
        );
        return Ok(());
    }

    ui::print_banner();
    println!();

    for project in &projects {
        println!(
            "{} {}   {}",
            "■".bright_magenta(),
            project.name.bold(),
            project.id.dimmed()
        );

        let services = project.services();
        if services.is_empty() {
            println!("   {}", "(no services)".dimmed());
            continue;
        }

        for svc in services {
            let status_str = match svc.latest_deployment() {
                Some(d) => format!("{}", ui::color_status(d.status)),
                None => "NO DEPLOYMENTS".dimmed().to_string(),
            };
            println!(
                "   {} {:<24} {:<14}  {}",
                "•".cyan(),
                svc.name,
                status_str,
                svc.id.dimmed()
            );
        }
        println!();
    }

    println!(
        "{}",
        "tip: `rlwy watch <service-id>` to follow a deployment".dimmed()
    );
    Ok(())
}
