# Software terms (cspell dictionaries)

Words that are correct in technical writing but in no general English
word list: tool and project names written in lower case (*systemd*,
*etcd*, *rustc*), and technical vocabulary (*async*, *enum*, *cgroups*,
*composable*, *deallocating*). Used only to accept words as known in the
`spelling` rule kind (`emdysi-check`, `dict::accepted`); never as
suggestions.

- Upstream: https://github.com/streetsidesoftware/cspell-dicts, commit
  `a69283e74295a9fed0ca16648266f4decfd95573` (2026-10-04); retrieved
  2026-10-04.
- Files, copied verbatim from each dictionary's `src/`:
  - `software-terms.txt`, `software-tools.txt`, `coding-terms.txt` from
    `dictionaries/software-terms` (license `LICENSE-software-terms`);
  - `rust.txt` from `dictionaries/rust` (`LICENSE-rust`);
  - `k8s.txt` from `dictionaries/k8s` (`LICENSE-k8s`);
  - `fullstack.txt` from `dictionaries/fullstack` (`LICENSE-fullstack`).
- License: MIT, per each dictionary's own `LICENSE` file and its
  `package.json` (`"license": "MIT"`), copied here. The repository root
  is GPL-3.0; it covers the repository as a whole, while each dictionary
  package declares MIT, and only these MIT packages' word lists are
  copied.
- Copyright: Street Side Software (all four); the Rust dictionary also
  Alexander Andreev (see `LICENSE-rust`).
- Format: one word per line; `#` starts a comment. Entries with characters
  other than letters, digits, `'`, `-` and `.` are ignored when loading.
