use crate::api::{DeploymentStatus, Railway};
use crate::config;
use crate::ui;
use anyhow::{Context, Result, anyhow, bail};
use colored::Colorize;
use dialoguer::FuzzySelect;
use std::collections::VecDeque;
use std::time::Duration;

pub async fn run(query: Option<String>, interval_secs: u64, pick: bool) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let Some(ctx) = api.latest_deployment(&service_id).await? else {
        println!(
            "{} no deployments found for service {}",
            "!".yellow().bold(),
            service_id.dimmed()
        );
        return Ok(());
    };

    let env_label = env_label(&ctx.env_name);
    let mut current = ctx.deployment;

    ui::print_banner();
    println!(
        "watching service {} {}  deployment {}",
        service_id.cyan(),
        env_label,
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

pub(crate) fn env_label(name: &Option<String>) -> colored::ColoredString {
    match name {
        Some(n) => format!("[{n}]").dimmed(),
        None => "".normal(),
    }
}

pub async fn logs(
    query: Option<String>,
    pick: bool,
    follow: bool,
    since: Option<String>,
    grep: Option<String>,
    interval_secs: u64,
) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let (deployment_id, env_name) =
        resolve_deployment_for_logs(&api, query.as_deref(), pick).await?;
    let start_date = since.as_deref().map(parse_since).transpose()?;
    let start_iso = start_date.map(|dt| dt.to_rfc3339());
    let grep_lower = grep.as_deref().map(|s| s.to_ascii_lowercase());

    println!(
        "{} deployment {} {}",
        "══".bright_magenta(),
        deployment_id.dimmed(),
        env_label(&env_name)
    );
    if let Some(iso) = &start_iso {
        println!("   since: {}", iso.dimmed());
    }
    if let Some(g) = &grep_lower {
        println!("   grep:  {}", g.dimmed());
    }
    println!();

    println!("{} build logs", "══".bright_magenta());
    let build = api
        .build_logs(&deployment_id, 500, start_iso.as_deref())
        .await
        .context("fetching build logs")?;
    let build_shown = print_lines(&build, grep_lower.as_deref());
    if build_shown == 0 {
        println!("  {}", "(no build logs in this window)".dimmed());
    }

    println!();
    println!("{} deploy logs", "══".bright_magenta());
    let deploy = api
        .deployment_logs(&deployment_id, 500, start_iso.as_deref())
        .await
        .context("fetching deploy logs")?;
    let deploy_shown = print_lines(&deploy, grep_lower.as_deref());
    if deploy_shown == 0 && !follow {
        println!("  {}", "(no deploy logs in this window)".dimmed());
    }

    if !follow {
        return Ok(());
    }

    let mut last_ts = deploy
        .iter()
        .filter_map(|l| l.timestamp.clone())
        .max();
    let mut seen_recent: VecDeque<(Option<String>, String)> = deploy
        .iter()
        .rev()
        .take(64)
        .map(|l| (l.timestamp.clone(), l.message.clone()))
        .collect();

    println!();
    println!(
        "{} following (ctrl-c to exit)",
        "↻".cyan().bold()
    );

    let sleep = Duration::from_secs(interval_secs.max(1));
    loop {
        tokio::select! {
            _ = tokio::time::sleep(sleep) => {}
            _ = tokio::signal::ctrl_c() => {
                println!();
                return Ok(());
            }
        }

        let next = match api
            .deployment_logs(&deployment_id, 500, last_ts.as_deref())
            .await
        {
            Ok(v) => v,
            Err(err) => {
                eprintln!("{} {err}", "warn:".yellow());
                continue;
            }
        };

        for line in &next {
            let key = (line.timestamp.clone(), line.message.clone());
            if seen_recent.contains(&key) {
                continue;
            }
            if !passes_grep(&line.message, grep_lower.as_deref()) {
                continue;
            }
            ui::print_log_line(line);

            if let Some(ts) = &line.timestamp
                && last_ts.as_deref().map_or(true, |prev| ts.as_str() > prev)
            {
                last_ts = Some(ts.clone());
            }
            seen_recent.push_back(key);
            if seen_recent.len() > 128 {
                seen_recent.pop_front();
            }
        }
    }
}

fn print_lines(lines: &[crate::api::LogLine], grep_lower: Option<&str>) -> usize {
    let mut shown = 0;
    for line in lines {
        if !passes_grep(&line.message, grep_lower) {
            continue;
        }
        ui::print_log_line(line);
        shown += 1;
    }
    shown
}

fn passes_grep(msg: &str, grep_lower: Option<&str>) -> bool {
    match grep_lower {
        Some(g) => msg.to_ascii_lowercase().contains(g),
        None => true,
    }
}

fn parse_since(raw: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    let s = raw.trim();
    if s.is_empty() {
        bail!("--since is empty");
    }
    let split_at = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num_str, unit) = s.split_at(split_at);
    let num: i64 = num_str
        .trim()
        .parse()
        .with_context(|| format!("--since needs a number before the unit, got '{raw}'"))?;
    let dur = match unit.trim() {
        "s" | "sec" | "secs" | "second" | "seconds" => chrono::Duration::seconds(num),
        "" | "m" | "min" | "mins" | "minute" | "minutes" => chrono::Duration::minutes(num),
        "h" | "hr" | "hrs" | "hour" | "hours" => chrono::Duration::hours(num),
        "d" | "day" | "days" => chrono::Duration::days(num),
        other => bail!("unknown --since unit '{other}' (use s/m/h/d)"),
    };
    Ok(chrono::Utc::now() - dur)
}

pub(crate) async fn resolve_service(
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
) -> Result<(String, Option<String>)> {
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
                            return latest_deployment_with_env(api, q).await;
                        }
                    }
                }
                return Ok((q.to_string(), None));
            }
            let service_id = resolve_by_name(api, q).await?;
            let _ = config::remember_service(&service_id);
            return latest_deployment_with_env(api, &service_id).await;
        }
        if let Some(id) = config::load().ok().and_then(|c| c.last_service_id) {
            println!(
                "{} resuming last service {} (pass {} to pick another)",
                "↻".cyan(),
                id.cyan(),
                "--pick".bold()
            );
            return latest_deployment_with_env(api, &id).await;
        }
    }
    let service_id = pick_service_interactively(api).await?;
    let _ = config::remember_service(&service_id);
    latest_deployment_with_env(api, &service_id).await
}

async fn latest_deployment_with_env(
    api: &Railway,
    service_id: &str,
) -> Result<(String, Option<String>)> {
    let ctx = api
        .latest_deployment(service_id)
        .await?
        .ok_or_else(|| anyhow!("no deployments found for service {service_id}"))?;
    Ok((ctx.deployment.id, ctx.env_name))
}
