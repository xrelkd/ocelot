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
  idle         Run as a minimalist PID 1 to reap zombies and hold namespaces [aliases: noop, pause]
  entry        Spawns and supervises a child process as a minimalist PID 1 with signal forwarding and zombie reaping [aliases: wrap]
  zombie       Creates zombie processes by forking child processes that immediately exit, while the parent process sleeps. This is useful for testing how systems handle zombie processes.
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

### The `idle` Command (Kubernetes Pause Equivalent)

The `idle` command is the core functionality for container init responsibilities. It is designed to be a direct replacement for the Kubernetes pause process, serving as the "infra" container or parent process that:

- Holds Namespaces: Keeps the network/IPC namespaces alive by waiting indefinitely.
- Reaps Zombies: Acts as `PID 1` to listen for `SIGCHLD` and reap orphaned processes.
- Graceful Shutdown: Properly handles `SIGINT` or `SIGTERM` to allow the pod to terminate cleanly.

### The `entry` Command (Minimal Init & Supervisor)

The `entry` command provides a robust entry point for containerized workloads, serving as a minimal init system (PID 1). It is designed to manage the full lifecycle of a primary application while ensuring the container remains stable and responsive. Its key responsibilities include:

- Process Supervision: Spawns a child process via fork/exec and tracks its execution state, returning the correct Unix exit codes (including signal offsets).
- Signal Forwarding & Proxying: Intercepts SIGINT and SIGTERM from the container runtime and propagates them to the child process to facilitate graceful shutdowns.
- Zombie Reaping: Monitors SIGCHLD to proactively reap orphaned or "zombie" processes, preventing process table exhaustion within the PID namespace.
- Graceful Timeout Enforcement: Implements a configurable "kill-timer" that allows the child process a window to exit cleanly before forcibly terminating it with SIGKILL.

### The `zombie` Command

The `zombie` command is a specialized systems utility that illustrates a classic edge case in Unix process management.

**WARNING**: This command is intended for local testing and educational use. Generating an excessive number of zombie processes can exhaust the system's process ID (PID) limit, potentially preventing new processes from starting.

#### Core Behavior

Upon execution, the program enters a continuous loop where it utilizes the `fork()` system call to spawn new child processes. Each child process is programmed to terminate immediately. However, the parent process is explicitly designed to not call `wait()` or `waitpid()`.

#### The Resulting State

Under standard Unix semantics, when a child terminates, the kernel retains its exit status and process ID in the process table so the parent can eventually retrieve it. Because this parent process ignores these "death certificates," the children transition into a Zombie state (`Z`), appearing as `<defunct>` in system monitors like ps or top.

#### Signal Handling and Cleanup

The application is built to be "fire-and-forget":

- Signal Interruption: The parent process monitors for `SIGINT` (Ctrl+C) and `SIGTERM`.
- Instant Exit: Upon receiving these signals, the parent terminates immediately without attempting to clean up or "reap" its children.
- System Recovery: Once the parent process dies, the orphaned zombie processes are adopted by the system's init process (PID 1), which automatically reaps them, clearing them from the system process table.

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

# Run with 'idle' to handle PID 1 duties
ENTRYPOINT ["ocelot", "idle"]
```

---

## License

Ocelot is licensed under the GNU General Public License version 3. See [LICENSE](./LICENSE) for more information.
