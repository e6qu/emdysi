# emdysi for Visual Studio Code

Checks Markdown and plain text as you write with
[emdysi](https://github.com/e6qu/emdysi): spelling, grammatical errors the
grammar can prove, style, terminology, and the tells of machine-written
prose. Findings appear as squiggles and in the Problems panel; fixes and
suggestions are quick fixes. Everything runs on your machine: the extension
starts emdysi's command-line tool, `en`, as a language server (`en lsp`),
and nothing is sent anywhere.

- [What you need](#what-you-need)
- [Install `en`](#install-en)
- [Install the extension](#install-the-extension)
- [Use it](#use-it)
- [Settings](#settings)
- [Commands](#commands)
- [Speed and memory](#speed-and-memory)
- [Workspace Trust](#workspace-trust)
- [Troubleshooting](#troubleshooting)
- [Uninstall](#uninstall)
- [Build it yourself](#build-it-yourself)

## What you need

- VS Code 1.82 or later (or another editor built on it that installs
  `.vsix` files, such as VSCodium or Cursor).
- `en`, emdysi's command-line tool: the extension does not include it.
  Linux (x86-64), macOS (Apple silicon) and Windows (x86-64) builds are
  attached to each release; elsewhere, build it with Rust.
- About 0.6 GB of memory, and up to about 1.2 GB more for each sentence
  parsed at once (see [Speed and memory](#speed-and-memory)).

## Install `en`

**From a release.** Open the [releases](https://github.com/e6qu/emdysi/releases)
and download the archive for your system:

| System | Archive |
|---|---|
| Linux, x86-64 | `en-VERSION-x86_64-unknown-linux-gnu.tar.gz` |
| macOS, Apple silicon | `en-VERSION-aarch64-apple-darwin.tar.gz` |
| Windows, x86-64 | `en-VERSION-x86_64-pc-windows-msvc.tar.gz` |

Unpack it and put `en` (`en.exe` on Windows) on your `PATH`:

```sh
# Linux and macOS
tar xzf en-VERSION-TARGET.tar.gz
sudo install en-VERSION-TARGET/en /usr/local/bin/en      # or ~/.local/bin
```

```powershell
# Windows (PowerShell; tar ships with Windows 10 and later)
tar -xzf en-VERSION-x86_64-pc-windows-msvc.tar.gz
New-Item -ItemType Directory -Force "$env:LOCALAPPDATA\Programs\en"
Move-Item en-VERSION-x86_64-pc-windows-msvc\en.exe "$env:LOCALAPPDATA\Programs\en\"
# then add that folder to your user PATH (Settings > System > About >
# Advanced system settings > Environment Variables), or set emdysi.path
```

On macOS, a program downloaded with a browser is quarantined, and `en` is
not signed, so macOS may refuse to run it ("Apple could not verify...").
If it does, clear the quarantine once:

```sh
xattr -d com.apple.quarantine /usr/local/bin/en
```

**With Rust** (1.85 or later), on any system Rust supports:

```sh
cargo install --git https://github.com/e6qu/emdysi emdysi
```

This puts `en` in `~/.cargo/bin`, which rustup adds to the `PATH`.

**Check it.** In a new terminal:

```sh
echo "The new version are faster." | en check
```

should report the agreement error. The first run writes the grammar to
the cache directory and compiles it, which takes a few seconds; later runs
start in under a second.

## Install the extension

The extension is not on the Visual Studio Marketplace yet; it is attached
to each release as `emdysi.vsix`.

1. Download `emdysi.vsix` from the
   [releases](https://github.com/e6qu/emdysi/releases).
2. In VS Code, open the Extensions view (Ctrl+Shift+X, Cmd+Shift+X on
   macOS), click **...** (Views and More Actions) at its top, and choose
   **Install from VSIX...**; pick the file. Or, from a terminal:

   ```sh
   code --install-extension emdysi.vsix
   ```

3. Open a Markdown or plain-text file. If `en` is not on the `PATH` VS
   Code sees (on macOS, an app started from the Dock does not see your
   shell's `PATH`), set **emdysi.path** to its full path (see
   [Settings](#settings)).

To update, install the newer `emdysi.vsix` the same way; to update `en`,
replace the binary and run **emdysi: Restart Server**.

## Use it

**Which files.** Files in the *Markdown* or *Plain Text* language mode:
`.md` and `.txt` files, and new untitled files set to either. For other
prose files, pick one of the two in the language mode picker at the right
of the status bar (or **Change Language Mode** in the Command Palette).
Code blocks and inline code in Markdown are not checked as prose.

**When.** A document is checked when you open it, when you save it, and
when you stop typing for a moment (0.8 s). When a document is first
opened, the findings that need no parse (spelling, words and phrases)
appear at once, and the rest when all its sentences are parsed; the status
bar shows *emdysi: Checking NAME* until then. After that, only the
sentences you change are parsed again, so findings follow your edits
quickly.

**What you see.** Each finding is a squiggle, with its message on hover,
and an entry in the Problems panel (Ctrl+Shift+M, Cmd+Shift+M on macOS),
named by its rule (`core.spelling`, `plain-style.passive`, ...):

| emdysi | VS Code | Squiggle |
|---|---|---|
| error | Error | red |
| warning | Warning | yellow |
| suggestion | Information | blue |

Errors and warnings from the `core` pack (spelling, grammatical errors the
grammar can prove, a and an, repeated words) are claims about the text;
the style packs' suggestions are advice to weigh. Text in quotation marks
or in Markdown emphasis (`*...*`) is taken as an example or someone else's
words and only checked for errors.

**Fixing.** Put the cursor on a finding and press Ctrl+. (Cmd+. on macOS),
or click the light bulb: each fix or suggested replacement is a quick fix,
the one `en fix` would apply marked as preferred. The rules behind the
findings are described in
[rules.md](https://github.com/e6qu/emdysi/blob/main/docs/rules.md).

**Silencing a rule.** Add `--disable` and the rule id (or a prefix ending
in `*`, such as `ai-tells.*`) to **emdysi.args**; for a whole category of
findings, raise **emdysi.minSeverity**.

## Settings

Open them with **Preferences: Open Settings** and search for *emdysi*, or
add them to `settings.json` (user settings, or `.vscode/settings.json` for
one workspace). Changing one restarts the server.

| Setting | Default | What it does |
|---|---|---|
| `emdysi.path` | `en` | The `en` executable: a name on the `PATH` or a full path. |
| `emdysi.args` | `[]` | Options for `en lsp`, as `en check` takes them (`en --help` lists them all). Relative paths are read from the workspace folder. |
| `emdysi.minSeverity` | `suggestion` | Show only findings this severe or worse: `error`, `warning` or `suggestion` (everything). |

Some examples:

```jsonc
{
  // Where en is, if not on the PATH.
  "emdysi.path": "/opt/emdysi/en",                 // Windows: "C:\\Tools\\en\\en.exe"
  // Errors and warnings only.
  "emdysi.minSeverity": "warning",
  "emdysi.args": [
    // Packs beyond the defaults (core, ai-tells, plain-style, substance,
    // structure, terms): google, microsoft, elastic, languagetool, ...
    "--pack", "google",
    // A team's own pack, and its glossary (TOML, TBX or a Vale vocabulary).
    "--pack", "docs/style.toml",
    "--glossary", "docs/terms.toml",
    // Rules you do not want.
    "--disable", "plain-style.passive",
    "--disable", "ai-tells.*",
    // Sentences parsed at once (default: one per core, as memory allows).
    "--threads", "2"
  ]
}
```

Naming packs replaces the default set: `--pack google` alone checks with
the `google` pack only. To add one to the defaults, name them all
(`"--pack", "core", "--pack", "plain-style", ..., "--pack", "google"`).
How to write a pack or a glossary is in
[rules.md](https://github.com/e6qu/emdysi/blob/main/docs/rules.md).

## Commands

In the Command Palette (Ctrl+Shift+P, Cmd+Shift+P on macOS):

- **emdysi: Restart Server**: starts `en lsp` again, after `en` was
  updated or if it stopped.
- **emdysi: Show Output**: the server's messages and errors, in the
  Output panel.

## Speed and memory

Parsing takes from tens of milliseconds to a few seconds a sentence, about
1.3 s on average, so the first check of a long document takes a while
(emdysi's 1,500-word README: about a minute on four cores); the findings
that need no parse are there at once. Parses are cached on disk (in
`~/.cache/emdysi/en-parses`, or `%LOCALAPPDATA%\emdysi\en-parses` on
Windows, at most 256 MB; the `EMDYSI_CACHE_MB` environment variable changes
that), so reopening a document, or
checking it again after an edit, parses only the sentences that changed.

Each sentence parsed at once can take up to about 1.2 GB while it is
parsed. emdysi parses as many at once as you have cores, as far as the
memory available allows; `"--threads", "1"` in **emdysi.args** keeps it to
about 2 GB in all, at the cost of a slower first check.

## Workspace Trust

The extension runs the program **emdysi.path** names, with the options in
**emdysi.args**. A repository you clone could set these in its
`.vscode/settings.json` and have a program of its own run. So in a
workspace you have not trusted (VS Code's Restricted Mode), the workspace's
values of these two settings are ignored, and your user settings are used
instead. Trust the workspace to use its own.

## Troubleshooting

- **"emdysi: could not start 'en lsp'"**: `en` is not where
  **emdysi.path** says. Check `en --help` in a terminal; set the full path
  if VS Code does not see your shell's `PATH`.
- **No findings at all**: check that the file is in the Markdown or Plain
  Text language mode (status bar), and look at **emdysi: Show Output** for
  errors. A long document's grammatical findings come only when its
  sentences are parsed: wait for *emdysi: Checking* to leave the status
  bar.
- **macOS refuses to run `en`**: clear the quarantine (see
  [Install `en`](#install-en)).
- **Too many suggestions**: set **emdysi.minSeverity** to `warning`, or
  disable the rules you do not want (`--disable`).
- **Something else**: the Output panel (**emdysi: Show Output**) has the
  server's errors; `en check FILE` in a terminal shows the same findings.
  Report problems at
  [github.com/e6qu/emdysi/issues](https://github.com/e6qu/emdysi/issues).

## Uninstall

Uninstall the extension from the Extensions view (or `code
--uninstall-extension e6qu.emdysi`); delete `en` and its cache directory
(`~/.cache/emdysi`, or `%LOCALAPPDATA%\emdysi` on Windows) if you no longer
need them.

## Build it yourself

With Node.js 20 or later, in `editors/vscode` of the repository:

```sh
npm ci
npm run package        # writes emdysi.vsix
npm test               # downloads VS Code and checks a draft end to end
                       # (needs `en` built in target/release, or EMDYSI_EN;
                       # on Linux without a display: xvfb-run -a npm test)
```

## License

GNU Affero General Public License 3.0 or later, as emdysi. The bundled
language-client libraries are MIT or ISC
([dependencies.md](https://github.com/e6qu/emdysi/blob/main/docs/dependencies.md)).
