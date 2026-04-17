# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each release section describes the user-visible impact: what a developer using
the CLI will actually notice after upgrading. Group entries under **Added**,
**Changed**, **Deprecated**, **Removed**, **Fixed**, or **Security**.

## [Unreleased]

## [0.4.0] - 2026-04-18

### Added
- `rlwy completions <SHELL>`: prints a bash/zsh/fish/powershell/elvish
  completion script to stdout (clap_complete generates from the actual
  argument tree, so every subcommand/flag/env/pick option completes).
  Example: `rlwy completions zsh > ~/.zsh/completions/_rlwy`.
- `rlwy deployments [QUERY]`: list the last N deployments of a service
  (status, created, short sha, author, message, id). `--limit N` to
  change how many, `--env <name>` to scope to a specific env. Queries
  Railway's top-level `deployments(input: DeploymentListInput)` with
  `projectId + serviceId + environmentId` filters.
- `rlwy rollback [QUERY]`: rolls a service back to an earlier deployment
  via Railway's `deploymentRollback` mutation. Defaults to the most
  recent earlier SUCCESS deployment; `--to <id|sha-prefix>` picks one
  explicitly; when multiple candidates exist, a fuzzy picker shows
  sha+commit-message. Tails the resulting deployment just like
  `rlwy redeploy`, and inherits the same `--no-watch`, `--env`, `--pick`
  flags.
- `rlwy status [PROJECT]`: one-line health summary (`✓ 27/30 services up`)
  plus a list of any FAILED/CRASHED services, suitable for shell prompts
  or CI. Exits non-zero when anything is broken. Optional project-name
  filter, and `--all` also lists services that are building/deploying/queued.
  Uses the single `projects()` call we already cache — no extra API hit.
- `rlwy env get NAME [QUERY]`: prints a single variable's raw value to
  stdout — designed for shell scripting
  (`export DB_URL=$(rlwy env get DATABASE_URL frontend --env staging)`).
  Exits non-zero with a clear error if the variable doesn't exist on the
  resolved service/env. Same resolution flags as the rest
  (`--pick`, `--env`, last-service memory).
- `rlwy env ls [QUERY]`: read-only listing of a service's environment
  variables, rendered as a sorted NAME / VALUE table (values truncated
  to 100 chars / first line). `--env <name>` targets a specific env;
  defaults to the latest deployment's env. `--keys-only` prints just
  the names one per line for piping (`rlwy env ls api --keys-only | grep SECRET`).
  Uses Railway's `variables(projectId, environmentId, serviceId)` query
  which returns the full set visible to the service (own + shared).
- `rlwy open [QUERY]`: opens the Railway dashboard for a service in your
  default browser (uses the `open` crate, works on Linux/macOS/Windows).
  Same resolution rules as `watch`/`logs`/`redeploy`; `--env <name>`
  adds `environmentId` to the URL so the right environment is selected.
  Override the base URL via `RLWY_DASHBOARD_URL` if you run a self-hosted
  Railway instance.

## [0.3.0] - 2026-04-18

### Changed
- `rlwy logs` now accepts a service name / id / `project/service` query
  (same syntax as `rlwy watch`) and fetches the latest deployment's logs
  for that service. Passing a bare deployment UUID still works — if the
  UUID matches a known service we use its latest deployment, otherwise we
  treat it as a raw deployment id. Running `rlwy logs` with no args
  resumes the last-picked service; `--pick` forces the picker. Before,
  `rlwy logs <service-uuid>` failed with "Deployment not found" because
  the argument was taken literally as a deployment id.

### Added
- Environment awareness: each project's environments (production,
  staging, …) are now fetched alongside its services. `rlwy ls` shows
  them in the project header (e.g. `envs: production, staging`). The
  `rlwy watch`, `rlwy logs`, and `rlwy redeploy` output shows which
  environment the active deployment belongs to (e.g.
  `watching service <id> [production]`), so you can tell at a glance
  whether you're tailing prod or staging.
- `--env <name>` flag on `rlwy watch`, `rlwy logs`, and `rlwy redeploy`
  targets a specific environment by name (case-insensitive). Under the
  hood, the `deployments(first: 1)` selection is filtered with
  `input: { environmentId }`, so the "latest deployment" is guaranteed
  to be the latest in that env, not the latest across all envs. Errors
  out with a helpful list of available environment names when the given
  name doesn't exist in the target project.
