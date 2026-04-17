# rlwy

Tiny CLI to watch Railway deployments from your terminal.
Rust core. Distributed via npm + direct installers.

## Install

```bash
# npm (Linux, macOS, Windows — needs Node/npm)
npm install -g railwaycli

# Linux / macOS (no npm needed)
curl -fsSL https://raw.githubusercontent.com/opensource-com/rlwy/main/install.sh | bash

# Windows PowerShell (no npm needed)
irm https://raw.githubusercontent.com/opensource-com/rlwy/main/install.ps1 | iex
```

Pin a version with `RLWY_VERSION=v0.1.1` (bash) or `$env:RLWY_VERSION='v0.1.1'` (PowerShell).

## Usage

```bash
rlwy login           # paste a Railway token (https://railway.com/account/tokens)
rlwy ls              # list your projects + last deployment status
rlwy watch           # pick a service and tail the current deployment
```

Also available as `rlwycli`, `railwaycli`, and `railwycli` — same binary, pick whichever you remember.

## Commands

| Command                    | What it does                                               |
|----------------------------|------------------------------------------------------------|
| `rlwy login [--token T]`   | Save your Railway API token                                |
| `rlwy whoami`              | Show the account the current token belongs to              |
| `rlwy ls [QUERY]`          | Table of projects, services, status, and latest commit. QUERY filters by project-name substring (e.g. `rlwy ls uft`) |
| `rlwy watch [QUERY]`       | Poll the active deployment. QUERY is a service id, name, or `project/service`. Omit to resume the last choice; add `--pick` to force the picker. |
| `rlwy logs DEPLOYMENT_ID`  | Print build + deploy logs for a deployment                 |
| `rlwy upgrade`             | Fetch the latest GitHub release and replace your binary    |

## Dev

```bash
npm install
npx nx build cli            # cargo build --release
npx nx build rlwy           # compiles the npm wrapper
```

### Use your local build as the global `rlwy`

```bash
npm run dev:link            # build + symlink binary + npm link (run once)
npm run dev:refresh         # rebuild Rust after code changes (auto-updates the link)
npm run dev:unlink          # remove the global link
```

After `dev:link`, the commands `rlwy`, `rlwycli`, `railwaycli`, and `railwycli`
all point at `apps/cli/target/release/rlwy` via a symlink — so every subsequent
`cargo build --release` (or `npm run dev:refresh`) is picked up immediately.

- `apps/cli`       — Rust binary (clap + reqwest + indicatif)
- `packages/rlwy`  — npm wrapper; `postinstall` downloads the matching binary
- `install.sh` / `install.ps1` — standalone installers that pull from GitHub Releases
- `.github/workflows/release.yml` — tag `vX.Y.Z` → builds 5 targets → publishes release + npm

## Release

```bash
git tag v0.1.1
git push --tags
```

## Token

Stored at `$XDG_CONFIG_HOME/rlwy/config.json` (or `~/.config/rlwy/config.json`).
