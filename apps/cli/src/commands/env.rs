use crate::api::{Project, Railway};
use crate::commands::watch;
use crate::config;
use anyhow::{Result, bail};
use colored::Colorize;
use tabled::settings::Style;
use tabled::{Table, Tabled};

const VALUE_MAX: usize = 100;

#[derive(Tabled)]
struct EnvRow {
    #[tabled(rename = "NAME")]
    name: String,
    #[tabled(rename = "VALUE")]
    value: String,
}

pub async fn get(
    name: String,
    query: Option<String>,
    pick: bool,
    env: Option<String>,
) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = watch::resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let projects = api.projects().await?;
    let ctx = find_ctx(&projects, &service_id, env.as_deref())?;

    let Some(env_id) = ctx.env_id else {
        bail!(
            "could not determine environment for service '{}'. Pass --env <name> explicitly.",
            ctx.service_name
        );
    };

    let vars = api
        .variables(&ctx.project_id, &env_id, Some(&service_id))
        .await?;

    match vars.get(&name) {
        Some(value) => {
            println!("{value}");
            Ok(())
        }
        None => bail!(
            "variable '{}' not found on {} › {} {}",
            name,
            ctx.project_name,
            ctx.service_name,
            match &ctx.env_name {
                Some(n) => format!("[{n}]"),
                None => String::new(),
            }
        ),
    }
}

pub async fn ls(
    query: Option<String>,
    pick: bool,
    env: Option<String>,
    keys_only: bool,
) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = watch::resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let projects = api.projects().await?;
    let ctx = find_ctx(&projects, &service_id, env.as_deref())?;

    let Some(env_id) = ctx.env_id else {
        bail!(
            "could not determine environment for service '{}'. Pass --env <name> explicitly.",
            ctx.service_name
        );
    };

    let vars = api
        .variables(&ctx.project_id, &env_id, Some(&service_id))
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

    if vars.is_empty() {
        println!("  {}", "(no variables)".dimmed());
        return Ok(());
    }

    if keys_only {
        for k in vars.keys() {
            println!("{k}");
        }
        return Ok(());
    }

    let total = vars.len();
    let rows: Vec<EnvRow> = vars
        .into_iter()
        .map(|(k, v)| EnvRow {
            name: k,
            value: truncate_value(&v),
        })
        .collect();

    let mut table = Table::new(rows);
    table.with(Style::rounded());
    println!("{table}");
    println!();
    println!("{}", format!("{total} variables").dimmed());
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

fn truncate_value(s: &str) -> String {
    let first_line = s.lines().next().unwrap_or("").trim();
    if first_line.chars().count() <= VALUE_MAX {
        return first_line.to_string();
    }
    let head: String = first_line.chars().take(VALUE_MAX - 1).collect();
    format!("{head}…")
}
