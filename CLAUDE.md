# Project rules

1. **Commits**: very short, conventional commit format (`feat:`, `fix:`, `chore:`, etc.). Never add `Co-Authored-By` trailers or any AI attribution. Use plain `git commit -m "type: short message"` — no HEREDOC.
2. **Pre-commit check**: always run the build before committing to make sure nothing is broken.
3. **README**: update `README.md` (and package READMEs) whenever a change affects user-facing features, commands, or install steps.
4. **Language**: all code, comments, commit messages, and git-related text must be in English.
5. **Root cause**: when fixing a problem, investigate and fix the root cause. Don't wander into unrelated refactors or touch code outside the scope of the issue.
6. **Local readiness**: every new feature must be immediately runnable locally via the globally installed commands. After committing a feature, run `npm run dev:link` (first time) or `npm run dev:refresh` (subsequent) so `rlwy`/`rlwycli`/`railwaycli`/`railwycli` on the user's PATH reflect the latest code.
7. **Changelog**: every commit that changes user-visible behavior must also add an entry to `CHANGELOG.md` under the `## [Unreleased]` section. Follow the [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format: group under **Added**, **Changed**, **Deprecated**, **Removed**, **Fixed**, or **Security**. Write entries a developer actually upgrading the CLI can understand — describe the observable change, not the implementation. Skip this only for internal-only commits (CI, refactors with no behavior change, typo fixes). When cutting a release, move the `Unreleased` entries under a new `## [x.y.z] - YYYY-MM-DD` heading.
