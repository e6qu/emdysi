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
| pulldown-cmark | 0.13.4 (default features off) | MIT | crate `Cargo.toml` + `LICENSE` (Copyright 2015 Google Inc.) | 2026-10-01 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-01 |
| unicase | 2.9.0 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-01 |
| quick-xml | 0.41.0 (default features off; reads TBX glossaries) | MIT | crate `Cargo.toml` + `LICENSE-MIT.md` (Copyright (c) 2016 Johann Tuffe) | 2026-10-03 |

## The `http` feature (on by default)

`en rewrite --server URL` talks to a model served with the
OpenAI-compatible API: MLX's `mlx_lm.server` on Apple silicon, LM Studio,
Ollama, llama.cpp's `llama-server` and others. Pure Rust; plain HTTP only
(no TLS), meant for a server on the same machine.

| Crate | Version | License | Verified from | Date |
|---|---|---|---|---|
| base64 | 0.23.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| bytes | 1.12.1 | MIT | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| http | 1.5.0 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| httparse | 1.10.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| itoa | 1.0.18 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| log | 0.4.34 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| ureq | 3.4.2 (default features off: no TLS) | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| ureq-proto | 0.6.4 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT.txt`, `LICENSE-APACHE.txt` | 2026-10-02 |
| utf8-zero | 0.8.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-MIT`, `LICENSE-APACHE` | 2026-10-02 |
| zmij | 1.0.23 | MIT | crate `Cargo.toml` + `LICENSE-MIT` | 2026-10-02 |

## Optional: the `llama` feature

`en rewrite --model` needs a build with `--features llama`, which adds
`llama-cpp-2` and builds llama.cpp from source. Not part of the default
build. llama.cpp (`llama-cpp-sys-2`, `llama.cpp/LICENSE`) is MIT, Copyright
(c) 2023-2026 The ggml authors; the code it vendors is permissive:
cpp-httplib (MIT), nlohmann/json (MIT), sheredom/subprocess.h (Unlicense),
stb (MIT or public domain), miniaudio (public domain or MIT-0), and hash
functions: sha256 (public domain), rotate-bits (MIT), xxHash (BSD-2-Clause).
Model weights are never bundled; the user supplies a GGUF file.

| Crate | Version | License | Verified from | Date |
|---|---|---|---|---|
| bindgen | 0.72.1 | BSD-3-Clause | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| cc | 1.5.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| cexpr | 0.6.0 | Apache-2.0/MIT | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| clang-sys | 1.9.1 | Apache-2.0 | crate `Cargo.toml` + `LICENSE.txt` | 2026-10-02 |
| cmake | 0.1.58 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| either | 1.18.0 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| enumflags2 | 0.7.12 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| enumflags2_derive | 0.7.12 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| find_cuda_helper | 0.2.0 | MIT OR Apache-2.0 | crate `Cargo.toml` (no license file in the published crate; the repository's license files match) | 2026-10-02 |
| glob | 0.3.4 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| itertools | 0.13.0 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| jobserver | 0.1.35 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| libc | 0.2.189 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| libloading | 0.8.9 | ISC | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| llama-cpp-2 | 0.1.158 | MIT OR Apache-2.0 | crate `Cargo.toml` (no license file in the published crate; the repository's license files match) | 2026-10-02 |
| llama-cpp-sys-2 | 0.1.158 | MIT OR Apache-2.0 | crate `Cargo.toml` (no license file in the published crate; the repository's license files match) | 2026-10-02 |
| log | 0.4.34 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.md` | 2026-10-02 |
| nom | 7.1.3 | MIT | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| quote | 1.0.47 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| regex | 1.13.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| same-file | 1.0.6 | Unlicense/MIT | crate `Cargo.toml` + `COPYING`, `LICENSE-MIT`, `UNLICENSE` | 2026-10-02 |
| shlex | 1.3.0 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| shlex | 2.0.1 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| syn | 2.0.119 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| syn | 3.0.6 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT` | 2026-10-02 |
| tracing | 0.1.44 | MIT | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| tracing-attributes | 0.1.31 | MIT | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| tracing-core | 0.1.36 | MIT | crate `Cargo.toml` + `LICENSE` | 2026-10-02 |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 | crate `Cargo.toml` + `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-UNICODE` | 2026-10-02 |
| walkdir | 2.5.0 | Unlicense/MIT | crate `Cargo.toml` + `COPYING`, `LICENSE-MIT`, `UNLICENSE` | 2026-10-02 |

## Data

Bundled data is listed in [`THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).
Test-only data is in [`corpora/`](../corpora/README.md).
