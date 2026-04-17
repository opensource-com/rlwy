use crate::api::{Deployment, Project, Railway};
use crate::commands::watch;
use crate::config;
use crate::ui;
use anyhow::{Result, bail};
use colored::Colorize;
use tabled::settings::Style;
use tabled::{Table, Tabled};

const MESSAGE_MAX: usize = 50;
const AUTHOR_MAX: usize = 18;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "#")]
    index: String,
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "CREATED")]
    created: String,
    #[tabled(rename = "COMMIT")]
    commit: String,
    #[tabled(rename = "AUTHOR")]
    author: String,
    #[tabled(rename = "MESSAGE")]
    message: String,
    #[tabled(rename = "ID")]
    id: String,
}

pub async fn run(
    query: Option<String>,
    pick: bool,
    env: Option<String>,
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

    println!(
        "{} {} › {} {}",
        "→".cyan().bold(),
        ctx.project_name.bold(),
        ctx.service_name.bold(),
        match &ctx.env_name {
            Some(n) => format!("[{n}]").dimmed(),
            None => "".normal(),
        }
    );

    if deployments.is_empty() {
        println!("  {}", "(no deployments)".dimmed());
        return Ok(());
    }

    let rows: Vec<Row> = deployments
        .iter()
        .enumerate()
        .map(|(i, d)| Row {
            index: format!("{}", i + 1),
            status: ui::color_status(d.status).to_string(),
            created: d
                .created_at
                .as_deref()
                .map(short_time)
                .unwrap_or_else(|| "—".dimmed().to_string()),
            commit: commit_cell(d),
            author: author_cell(d),
            message: message_cell(d),
            id: d.id.dimmed().to_string(),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    println!("{table}");
    println!();
    println!(
        "{}",
        format!(
            "tip: `rlwy logs <ID>` for logs • `rlwy rollback {}` to restore an older one",
            ctx.service_name
        )
        .dimmed()
    );
    Ok(())
}

struct Ctx {
    project_id: String,
    project_name: String,
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
                    project_name: p.name.clone(),
                    service_name: s.name.clone(),
                    env_id,
                    env_name,
                });
            }
        }
    }
    bail!("service {service_id} not found in accessible projects")
}

fn commit_cell(d: &Deployment) -> String {
    match d.commit_hash() {
        Some(h) if !h.is_empty() => {
            let short: String = h.chars().take(7).collect();
            short.cyan().to_string()
        }
        _ => "—".dimmed().to_string(),
    }
}

fn author_cell(d: &Deployment) -> String {
    match d.commit_author() {
        Some(a) if !a.trim().is_empty() => truncate(a.trim(), AUTHOR_MAX),
        _ => "—".dimmed().to_string(),
    }
}

fn message_cell(d: &Deployment) -> String {
    match d.commit_message() {
        Some(m) if !m.trim().is_empty() => {
            truncate(m.lines().next().unwrap_or("").trim(), MESSAGE_MAX)
        }
        _ => "—".dimmed().to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max - 1).collect();
    format!("{head}…")
}

fn short_time(iso: &str) -> String {
    // Strip subseconds + "T"/"Z" for a compact yyyy-mm-dd hh:mm display.
    let compact = iso.replace('T', " ");
    match compact.split('.').next() {
        Some(head) => head.trim_end_matches('Z').to_string(),
        None => compact,
    }
}
