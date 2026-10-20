# Development

## Prerequisites

- Linux with an X11 session or XWayland
- Docker Engine and the Docker Compose plugin
- Rust 1.92 or newer
- A StatusNotifierItem-compatible panel for the tray icon

The app builds with the `glow` renderer and the X11 backend. It does not require
GTK, a web runtime, or Docker Desktop.

## Build and run

```bash
cargo run --release
```

For a debug build:

```bash
cargo run
```

The debug binary includes symbols and is considerably larger; use release builds
for normal use.

## Tests and checks

```bash
cargo fmt --check
cargo test
cargo check
cargo build --release
```

The tests are parser tests and do not start containers or require a Docker
daemon. The UI is tested manually against a local Docker Engine.

## Install locally

```bash
./install.sh
```

The script builds the release binary, installs it to `~/.local/bin`, and installs
the desktop entry and icon. It substitutes the absolute binary path into the
installed desktop entry, so `~/.local/bin` does not have to be in the session
PATH.

To uninstall:

```bash
rm -f ~/.local/bin/docker-deck
rm -f ~/.local/share/applications/docker-deck.desktop
rm -f ~/.local/share/icons/hicolor/scalable/apps/docker-deck.svg
update-desktop-database ~/.local/share/applications 2>/dev/null || true
gtk-update-icon-cache ~/.local/share/icons/hicolor 2>/dev/null || true
```

## Project layout

```text
.
├── assets/
│   ├── docker-deck.desktop
│   └── docker-deck.svg
├── docs/
│   ├── ARCHITECTURE.md
│   └── DEVELOPMENT.md
├── src/
│   ├── app.rs
│   ├── docker.rs
│   ├── icon.rs
│   ├── main.rs
│   ├── model.rs
│   ├── tray.rs
│   └── ui/
│       ├── components.rs
│       ├── compose.rs
│       ├── containers.rs
│       ├── images.rs
│       ├── mod.rs
│       ├── networks.rs
│       ├── system.rs
│       └── volumes.rs
├── Cargo.lock
├── Cargo.toml
├── install.sh
└── README.md
```

## Adding a resource view

1. Add a `Tab` variant and its label in `src/model.rs`.
2. Add a module under `src/ui/` and register it in `src/ui/mod.rs`.
3. Add the resource to `Snapshot` and to the refresh path in `src/docker.rs`.
4. Add any action and command variants to `DockerCommand`/`PendingAction`.
5. Render the view from `src/app.rs` and route its actions through the existing
   confirmation flow.
6. Add parser tests for new command output before touching the UI.

## Docker command guidelines

- Pass every argument as a separate `String`; never build a shell command.
- Return Docker's stderr on failure so the UI can show the real cause.
- Refresh the full snapshot after a mutating action.
- Keep destructive operations behind `ask`/`ConfirmRequest`.
- Use `docker inspect` only for data that the list commands do not expose.

## Release checklist

```bash
cargo fmt --check
cargo test
cargo build --release
./install.sh
desktop-file-validate ~/.local/share/applications/docker-deck.desktop
```

Then verify:

- Compose list and port links load.
- Compose stop/start works on a disposable project.
- Volume deletion is disabled for an in-use volume.
- The tray icon toggles the window and **Quit** exits.
- The installed desktop entry launches the app.

## Troubleshooting

### The window opens but the tray icon is missing

Install or start a StatusNotifierItem host. On Cinnamon this is normally
`xapp-sn-watcher`; without a host, `ksni` cannot register the icon.

### Docker commands fail

Verify the CLI first:

```bash
docker version
docker compose version
docker ps
```

The app forces `DOCKER_HOST=unix:///var/run/docker.sock` and
`DOCKER_CONTEXT=default`, so it does not use a remote Docker context.

### The desktop entry does not launch

`./install.sh` writes an absolute `Exec` path into the installed entry. Re-run
the script after moving the binary, or inspect the entry with:

```bash
desktop-file-validate ~/.local/share/applications/docker-deck.desktop
```
