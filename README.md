# rlwy

Tiny CLI to watch Railway deployments from your terminal.
Rust core. Distributed via npm + direct installers.

## Install

```bash
# npm (Linux, macOS, Windows — needs Node/npm)
npm install -g rlwy

# Linux / macOS (no npm needed)
curl -fsSL https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.sh | bash

# Windows PowerShell (no npm needed)
irm https://raw.githubusercontent.com/rlwy-dev/rlwy/main/install.ps1 | iex
```

Pin a version with `RLWY_VERSION=v0.1.0` (bash) or `$env:RLWY_VERSION='v0.1.0'` (PowerShell).

## Usage

```bash
rlwy login           # paste a Railway token (https://railway.com/account/tokens)
rlwy ls              # list your projects + last deployment status
rlwy watch           # pick a service and tail the current deployment
```

## Commands

| Command                    | What it does                                               |
|----------------------------|------------------------------------------------------------|
| `rlwy login [--token T]`   | Save your Railway API token                                |
| `rlwy whoami`              | Show the account the current token belongs to              |
| `rlwy ls`                  | List projects, services, and latest deployment status      |
| `rlwy watch [SERVICE_ID]`  | Poll the active deployment and show progress + status      |
| `rlwy logs DEPLOYMENT_ID`  | Print build + deploy logs for a deployment                 |

## Dev

```bash
npm install
npx nx build cli            # cargo build --release
npx nx build rlwy           # compiles the npm wrapper
```

- `apps/cli`       — Rust binary (clap + reqwest + indicatif)
- `packages/rlwy`  — npm wrapper; `postinstall` downloads the matching binary
- `install.sh` / `install.ps1` — standalone installers that pull from GitHub Releases
- `.github/workflows/release.yml` — tag `vX.Y.Z` → builds 5 targets → publishes release + npm

## Release

```bash
git tag v0.1.0
git push --tags
```

## Token

Stored at `$XDG_CONFIG_HOME/rlwy/config.json` (or `~/.config/rlwy/config.json`).
