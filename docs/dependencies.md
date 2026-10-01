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
| (none yet: the workspace has no external dependencies) | | | | |

## Data

Bundled data is listed in [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).
Test-only data is in [`corpora/`](../corpora/README.md).
