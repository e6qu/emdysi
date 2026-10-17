# GitLab documentation Vale rules

`gitlab_base/`: the Vale rules GitLab runs on its own documentation
(`doc/.vale/gitlab_base/*.yml`): word choice, substitutions, Latin terms,
inclusive language, future tense, and checks of Markdown form.

- Upstream: https://gitlab.com/gitlab-org/gitlab, commit
  1d58eb23e25fffbf37bacc83ca32aa9f99a4c793, `doc/.vale/gitlab_base/`
  (the `.yml` files; `spelling-exceptions.txt` left out), and the
  repository's `LICENSE`.
- License: Creative Commons Attribution-ShareAlike 4.0 International. The
  repository's `LICENSE`: "All content residing under the "doc/" directory
  of this repository is licensed under "Creative Commons: CC BY-SA 4.0
  license"." Copyright GitLab Inc. The license text is
  `LICENSE-CC-BY-SA-4.0.txt` (from the UD English EWT distribution). It
  combines with emdysi's AGPL-3.0-or-later (decision D15).
- Unchanged.
- Use: `scripts/import-rules.py` converts the `existence` and
  `substitution` rules into `packs/gitlab.toml` (opt-in).
