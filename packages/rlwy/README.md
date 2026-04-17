# rlwy

Tiny terminal-first watcher for Railway deployments. Rust under the hood, delivered over npm.

```bash
npm install -g railwaycli

rlwy login
rlwy ls
rlwy watch
```

The package installs four interchangeable commands: `rlwy`, `rlwycli`, `railwaycli`, and `railwycli`.

On install, this package downloads the Rust binary for your platform from the
matching GitHub release. Set `RLWY_SKIP_INSTALL=1` to opt out.

Supported platforms:

| OS      | Arch    |
|---------|---------|
| Linux   | x64     |
| Linux   | arm64   |
| macOS   | x64     |
| macOS   | arm64   |
| Windows | x64     |

Source, issues, and release binaries: https://github.com/opensource-com/rlwy
