# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Each release section describes the user-visible impact: what a developer using
the CLI will actually notice after upgrading. Group entries under **Added**,
**Changed**, **Deprecated**, **Removed**, **Fixed**, or **Security**.

## [Unreleased]

### Changed
- `rlwy ls` now renders projects and services as a single rounded table
  with PROJECT / SERVICE / STATUS / COMMIT / MESSAGE columns. Each row
  shows the short commit hash and first line of the commit message for the
  latest deployment, pulled from Railway's deployment metadata.

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
