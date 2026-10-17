# Tidepool

**Tidepool** is a fast, lightweight, and developer-friendly CLI for managing local development environments. Whether you're spinning up a single database or orchestrating a dozen microservices, Tidepool has you covered.

## ✨ Features

- 🚀 **Blazing fast:** Environments start in seconds, not minutes.
- 🔒 **Secure by default:** Secrets never leave your machine.
- 🧩 **Extensible:** Plug in your own providers with a simple, intuitive API.
- 🌍 **Cross-platform:** Works seamlessly on macOS, Linux, and Windows.

## Why Tidepool?

Managing local environments shouldn't be painful. Yet for most teams, it is. Tidepool was built to change that — streamlining the entire workflow so you can focus on what really matters: writing great code.

## Getting Started

Getting up and running is simple. First, install Tidepool:

```sh
curl -fsSL https://example.invalid/install.sh | sh
```

Next, initialize a new environment:

```sh
tidepool init
```

That's it! You're ready to go. 🎉

## Configuration

Tidepool uses a single `tidepool.toml` file to define your environment. Here's a quick example to get you started:

```toml
[services.db]
image = "postgres:16"
```

> **Note:** Tidepool will automatically detect existing configuration files and offer to migrate them.

## Contributing

We welcome contributions of all kinds! Whether it's fixing a typo, reporting a bug, or proposing a new feature, your input helps make Tidepool better for everyone. Please see `CONTRIBUTING.md` for guidelines.

## License

MIT — see `LICENSE` for details.

---

Made with ❤️ by developers, for developers.
