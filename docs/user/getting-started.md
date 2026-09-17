# Getting started

Chikachika is a development build run from the source repository. It does not
currently provide a packaged macOS application or Linux installer.

## Prerequisites

You need:

- A supported macOS or Linux desktop.
- Git, if you are checking out the repository.
- The stable Rust toolchain, including Cargo. Install it with
  [rustup](https://www.rust-lang.org/tools/install).

Node.js 22 or newer and Python 3 are needed only when running the complete
repository test and documentation checks; they are not required for ordinary
overlay authoring.

### Linux

On NixOS, enter the development environment from the repository root:

```sh
nix develop
```

The flake supplies the Rust, Node.js, Python, and native GUI/OpenSSL
development dependencies used by the project.

On Ubuntu or Debian outside the Nix environment, install the native build
libraries used by the development build:

```sh
sudo apt-get update
sudo apt-get install --no-install-recommends -y \
  libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
  libxkbcommon-dev libssl-dev libgtk-3-dev
```

Other Linux distributions need equivalent packages for the native GUI and
OpenSSL development libraries.

### macOS

Install the stable Rust toolchain and use a normal macOS desktop development
environment. The project’s continuous integration validates the source build
on macOS; no packaged application is documented yet.

## Download and launch

From a terminal, clone the repository and enter it:

```sh
git clone https://github.com/marcybelardo/chikachika.git
cd chikachika
```

Start the application:

```sh
cargo run
```

Chikachika starts its local server before opening the workspace. With missing
settings, the server uses the loopback address `127.0.0.1` and port `51737`.
The workspace shows the active server state and configured port. A Browser
Source URL becomes available only after the server is ready and an overlay is
selected and registered.

## Port settings and restart behavior

The **Local server settings** panel accepts a port from `1` through `65535`.
Click **Save port for next launch** after entering a new value. The running
server keeps its current port; the new value is used only after restarting
Chikachika. Update any OBS Browser Source URL after that restart by copying the
new exact URL from the workspace.

The server is loopback-only: it is available on the same computer, not as a LAN
or internet service. If the configured port is already occupied, Chikachika
reports the error and does not silently choose another port. See
[Troubleshooting](troubleshooting.md#the-server-is-not-ready-or-the-url-is-unavailable)
for recovery steps.

## Where Chikachika saves data

Chikachika keeps overlay documents and application settings in separate files.
The exact resolved locations are also shown by the application where relevant:
the settings panel surfaces the settings path, and blocked overlay startup
surfaces the overlay source path.

| Platform | Overlay documents | Server settings |
| --- | --- | --- |
| Linux | `$XDG_DATA_HOME/chikachika/overlays.json` when `XDG_DATA_HOME` is an absolute path; otherwise `$HOME/.local/share/chikachika/overlays.json` | `$XDG_CONFIG_HOME/chikachika/settings.json` when `XDG_CONFIG_HOME` is an absolute path; otherwise `$HOME/.config/chikachika/settings.json` |
| macOS | `$HOME/Library/Application Support/Chikachika/overlays.json` | `$HOME/Library/Application Support/Chikachika/settings.json` |

These locations come from `ProjectDirs::from("", "", "Chikachika")`: overlay
documents use `data_local_dir`, and settings use `config_local_dir`. The
application creates parent directories when needed and does not use the
repository or current working directory as a fallback.

`overlays.json` is the complete version-2 ordered overlay snapshot. It stores
widget identities and durable properties, not selection, runtime revisions, or
history. `settings.json` is a separate version-1 envelope containing the
validated loopback server port; saving it is for the next launch and does not
live-rebind the running server.

Back up files before manually repairing them. An existing format-1 overlay file
is not migrated. If an overlay source is malformed, unsupported, or otherwise
incompatible, first copy it to a separately named backup, then move the source
aside yourself and restart to create a fresh workspace. Chikachika never
silently converts, overwrites, deletes, or automatically moves user data. Use
the same user-managed backup discipline for malformed or unsupported settings.
See [Troubleshooting](troubleshooting.md#saved-overlays-do-not-appear) for
step-by-step recovery.

Next: [Create and edit an overlay](overlay-workflow.md).
