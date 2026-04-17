use crate::api::Railway;
use crate::commands::watch;
use crate::config;
use anyhow::{Context, Result, bail};
use colored::Colorize;

const DEFAULT_DASHBOARD: &str = "https://railway.com";

pub async fn run(query: Option<String>, pick: bool, env: Option<String>) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;

    let service_id = watch::resolve_service(&api, query.as_deref(), pick).await?;
    let _ = config::remember_service(&service_id);

    let projects = api.projects().await?;
    let (project_id, project_name, service_name, env_id) =
        find_service_context(&projects, &service_id, env.as_deref())?;

    let base = std::env::var("RLWY_DASHBOARD_URL").unwrap_or_else(|_| DEFAULT_DASHBOARD.into());
    let mut url = format!(
        "{}/project/{}?service={}",
        base.trim_end_matches('/'),
        project_id,
        service_id
    );
    if let Some(eid) = &env_id {
        url.push_str("&environmentId=");
        url.push_str(eid);
    }

    println!(
        "{} opening {} › {}",
        "→".cyan().bold(),
        project_name.bold(),
        service_name.bold()
    );
    println!("  {}", url.underline());

    open::that(&url).context("failed to open browser")?;
    Ok(())
}

fn find_service_context(
    projects: &[crate::api::Project],
    service_id: &str,
    env_name: Option<&str>,
) -> Result<(String, String, String, Option<String>)> {
    for p in projects {
        for s in p.services() {
            if s.id == service_id {
                let env_id = match env_name {
                    Some(name) => {
                        let found = p
                            .environments()
                            .iter()
                            .find(|e| e.name.eq_ignore_ascii_case(name))
                            .map(|e| e.id.clone());
                        match found {
                            Some(id) => Some(id),
                            None => {
                                let available: Vec<_> = p
                                    .environments()
                                    .iter()
                                    .map(|e| e.name.clone())
                                    .collect();
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
                    None => s
                        .latest_deployment()
                        .and_then(|d| d.environment_id.clone()),
                };
                return Ok((
                    p.id.clone(),
                    p.name.clone(),
                    s.name.clone(),
                    env_id,
                ));
            }
        }
    }
    bail!("service {service_id} not found in accessible projects")
}
