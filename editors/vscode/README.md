# emdysi for Visual Studio Code

Checks Markdown and plain text as you write with
[emdysi](https://github.com/e6qu/emdysi): spelling, grammatical errors the
grammar can prove, style, and the tells of machine-written prose. Findings
are diagnostics; fixes and suggestions are quick fixes (Ctrl+. / Cmd+.).

The extension runs `en lsp`, so it needs `en` installed: download it from
the [releases](https://github.com/e6qu/emdysi/releases), or
`cargo install --git https://github.com/e6qu/emdysi emdysi`.

When a document is first opened, the findings that need no parse appear
at once and the rest once its sentences are parsed (about a second each
the first time; afterwards only the sentences you change are parsed).

## Settings

- `emdysi.path`: the `en` executable (default `en`, found on the PATH).
- `emdysi.args`: options for `en lsp`, such as `["--pack", "google"]` or
  `["--glossary", "terms.toml"]`.
- `emdysi.minSeverity`: `error`, `warning` or `suggestion` (the default,
  everything).

## Building

`npm ci && npm run package` writes `emdysi.vsix`; install it with
*Extensions: Install from VSIX...*.

## License

GNU Affero General Public License 3.0 or later, as emdysi.
