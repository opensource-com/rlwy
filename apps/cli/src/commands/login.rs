use crate::api::Railway;
use crate::config;
use anyhow::{Context, Result};
use colored::Colorize;

pub async fn run(token: Option<String>) -> Result<()> {
    let token = match token {
        Some(t) => t.trim().to_string(),
        None => {
            eprintln!(
                "{} Paste a Railway API token (create one at {})",
                "›".cyan().bold(),
                "https://railway.com/account/tokens".underline()
            );
            dialoguer::Password::new()
                .with_prompt("token")
                .interact()
                .context("reading token from prompt")?
        }
    };

    if token.is_empty() {
        anyhow::bail!("token is empty");
    }

    let api = Railway::new(token.clone())?;
    let who = identify(&api)
        .await
        .context("validating token against Railway")?;

    let mut cfg = config::load().unwrap_or_default();
    cfg.token = Some(token);
    let path = config::save(&cfg)?;

    println!(
        "{} logged in as {}  ({})",
        "✓".green().bold(),
        who.bold(),
        path.display().to_string().dimmed()
    );
    Ok(())
}

async fn identify(api: &Railway) -> anyhow::Result<String> {
    match api.me().await {
        Ok(me) => Ok(me
            .email
            .clone()
            .or_else(|| me.name.clone())
            .unwrap_or_else(|| me.id.clone())),
        Err(_) => {
            // team / project tokens can't query `me` — fall back to projects count
            let projects = api.projects().await?;
            Ok(format!("team token · {} projects visible", projects.len()))
        }
    }
}

pub async fn whoami() -> Result<()> {
    let token = config::require_token()?;
    let api = Railway::new(token)?;
    let who = identify(&api).await?;
    println!("{} {}", "▲".bright_magenta(), who.bold());
    Ok(())
}
