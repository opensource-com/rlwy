use anyhow::{Context, Result, bail};
use colored::Colorize;
use serde::Deserialize;
use std::env;
use std::fs;
use std::io::Write;

const RELEASE_API: &str = "https://api.github.com/repos/opensource-com/rlwy/releases/latest";
const DOWNLOAD_BASE: &str = "https://github.com/opensource-com/rlwy/releases/download";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
}

pub async fn run() -> Result<()> {
    let current = env!("CARGO_PKG_VERSION");
    println!("{} current: {}", "→".dimmed(), current);

    let client = reqwest::Client::builder()
        .user_agent(concat!("rlwy/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("building HTTP client")?;

    let release: Release = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .context("checking latest release on GitHub")?
        .error_for_status()
        .context("GitHub API returned an error")?
        .json()
        .await
        .context("parsing GitHub release response")?;

    let latest = release.tag_name.trim_start_matches('v').to_string();
    println!("{} latest:  {}", "→".dimmed(), latest);

    match compare_semver(&latest, current) {
        std::cmp::Ordering::Equal => {
            println!("{} already up to date", "✓".green().bold());
            return Ok(());
        }
        std::cmp::Ordering::Less => {
            println!(
                "{} local build ({}) is newer than the latest release ({}); nothing to do",
                "!".yellow().bold(),
                current,
                latest
            );
            return Ok(());
        }
        std::cmp::Ordering::Greater => {}
    }

    let exe = env::current_exe().context("locating current executable")?;
    if is_dev_build(&exe) {
        bail!(
            "binary at {} looks like a local cargo build. Run `npm run dev:refresh` instead of `rlwy upgrade`.",
            exe.display()
        );
    }

    let target = detect_target()?;
    let ext = if cfg!(windows) { ".exe" } else { "" };
    let asset = format!("rlwy-v{}-{}{}", latest, target, ext);
    let url = format!("{}/v{}/{}", DOWNLOAD_BASE, latest, asset);

    println!("{} downloading {}", "→".dimmed(), asset);
    let bytes = client
        .get(&url)
        .send()
        .await
        .with_context(|| format!("downloading {url}"))?
        .error_for_status()
        .with_context(|| format!("download failed for {url}"))?
        .bytes()
        .await
        .context("reading download body")?;

    let dir = exe.parent().context("binary has no parent directory")?;
    let tmp = dir.join(format!("rlwy.upgrade-{}", std::process::id()));
    {
        let mut f = fs::File::create(&tmp)
            .with_context(|| format!("writing to {}", tmp.display()))?;
        f.write_all(&bytes)
            .with_context(|| format!("writing binary to {}", tmp.display()))?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
            .context("marking new binary executable")?;
    }

    #[cfg(windows)]
    {
        let old = dir.join("rlwy.exe.old");
        let _ = fs::remove_file(&old);
        fs::rename(&exe, &old).context(
            "could not stage the running binary — try closing other rlwy processes",
        )?;
    }

    fs::rename(&tmp, &exe).with_context(|| {
        format!(
            "replacing {} failed — the install dir may need elevated permissions",
            exe.display()
        )
    })?;

    println!("{} installed to {}", "→".dimmed(), exe.display());
    println!(
        "{} upgraded {} → {}",
        "✓".green().bold(),
        current,
        latest
    );

    if let Some(body) = release.body.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        println!();
        println!("{}", "What's new:".bold());
        for line in body.lines() {
            println!("  {line}");
        }
    }

    Ok(())
}

fn is_dev_build(exe: &std::path::Path) -> bool {
    let p = exe.to_string_lossy();
    let release_seg = if cfg!(windows) {
        "\\target\\release\\"
    } else {
        "/target/release/"
    };
    let debug_seg = if cfg!(windows) {
        "\\target\\debug\\"
    } else {
        "/target/debug/"
    };
    p.contains(release_seg) || p.contains(debug_seg)
}

fn detect_target() -> Result<&'static str> {
    Ok(match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        (os, arch) => bail!("no prebuilt binary for {os}/{arch}"),
    })
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    fn parse(v: &str) -> Vec<u64> {
        v.split('.')
            .map(|p| p.split('-').next().unwrap_or(p))
            .filter_map(|p| p.parse().ok())
            .collect()
    }
    parse(a).cmp(&parse(b))
}
