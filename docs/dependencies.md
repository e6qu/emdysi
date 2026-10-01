# Dependencies and license verification

Policy (decision D2): a crate may be added only after its license has been
verified from the primary source, meaning the `license` field in the
published crate's `Cargo.toml` **and** the LICENSE file(s) in the published
crate or its repository at the pinned version. Transitive dependencies count.

Allowed SPDX licenses: `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`,
`BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Zlib`, `Unicode-3.0`,
`Unicode-DFS-2016`, `Unlicense`, `CC0-1.0`. Anything else (including MPL,
LGPL and GPL) requires a new decision.

`deny.toml` enforces the allowlist in CI via `cargo-deny`.

## Verified dependencies

| Crate | Version | License | Verified from | Date |
|---|---|---|---|---|
| fancy-regex | 0.19.2 | MIT | crate `Cargo.toml` + `LICENSE` (Copyright 2015 The Fancy Regex Authors) | 2026-10-01 |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-01 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0; bundled Unicode tables under the Unicode License (`src/unicode_tables/LICENSE-UNICODE`) | crate `Cargo.toml` + license files | 2026-10-01 |
| bit-set | 0.8.0 | Apache-2.0 OR MIT | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-01 |
| bit-vec | 0.8.0 | Apache-2.0 OR MIT | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-01 |
| aho-corasick | 1.1.5 | Unlicense OR MIT | crate `Cargo.toml` + `LICENSE-MIT`, `UNLICENSE` | 2026-10-01 |
| memchr | 2.8.3 | Unlicense OR MIT | crate `Cargo.toml` + `LICENSE-MIT`, `UNLICENSE` | 2026-10-01 |

## Data

Bundled data is listed in [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).
Test-only data is in [`corpora/`](../corpora/README.md).