- `rlwy redeploy [QUERY]`: re-triggers a service's latest deployment
  (Railway's `deploymentRedeploy` mutation) and tails the resulting new
  deployment until it reaches a terminal status, just like `rlwy watch`.
  Same resolution rules as `watch`/`logs`: accepts a service name, id,
  or `project/service`; uses the last-picked service with no args;
  `--pick` forces the picker. Pass `--no-watch` to trigger and exit
  immediately instead of tailing.
- `rlwy logs -f` / `--follow`: after printing the initial batch, polls
  Railway's `deploymentLogs` every `--interval` seconds (default 2) and
  streams new lines until ctrl-c. Deduplicates against the last ~128
  lines to avoid repeat prints at timestamp boundaries.
- `rlwy logs --since <duration>`: filters both build + deploy logs to
  entries after `now - duration`. Accepts `30s`, `15m`, `2h`, `7d`, etc.
  Passes `startDate` through to Railway's GraphQL query so the server
  paginates for us. Railway's log retention (typically ~3 days on Free
  and ~7 days on Pro plans) limits how far back you can go.
- `rlwy logs --grep <text>`: case-insensitive substring filter applied
  to each log message both for the initial batch and during follow.

## [0.2.0] - 2026-04-18

### Changed
- `rlwy ls` now renders each project as its own rounded table under a
  project header (name + id), with SERVICE / STATUS / COMMIT / MESSAGE
  columns. Each row shows the short commit hash and first line of the
  commit message for the latest deployment, pulled from Railway's
  deployment metadata. Per-project tables make service-to-project
  grouping unambiguous, especially across many projects.
- `rlwy ls` accepts an optional project-name filter
  (`rlwy ls uft` shows only projects whose name contains "uft",
  case-insensitive). Errors out if no project matches.
- `rlwy ls` table now includes an AUTHOR column showing who made the
  commit for the latest deployment (pulled from Railway's deployment
  metadata, truncated to 18 chars).
- `rlwy ls` now classifies each service by type: `web`, `postgres`,
  `redis`, `mysql`, `mongo`, `clickhouse`, `memcached`, `image`, or
  `data`. The TYPE is inferred from the deployment image (if present in
  Railway's deployment meta) or falls back to a service-name pattern
  match — so Railway-template databases like "Postgres", "Redis-4QR1",
  "Primary DB Mongo" all get tagged correctly even though they carry no
  commits. Each type gets a distinct color (postgres=blue,
  redis=red, mysql=yellow, mongo=green, …). Rows are sorted web first,
  then by type, so actionable services stand out.
- `rlwy watch` picker is now a fuzzy search — type any part of a service
  or project name to narrow the list, arrow keys + Enter to pick.
- `rlwy watch` accepts a query arg: a service id (UUID), a service name
  (`rlwy watch frontend`), or `project/service` (`rlwy watch uft/frontend`).
  Unique matches skip the picker entirely; ambiguous matches show a
  narrowed picker.
- `rlwy watch` with no args resumes the last-picked service. Pass `--pick`
  to force the picker and override the remembered choice. The last choice
  is stored in the existing `config.json`.

### Added
- `rlwy upgrade` command: checks the latest GitHub release, downloads the
  binary for your platform, and atomically replaces the running one. Prints
  the release notes of the new version on success. Refuses to run against a
  local cargo build (detects the `target/release` path and suggests
  `npm run dev:refresh` instead).
- `npm run dev:link` workflow: builds the Rust binary, symlinks it into the
  npm wrapper, and runs `npm link` so the globally installed `rlwy`,
  `rlwycli`, `railwaycli`, and `railwycli` commands always reflect your
  latest local build. Run `npm run dev:refresh` after code changes to pick
  up new builds without re-linking.

## [0.1.2] - 2026-04-17

### Added
- `rlwycli`, `railwaycli`, and `railwycli` command aliases — all four names
  now launch the same binary, so you can type whichever you remember.

## [0.1.1] - 2026-04-17

### Changed
- Renamed the npm package to `railwaycli` (published from `packages/rlwy`).
- Fixed repository URLs in package metadata.

### Added
- GitHub Actions release workflow that builds binaries for 5 targets
  (Linux x64/arm64, macOS x64/arm64, Windows x64), uploads them to the
  GitHub release, and publishes the npm wrapper.
- `install.sh` and `install.ps1` standalone installers for users without
  npm.

## [0.1.0] - 2026-04-17

### Added
- Initial release. Rust CLI (`apps/cli`) with `login`, `whoami`, `ls`,
  `watch`, and `logs` commands for Railway deployments. npm wrapper
  (`packages/rlwy`) downloads the matching platform binary on install.
