use crate::api::{DeploymentStatus, Project, Railway};
use crate::config;
use crate::ui;
use anyhow::{Result, bail};
use colored::Colorize;

pub async fn run(project_query: Option<String>, show_all: bool) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;
    let all_projects = api.projects().await?;

    if all_projects.is_empty() {
        println!("{} no projects found for this token", "!".yellow().bold());
        return Ok(());
    }

    let projects: Vec<&Project> =
        match project_query.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => all_projects.iter().collect(),
            Some(q) => {
                let q_lower = q.to_ascii_lowercase();
                let matched: Vec<&Project> = all_projects
                    .iter()
                    .filter(|p| p.name.to_ascii_lowercase().contains(&q_lower))
                    .collect();
                if matched.is_empty() {
                    bail!("no project matches '{q}'");
                }
                matched
            }
        };

    let mut up = 0usize;
    let mut total = 0usize;
    let mut broken: Vec<(String, String, DeploymentStatus)> = Vec::new();
    let mut in_progress: Vec<(String, String, DeploymentStatus)> = Vec::new();

    for p in &projects {
        for s in p.services() {
            let Some(d) = s.latest_deployment() else {
                continue;
            };
            total += 1;
            match d.status {
                DeploymentStatus::Success => up += 1,
                DeploymentStatus::Failed | DeploymentStatus::Crashed => {
                    broken.push((p.name.clone(), s.name.clone(), d.status));
                }
                DeploymentStatus::Building
                | DeploymentStatus::Deploying
                | DeploymentStatus::Queued
                | DeploymentStatus::Initializing
                | DeploymentStatus::Waiting => {
                    in_progress.push((p.name.clone(), s.name.clone(), d.status));
                }
                _ => {}
            }
        }
    }

    let head_glyph = if broken.is_empty() {
        "✓".green().bold()
    } else {
        "!".yellow().bold()
    };
    println!("{} {}/{} services up", head_glyph, up, total);

    if !broken.is_empty() {
        println!();
        for (proj, svc, status) in &broken {
            println!(
                "  {}  {}/{}",
                ui::color_status(*status),
                proj.bold(),
                svc
            );
        }
    }

    if show_all && !in_progress.is_empty() {
        println!();
        for (proj, svc, status) in &in_progress {
            println!(
                "  {}  {}/{}",
                ui::color_status(*status),
                proj.bold(),
                svc
            );
        }
    }

    if !broken.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}
