<h1 align="center">Ocelot</h1>

<p align="center">
A minimalist process supervisor and init system written in the <a href="https://www.rust-lang.org/" target="_blank">Rust Programming Language</a>. It is specifically designed to act as a lightweight PID 1 process in containerized environments, ensuring that zombie processes are reaped and system signals are handled gracefully.
</p>

<p align="center">
    <a href="https://github.com/xrelkd/ocelot/releases"><img src="https://img.shields.io/github/v/release/xrelkd/ocelot.svg"></a>
    <a href="https://deps.rs/repo/github/xrelkd/ocelot"><img src="https://deps.rs/repo/github/xrelkd/ocelot/status.svg"></a>
    <a href="https://github.com/xrelkd/ocelot/actions?query=workflow%3ARust"><img src="https://github.com/xrelkd/ocelot/workflows/Rust/badge.svg"></a>
    <a href="https://github.com/xrelkd/ocelot/actions?query=workflow%3ARelease"><img src="https://github.com/xrelkd/ocelot/workflows/Release/badge.svg"></a>
    <a href="https://github.com/xrelkd/ocelot/blob/main/LICENSE"><img alt="GitHub License" src="https://img.shields.io/github/license/xrelkd/ocelot"></a>
</p>

---

## 🛠 Usage

### Command Line Interface

```text
Process supervisor and init system written in Rust Programming Language

Usage: ocelot [COMMAND]

Commands:
version      Print the version information
completions  Output shell completion code for the specified shell (bash, zsh, fish)
noop         Run as a minimalist PID 1 to reap zombies and hold namespaces
help         Print this message or the help of the given subcommand(s)

Options:
-h, --help     Print help
-V, --version  Print version
```

### The `noop` Command (Kubernetes Pause Equivalent)

The `noop` command is the core functionality for container init responsibilities. It is designed to be a direct replacement for the Kubernetes pause process, serving as the "infra" container or parent process that:

1. Holds Namespaces: Keeps the network/IPC namespaces alive by waiting indefinitely.
2. Reaps Zombies: Acts as `PID 1` to listen for `SIGCHLD` and reap orphaned processes.
3. Graceful Shutdown: Properly handles `SIGINT` or `SIGTERM` to allow the pod to terminate cleanly.

---

## 🚀 Installation

### From Source

To build and install Ocelot from source, ensure you have the Rust toolchain installed:

```bash
git clone https://github.com/xrelkd/ocelot.git
cd ocelot
cargo install --path .
```

### Shell Completions

Generate autocompletion scripts for your favorite shell:

```bash

# For Zsh

ocelot completions zsh > /usr/local/share/zsh/site-functions/_ocelot

# For Bash

ocelot completions bash > /etc/bash_completion.d/ocelot
```

---

## 🐳 Running in Docker

Using Ocelot as your `ENTRYPOINT` ensures that your container correctly manages the process lifecycle.

```dockerfile

# Use ocelot as the init system in your Dockerfile

COPY --from=ocelot /usr/bin/ocelot /usr/bin/ocelot

# Run with 'noop' to handle PID 1 duties

ENTRYPOINT ["ocelot", "noop"]
```

---

## License

Ocelot is licensed under the GNU General Public License version 3. See [LICENSE](./LICENSE) for more information.

---
