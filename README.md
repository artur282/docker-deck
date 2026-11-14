# Docker Deck

A native Linux desktop app for managing Docker Engine, built with Rust and
[egui](https://github.com/emilk/egui). No Electron, no web UI, and no Docker
Desktop: Docker Deck talks to the local Docker CLI and parses its JSON output.

![Rust](https://img.shields.io/badge/rust-1.92%2B-orange)
![Platform](https://img.shields.io/badge/platform-Linux-blue)
![UI](https://img.shields.io/badge/UI-egui-8A2BE2)
![License](https://img.shields.io/badge/license-MIT-green)

The two workflows I use every day are the reason this app exists:

- **Compose**: see every project, start/stop/restart it, read logs, open its
  published ports, or tear it down with `down` and an optional `down -v`.
- **Volumes**: inspect sizes and in-use state, then delete free volumes
  individually or in bulk.

Containers, images, networks, and disk usage/prune actions are also available.

## Features

### Compose

- List all Compose projects with status and config files.
- `Up`, `Stop`, `Start`, `Restart`, `Logs`, and `Down`.
- `Down` asks for confirmation; removing named volumes requires checking an
  explicit box.
- Clickable `:port` links for published TCP ports, including ports exposed by
  containers running on the host network.
- Per-project search and a config-column summary with the full paths on hover.

### Volumes

- List named volumes with driver, size, and in-use link count.
- Delete free volumes one by one or in bulk, with confirmation.
- In-use volumes are marked and their delete control is disabled.
- Select-all-unused helper and search.

### More Docker resources

| View | Actions |
|---|---|
| Containers | Start, stop, restart, remove, logs |
| Images | Remove, prune dangling images |
| Networks | Remove unused, prune |
| System | Disk usage summary and prune actions |

Every destructive action requires an explicit confirmation.

## Requirements

- Linux with an X11 session or XWayland.
- Docker Engine reachable at `unix:///var/run/docker.sock`.
- Docker CLI with the Compose plugin (`docker compose`).
- Rust 1.92 or newer to build from source.
- A StatusNotifierItem-compatible panel for the tray icon (Cinnamon provides
  one through `xapp-sn-watcher`).

Docker Deck always uses the local Docker socket and the `default` Docker
context. Remote Docker hosts are intentionally out of scope.

## Install

```bash
git clone https://github.com/artur282/docker-deck.git
cd docker-deck
./install.sh
```

`install.sh` builds the release binary and installs:

- `~/.local/bin/docker-deck`
- `~/.local/share/applications/docker-deck.desktop`
- `~/.local/share/icons/hicolor/scalable/apps/docker-deck.svg`

The desktop entry appears as **Docker Deck**. The installer is user-local and
does not invoke `sudo`. Set `PREFIX` to install somewhere else:

```bash
PREFIX="$HOME/.local" ./install.sh
```

Use `--no-build` to install an existing `target/release/docker-deck`, and
`--uninstall` to remove the installed files:

```bash
./install.sh --uninstall
```

To run it without installing:

```bash
cargo run --release
```

## Usage

- **Compose** is the default view. Use the action buttons on each project row.
  Open a `:port` link to launch `http://127.0.0.1:<port>` in the browser.
- **Volumes** shows size and in-use state. Select free volumes and use the bulk
  delete action, or delete one from its row.
- **X** hides the window to the tray. Only the right-side tray icon remains.
- **Left-click** the tray icon toggles the window open/closed.
- **Right-click** the tray icon shows **Show/Hide Docker Deck** and **Quit**.
- Closing the app definitively is always explicit: tray **Quit** or `Ctrl+Q`.

| Shortcut | Action |
|---|---|
| `Ctrl+R` | Refresh now |
| `Ctrl+T` | Toggle light/dark theme |
| `Ctrl+Q` | Quit |

The app starts in dark mode and refreshes automatically every 30 seconds. Auto
refresh pauses while a confirmation dialog is open.

## Safety

- Compose `down` removes containers and networks but keeps named volumes unless
  **Also remove named volumes** is checked.
- Volume deletion is permanent. The UI disables deletion for in-use volumes.
- Prune actions follow Docker's own unused-resource rules; volume prune can
  delete data.
- Docker commands are executed as argument vectors on a worker thread. No shell
  interpolation is used.

## Architecture

Docker Deck is split into a UI layer, a Docker worker, and a tray service:

- `src/docker.rs` runs one Docker command at a time and parses JSON.
- `src/model.rs` defines snapshots, actions, and events.
- `src/app.rs` owns navigation, refresh, confirmations, logs, theme, and the
  channel to the worker.
- `src/ui/` contains one module per resource view.
- `src/tray.rs` implements the StatusNotifierItem icon and its menu.
- `src/icon.rs` generates the window and tray icons.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full design and
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for build and contribution notes.

## Project layout

```text
docker-deck/
├── assets/                 # Desktop entry and SVG icon
├── docs/                   # Architecture and development guides
├── src/
│   ├── app.rs              # App state and UI orchestration
│   ├── docker.rs           # Docker CLI worker and JSON parsing
│   ├── icon.rs             # Generated window/tray icons
│   ├── main.rs             # eframe entry point
│   ├── model.rs            # Snapshot and action types
│   ├── tray.rs             # StatusNotifierItem tray
│   └── ui/                 # One view per Docker resource
├── Cargo.toml
├── install.sh
└── README.md
```

## Development

```bash
cargo fmt --check
cargo test
cargo build --release
```

The parser tests cover Compose labels, JSON-lines output, Compose JSON arrays,
published ports, host-network ports, and project mapping. Tests do not require
a running Docker daemon.

## Notes

- Tested on Linux Mint 22.3 (Cinnamon, X11) against a native Docker Engine.
- The app intentionally stays small: Docker CLI instead of a Docker SDK, and
  egui instead of a web runtime.

## Contributing

Issues and pull requests are welcome. Keep changes focused and include a test
for any parser or Docker-command change.

## License

MIT © 2026 Luis Cruz. See [LICENSE](LICENSE).
