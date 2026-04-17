use crate::api::{Deployment, DeploymentStatus, Project, Railway};
use crate::commands::watch;
use crate::config;
use crate::ui;
use anyhow::{Result, bail};
use colored::Colorize;
use dialoguer::FuzzySelect;
use std::time::Duration;

pub async fn run(
    query: Option<String>,
    pick: bool,
    no_watch: bool,
    env: Option<String>,
    to: Option<String>,
    limit: i32,
) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = watch::resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let projects = api.projects().await?;
    let ctx = find_ctx(&projects, &service_id, env.as_deref())?;

    let deployments = api
        .deployments_for_service(&ctx.project_id, ctx.env_id.as_deref(), &service_id, limit)
        .await?;

    if deployments.is_empty() {
        bail!(
            "no deployments found for service '{}' in env '{}'",
            ctx.service_name,
            ctx.env_name.as_deref().unwrap_or("(unknown)")
        );
    }

    let target = choose_target(&deployments, to.as_deref())?;

    println!(
        "{} rolling {} {} back to deployment {}",
        "↺".yellow().bold(),
        ctx.service_name.bold(),
        match &ctx.env_name {
            Some(n) => format!("[{n}]").dimmed(),
            None => "".normal(),
        },
        target.id.cyan()
    );
    if let Some(msg) = target.commit_message() {
        let short = msg.lines().next().unwrap_or("").trim();
        if !short.is_empty() {
            println!("  commit: {}", short.dimmed());
        }
    }

    let fresh = api.rollback_to_deployment(&target.id).await?;

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
            println!("{} rollback succeeded", "✓".green().bold());
            if let Some(url) = &current.static_url {
                println!("  {}", url.underline());
            }
        }
        DeploymentStatus::Failed | DeploymentStatus::Crashed => {
            println!(
                "{} rollback {}",
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

fn choose_target<'a>(
    deployments: &'a [Deployment],
    to: Option<&str>,
) -> Result<&'a Deployment> {
    if let Some(id_or_short) = to.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(d) = deployments.iter().find(|d| d.id == id_or_short) {
            return Ok(d);
        }
        if let Some(d) = deployments
            .iter()
            .find(|d| d.commit_hash().map(|h| h.starts_with(id_or_short)).unwrap_or(false))
        {
            return Ok(d);
        }
        bail!(
            "no deployment in the last {} matched '{}'",
            deployments.len(),
            id_or_short
        );
    }

    // Default: pick the most recent SUCCESS deployment that isn't the current one.
    // The list is newest-first; element [0] is the current deployment.
    let (skip, candidates): (usize, Vec<&Deployment>) = (
        1,
        deployments
            .iter()
            .skip(1)
            .filter(|d| d.status == DeploymentStatus::Success)
            .collect(),
    );
    let _ = skip;
    if candidates.is_empty() {
        bail!(
            "no earlier SUCCESS deployment found in the last {} — pass --to <id|sha> to pick one explicitly",
            deployments.len()
        );
    }
    if candidates.len() == 1 {
        return Ok(candidates[0]);
    }

    let items: Vec<String> = candidates
        .iter()
        .map(|d| {
            let sha = d
                .commit_hash()
                .map(|h| h.chars().take(7).collect::<String>())
                .unwrap_or_else(|| "—".into());
            let msg = d
                .commit_message()
                .and_then(|m| m.lines().next())
                .unwrap_or("(no commit message)")
                .trim();
            format!("{sha}  {msg}")
        })
        .collect();
    let refs: Vec<&str> = items.iter().map(String::as_str).collect();
    let sel = FuzzySelect::new()
        .with_prompt("pick a deployment to roll back to")
        .default(0)
        .items(&refs)
        .interact()?;
    Ok(candidates[sel])
}

struct Ctx {
    project_id: String,
    service_name: String,
    env_id: Option<String>,
    env_name: Option<String>,
}

fn find_ctx(projects: &[Project], service_id: &str, env_name: Option<&str>) -> Result<Ctx> {
    for p in projects {
        for s in p.services() {
            if s.id == service_id {
                let (env_id, env_name) = match env_name {
                    Some(name) => {
                        let envs = p.environments();
                        let found = envs.iter().find(|e| e.name.eq_ignore_ascii_case(name));
                        match found {
                            Some(e) => (Some(e.id.clone()), Some(e.name.clone())),
                            None => {
                                let available: Vec<_> =
                                    envs.iter().map(|e| e.name.clone()).collect();
                                bail!(
                                    "project '{}' has no environment named '{}' (available: {})",
                                    p.name,
                                    name,
                                    if available.is_empty() {
                                        "(none)".to_string()
                                    } else {
                                        available.join(", ")
                                    }
                                );
                            }
                        }
                    }
                    None => {
                        let eid = s
                            .latest_deployment()
                            .and_then(|d| d.environment_id.clone());
                        let ename = eid
                            .as_deref()
                            .and_then(|id| p.env_name(id))
                            .map(str::to_string);
                        (eid, ename)
                    }
                };
                return Ok(Ctx {
                    project_id: p.id.clone(),
                    service_name: s.name.clone(),
                    env_id,
                    env_name,
                });
            }
        }
    }
    bail!("service {service_id} not found in accessible projects")
}
