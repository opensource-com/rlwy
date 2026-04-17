use crate::api::{Deployment, Project, Railway, Service};
use crate::config;
use crate::ui;
use anyhow::{Result, bail};
use colored::{ColoredString, Colorize};
use tabled::settings::Style;
use tabled::{Table, Tabled};

const MESSAGE_MAX: usize = 56;
const AUTHOR_MAX: usize = 18;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Web,
    Postgres,
    Redis,
    Mysql,
    Mongo,
    Clickhouse,
    Memcached,
    Image,
    Data,
}

impl Kind {
    fn label(self) -> ColoredString {
        match self {
            Self::Web => "web".cyan(),
            Self::Postgres => "postgres".blue().bold(),
            Self::Redis => "redis".red().bold(),
            Self::Mysql => "mysql".yellow().bold(),
            Self::Mongo => "mongo".green().bold(),
            Self::Clickhouse => "clickhouse".magenta().bold(),
            Self::Memcached => "memcached".bright_magenta().bold(),
            Self::Image => "image".dimmed(),
            Self::Data => "data".dimmed(),
        }
    }

    fn is_web(self) -> bool {
        matches!(self, Self::Web)
    }

    fn sort_key(self) -> u8 {
        match self {
            Self::Web => 0,
            Self::Postgres => 1,
            Self::Redis => 2,
            Self::Mysql => 3,
            Self::Mongo => 4,
            Self::Clickhouse => 5,
            Self::Memcached => 6,
            Self::Image => 7,
            Self::Data => 8,
        }
    }
}

#[derive(Tabled)]
struct Row {
    #[tabled(rename = "SERVICE")]
    service: String,
    #[tabled(rename = "TYPE")]
    kind: String,
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

        let mut rows: Vec<(Kind, Row)> =
            services.iter().map(|svc| build_row(svc)).collect();
        rows.sort_by_key(|(k, _)| k.sort_key());
        let rows: Vec<Row> = rows.into_iter().map(|(_, r)| r).collect();

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

fn build_row(svc: &Service) -> (Kind, Row) {
    let d = svc.latest_deployment();
    let kind = classify(svc, d);

    let (status, commit, author, message) = match d {
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

    let service_cell = if kind.is_web() {
        svc.name.clone()
    } else {
        svc.name.dimmed().to_string()
    };

    (
        kind,
        Row {
            service: service_cell,
            kind: kind.label().to_string(),
            status,
            commit,
            author,
            message,
        },
    )
}

fn classify(svc: &Service, d: Option<&Deployment>) -> Kind {
    if let Some(d) = d {
        if d.commit_hash().map(|h| !h.is_empty()).unwrap_or(false) {
            return Kind::Web;
        }
        if let Some(img) = d.image() {
            return image_to_kind(img);
        }
    }
    name_to_kind(&svc.name)
}

fn image_to_kind(raw: &str) -> Kind {
    let s = raw.to_ascii_lowercase();
    if s.contains("postgres") || s.contains("postgis") {
        Kind::Postgres
    } else if s.contains("redis") || s.contains("dragonfly") || s.contains("keydb") {
        Kind::Redis
    } else if s.contains("mysql") || s.contains("mariadb") {
        Kind::Mysql
    } else if s.contains("mongo") {
        Kind::Mongo
    } else if s.contains("clickhouse") {
        Kind::Clickhouse
    } else if s.contains("memcached") {
        Kind::Memcached
    } else {
        Kind::Image
    }
}

fn name_to_kind(name: &str) -> Kind {
    let s = name.to_ascii_lowercase();
    if s.contains("postgres") || s.contains("postgis") {
        Kind::Postgres
    } else if s.contains("redis") {
        Kind::Redis
    } else if s.contains("mysql") || s.contains("mariadb") {
        Kind::Mysql
    } else if s.contains("mongo") {
        Kind::Mongo
    } else if s.contains("clickhouse") {
        Kind::Clickhouse
    } else if s.contains("memcached") {
        Kind::Memcached
    } else {
        Kind::Data
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
    fn name_classifies_common_databases() {
        assert_eq!(name_to_kind("Postgres"), Kind::Postgres);
        assert_eq!(name_to_kind("Primary DB Postgres"), Kind::Postgres);
        assert_eq!(name_to_kind("Redis-4QR1"), Kind::Redis);
        assert_eq!(name_to_kind("MySQL"), Kind::Mysql);
        assert_eq!(name_to_kind("mariadb-prod"), Kind::Mysql);
        assert_eq!(name_to_kind("MongoDB"), Kind::Mongo);
        assert_eq!(name_to_kind("tokens"), Kind::Data);
    }

    #[test]
    fn image_classifies_common_databases() {
        assert_eq!(image_to_kind("postgres:15"), Kind::Postgres);
        assert_eq!(
            image_to_kind("ghcr.io/railwayapp-templates/postgres-ssl:latest"),
            Kind::Postgres
        );
        assert_eq!(image_to_kind("bitnami/redis:7"), Kind::Redis);
        assert_eq!(image_to_kind("dragonflydb/dragonfly"), Kind::Redis);
        assert_eq!(image_to_kind("nginx:alpine"), Kind::Image);
    }

    #[test]
    fn renders_table_with_sample_rows() {
        let rows = vec![
            Row {
                service: "api".into(),
                kind: Kind::Web.label().to_string(),
                status: "SUCCESS".green().to_string(),
                commit: "a1b2c3d".cyan().to_string(),
                author: "Alice".into(),
                message: "fix: retry on 502".into(),
            },
            Row {
                service: "Postgres".dimmed().to_string(),
                kind: Kind::Postgres.label().to_string(),
                status: "SUCCESS".green().to_string(),
                commit: em_dash(),
                author: em_dash(),
                message: em_dash(),
            },
            Row {
                service: "Redis-4QR1".dimmed().to_string(),
                kind: Kind::Redis.label().to_string(),
                status: "SUCCESS".green().to_string(),
                commit: em_dash(),
                author: em_dash(),
                message: em_dash(),
            },
        ];
        let mut table = Table::new(rows);
        table.with(Style::rounded());
        let out = table.to_string();
        assert!(out.contains("SERVICE"));
        assert!(out.contains("TYPE"));
        assert!(out.contains("api"));
        assert!(out.contains("Postgres"));
        assert!(out.contains("Redis"));
        eprintln!("{out}");
    }
}
