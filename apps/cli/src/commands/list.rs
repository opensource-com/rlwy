use crate::api::{Deployment, Railway};
use crate::config;
use crate::ui;
use anyhow::Result;
use colored::Colorize;
use tabled::settings::object::{Columns, Rows};
use tabled::settings::{Alignment, Modify, Style};
use tabled::{Table, Tabled};

const MESSAGE_MAX: usize = 56;

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "PROJECT")]
    project: String,
    #[tabled(rename = "SERVICE")]
    service: String,
    #[tabled(rename = "STATUS")]
    status: String,
    #[tabled(rename = "COMMIT")]
    commit: String,
    #[tabled(rename = "MESSAGE")]
    message: String,
}

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

    let mut rows: Vec<Row> = Vec::new();
    let mut last_project: Option<String> = None;
    for project in &projects {
        let services = project.services();
        if services.is_empty() {
            rows.push(Row {
                project: project_label(&project.name, &mut last_project),
                service: "(no services)".dimmed().to_string(),
                status: em_dash(),
                commit: em_dash(),
                message: em_dash(),
            });
            continue;
        }

        for svc in services {
            let (status, commit, message) = match svc.latest_deployment() {
                Some(d) => (
                    ui::color_status(d.status).to_string(),
                    commit_cell(d),
                    message_cell(d),
                ),
                None => (
                    "NO DEPLOYMENTS".dimmed().to_string(),
                    em_dash(),
                    em_dash(),
                ),
            };
            rows.push(Row {
                project: project_label(&project.name, &mut last_project),
                service: svc.name.clone(),
                status,
                commit,
                message,
            });
        }
    }

    let mut table = Table::new(rows);
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Alignment::center()))
        .with(Modify::new(Columns::single(0)).with(Alignment::left()))
        .with(Modify::new(Columns::single(3)).with(Alignment::left()));
    println!("{table}");

    println!();
    println!(
        "{}",
        "tip: `rlwy watch <service-id>` to follow a deployment".dimmed()
    );
    Ok(())
}

fn project_label(name: &str, last: &mut Option<String>) -> String {
    if last.as_deref() == Some(name) {
        String::new()
    } else {
        *last = Some(name.to_string());
        name.bold().to_string()
    }
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
        Some(m) if !m.trim().is_empty() => truncate(m.lines().next().unwrap_or("").trim()),
        _ => em_dash(),
    }
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= MESSAGE_MAX {
        return s.to_string();
    }
    let head: String = s.chars().take(MESSAGE_MAX - 1).collect();
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
                project: "my-app".bold().to_string(),
                service: "api".into(),
                status: "SUCCESS".green().to_string(),
                commit: "a1b2c3d".cyan().to_string(),
                message: "fix: retry on 502".into(),
            },
            Row {
                project: String::new(),
                service: "worker".into(),
                status: "BUILDING".cyan().to_string(),
                commit: "4e5f6a7".cyan().to_string(),
                message: "feat: add queue metrics".into(),
            },
        ];
        let mut table = Table::new(rows);
        table.with(Style::rounded());
        let out = table.to_string();
        assert!(out.contains("PROJECT"));
        assert!(out.contains("api"));
        assert!(out.contains("worker"));
        assert!(out.contains("fix: retry on 502"));
        eprintln!("{out}");
    }
}
