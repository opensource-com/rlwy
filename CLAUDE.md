# Project rules

1. **Commits**: very short, conventional commit format (`feat:`, `fix:`, `chore:`, etc.). Never add `Co-Authored-By` trailers or any AI attribution. Use plain `git commit -m "type: short message"` — no HEREDOC.
2. **Pre-commit check**: always run the build before committing to make sure nothing is broken.
3. **README**: update `README.md` (and package READMEs) whenever a change affects user-facing features, commands, or install steps.
4. **Language**: all code, comments, commit messages, and git-related text must be in English.
5. **Root cause**: when fixing a problem, investigate and fix the root cause. Don't wander into unrelated refactors or touch code outside the scope of the issue.
