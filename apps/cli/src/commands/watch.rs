use crate::api::{DeploymentStatus, Railway};
use crate::config;
use crate::ui;
use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;
use dialoguer::FuzzySelect;
use std::time::Duration;

pub async fn run(query: Option<String>, interval_secs: u64, pick: bool) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

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

pub async fn logs(query: Option<String>, pick: bool) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let deployment_id = resolve_deployment_for_logs(&api, query.as_deref(), pick).await?;
    println!("{} deployment {}", "══".bright_magenta(), deployment_id.dimmed());
    println!();

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

async fn resolve_service(
    api: &Railway,
    query: Option<&str>,
    force_pick: bool,
) -> Result<String> {
    if !force_pick {
        if let Some(q) = query {
            let trimmed = q.trim();
            if !trimmed.is_empty() {
                if looks_like_uuid(trimmed) {
                    return Ok(trimmed.to_string());
                }
                return resolve_by_name(api, trimmed).await;
            }
        } else if let Some(id) = config::load().ok().and_then(|c| c.last_service_id) {
            println!(
                "{} resuming last service {} (pass {} to pick another)",
                "↻".cyan(),
                id.cyan(),
                "--pick".bold()
            );
            return Ok(id);
        }
    }

    pick_service_interactively(api).await
}

fn looks_like_uuid(s: &str) -> bool {
    s.len() == 36
        && s.chars().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => c.is_ascii_hexdigit(),
        })
}

async fn resolve_by_name(api: &Railway, query: &str) -> Result<String> {
    let (proj_q, svc_q) = match query.split_once('/') {
        Some((p, s)) => (Some(p.trim()), s.trim()),
        None => (None, query),
    };
    if svc_q.is_empty() {
        bail!("service name is empty");
    }
    let proj_q_lower = proj_q.map(|s| s.to_ascii_lowercase());
    let svc_q_lower = svc_q.to_ascii_lowercase();

    let projects = api.projects().await?;
    let mut matches: Vec<(String, String, String)> = Vec::new();
    for p in &projects {
        if let Some(pf) = &proj_q_lower
            && !p.name.to_ascii_lowercase().contains(pf)
        {
            continue;
        }
        for s in p.services() {
            if s.name.to_ascii_lowercase().contains(&svc_q_lower) {
                matches.push((p.name.clone(), s.name.clone(), s.id.clone()));
            }
        }
    }

    match matches.len() {
        0 => Err(anyhow!("no service matches '{query}'")),
        1 => {
            let (proj, svc, id) = matches.pop().expect("len == 1");
            println!(
                "{} matched {} › {}",
                "→".dimmed(),
                proj.bold(),
                svc.bold()
            );
            Ok(id)
        }
        _ => {
            println!(
                "{} {} services match '{}', pick one:",
                "?".yellow().bold(),
                matches.len(),
                query
            );
            let options: Vec<(String, String)> = matches
                .into_iter()
                .map(|(p, s, id)| (format!("{p}  ›  {s}"), id))
                .collect();
            fuzzy_pick(&options, 0)
        }
    }
}

async fn pick_service_interactively(api: &Railway) -> Result<String> {
    let projects = api.projects().await?;
    let last_id = config::load().ok().and_then(|c| c.last_service_id);

    let mut options: Vec<(String, String)> = Vec::new();
    let mut default_idx = 0usize;
    for p in &projects {
        for s in p.services() {
            let label = format!("{}  ›  {}", p.name, s.name);
            if last_id.as_deref() == Some(&s.id) {
                default_idx = options.len();
            }
            options.push((label, s.id.clone()));
        }
    }
    if options.is_empty() {
        return Err(anyhow!("no services found for this token"));
    }
    fuzzy_pick(&options, default_idx)
}

fn fuzzy_pick(options: &[(String, String)], default_idx: usize) -> Result<String> {
    let items: Vec<&str> = options.iter().map(|(label, _)| label.as_str()).collect();
    let sel = FuzzySelect::new()
        .with_prompt("pick a service (type to filter)")
        .default(default_idx.min(items.len().saturating_sub(1)))
        .items(&items)
        .interact()
        .context("reading selection")?;
    Ok(options[sel].1.clone())
}

async fn resolve_deployment_for_logs(
    api: &Railway,
    query: Option<&str>,
    force_pick: bool,
) -> Result<String> {
    if !force_pick {
        if let Some(q) = query.map(str::trim).filter(|s| !s.is_empty()) {
            if looks_like_uuid(q) {
                // Check if the UUID is a known service; if so, use its latest deployment.
                // Otherwise treat the UUID as a raw deployment id.
                let projects = api.projects().await?;
                for p in &projects {
                    for s in p.services() {
                        if s.id == q {
                            let _ = config::remember_service(q);
                            return latest_deployment_id(api, q).await;
                        }
                    }
                }
                return Ok(q.to_string());
            }
            let service_id = resolve_by_name(api, q).await?;
            let _ = config::remember_service(&service_id);
            return latest_deployment_id(api, &service_id).await;
        }
        if let Some(id) = config::load().ok().and_then(|c| c.last_service_id) {
            println!(
                "{} resuming last service {} (pass {} to pick another)",
                "↻".cyan(),
                id.cyan(),
                "--pick".bold()
            );
            return latest_deployment_id(api, &id).await;
        }
    }
    let service_id = pick_service_interactively(api).await?;
    let _ = config::remember_service(&service_id);
    latest_deployment_id(api, &service_id).await
}

async fn latest_deployment_id(api: &Railway, service_id: &str) -> Result<String> {
    let dep = api
        .latest_deployment(service_id)
        .await?
        .ok_or_else(|| anyhow!("no deployments found for service {service_id}"))?;
    Ok(dep.id)
}
