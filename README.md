# emdysi

An MIT-licensed English text analyzer written in Rust. It aims to:

- parse English into phrase (constituency) and sentence structure,
- check spelling and apply safe automatic fixes,
- run semi-deterministic style and substance checks, especially on
  AI-generated prose, and enforce configurable style guides.

Status: pre-code. See [`docs/prior-art.md`](docs/prior-art.md) for the survey
of existing tools and data, their licenses, and the proposed architecture.

## License

MIT. See [`LICENSE`](LICENSE). Bundled third-party data will carry its own
notices in `THIRD_PARTY_NOTICES` once added.
