# Getting started

Chikachika `0.0.1` is a development build run from the source repository. It
does not currently provide a packaged macOS application or Linux installer.

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
on macOS; no packaged application is part of `0.0.1`.

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
The workspace shows the active server state and the configured port in the
**Local server settings** panel. A Browser Source URL becomes available only
after the server is ready and an overlay is selected and registered.

## Port settings and restart behavior

The **Local server settings** panel accepts a port from `1` through `65535`.
Click **Save port for next launch** after entering a new value. The running
server keeps its current port; the new value is used only after restarting
Chikachika. Update any OBS Browser Source URL after that restart by copying the
new exact URL from the workspace.

The server is loopback-only in `0.0.1`: it is available on the same computer,
not as a LAN or internet service. If the configured port is already occupied,
Chikachika reports the error and does not silently choose another port. See
[Troubleshooting](troubleshooting.md#the-server-is-not-ready-or-the-url-is-unavailable)
for recovery steps.

## Where Chikachika saves data

Chikachika keeps overlay documents and application settings in separate files.
The exact resolved locations are also shown by the application where relevant.

| Platform | Overlay documents | Server settings |
| --- | --- | --- |
| Linux | `$XDG_DATA_HOME/chikachika/overlays.json` when `XDG_DATA_HOME` is an absolute path; otherwise `$HOME/.local/share/chikachika/overlays.json` | `$XDG_CONFIG_HOME/chikachika/settings.json` when `XDG_CONFIG_HOME` is an absolute path; otherwise `$HOME/.config/chikachika/settings.json` |
| macOS | `$HOME/Library/Application Support/Chikachika/overlays.json` | `$HOME/Library/Application Support/Chikachika/settings.json` |

`overlays.json` is the complete versioned overlay snapshot. `settings.json`
stores the loopback server port separately. Chikachika creates the parent
directories when needed and does not use the repository or current working
directory as a fallback.

Back up these files before manually repairing them. A malformed or unsupported
overlay file blocks workspace startup without replacing the source file. A
malformed or unsupported settings file prevents the server from starting
without replacing the settings source. After repairing a file, restart
Chikachika.

Next: [Create and edit an overlay](overlay-workflow.md).
