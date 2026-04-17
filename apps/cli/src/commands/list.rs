use crate::api::{Deployment, Project, Railway};
use crate::config;
use crate::ui;
use anyhow::{Result, bail};
use colored::Colorize;
use tabled::settings::Style;
use tabled::{Table, Tabled};

const MESSAGE_MAX: usize = 56;
const AUTHOR_MAX: usize = 18;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "SERVICE")]
    service: String,
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "COMMIT")]
    commit: String,
    #[tabled(rename = "AUTHOR")]
    author: String,
    #[tabled(rename = "MESSAGE")]
    message: String,
}

pub async fn run(query: Option<String>) -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;
    let all_projects = api.projects().await?;

    if all_projects.is_empty() {
        println!(
            "{} no projects found for this token",
            "!".yellow().bold()
        );
        return Ok(());
    }

    let projects: Vec<&Project> = match query.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
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

    ui::print_banner();

    for project in &projects {
        let project = *project;
        println!();
        print_project_header(project);

        let services = project.services();
        if services.is_empty() {
            println!("  {}", "(no services)".dimmed());
            continue;
        }

        let rows: Vec<Row> = services
            .iter()
            .map(|svc| {
                let (status, commit, author, message) = match svc.latest_deployment() {
                    Some(d) => (
                        ui::color_status(d.status).to_string(),
                        commit_cell(d),
                        author_cell(d),
                        message_cell(d),
                    ),
                    None => (
                        "NO DEPLOYMENTS".dimmed().to_string(),
                        em_dash(),
                        em_dash(),
                        em_dash(),
                    ),
                };
                Row {
                    service: svc.name.clone(),
                    status,
                    commit,
                    author,
                    message,
                }
            })
            .collect();

        let mut table = Table::new(rows);
        table.with(Style::rounded());
        println!("{table}");
    }

    println!();
    println!(
        "{}",
        "tip: `rlwy watch <service-id>` to follow a deployment".dimmed()
    );
    Ok(())
}

fn print_project_header(project: &Project) {
    println!(
        "{} {}   {}",
        "■".bright_magenta(),
        project.name.bold(),
        project.id.dimmed()
    );
}

fn commit_cell(d: &Deployment) -> String {
    match d.commit_hash() {
        Some(h) if !h.is_empty() => {
            let short: String = h.chars().take(7).collect();
            short.cyan().to_string()
        }
        _ => em_dash(),
    }
}

fn message_cell(d: &Deployment) -> String {
    match d.commit_message() {
        Some(m) if !m.trim().is_empty() => {
            truncate(m.lines().next().unwrap_or("").trim(), MESSAGE_MAX)
        }
        _ => em_dash(),
    }
}

fn author_cell(d: &Deployment) -> String {
    match d.commit_author() {
        Some(a) if !a.trim().is_empty() => truncate(a.trim(), AUTHOR_MAX),
        _ => em_dash(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max - 1).collect();
    format!("{head}…")
}

fn em_dash() -> String {
    "—".dimmed().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_table_with_sample_rows() {
        let rows = vec![
            Row {
                service: "api".into(),
                status: "SUCCESS".green().to_string(),
                commit: "a1b2c3d".cyan().to_string(),
                author: "Alice".into(),
                message: "fix: retry on 502".into(),
            },
            Row {
                service: "worker".into(),
                status: "BUILDING".cyan().to_string(),
                commit: "4e5f6a7".cyan().to_string(),
                author: "Bob".into(),
                message: "feat: add queue metrics".into(),
            },
        ];
        let mut table = Table::new(rows);
        table.with(Style::rounded());
        let out = table.to_string();
        assert!(out.contains("SERVICE"));
        assert!(out.contains("api"));
        assert!(out.contains("worker"));
        assert!(out.contains("fix: retry on 502"));
        eprintln!("{out}");
    }
}
