<div align="center">
  <img src="data/com.magnotec.obelisk.svg" width="96" height="96" alt="Obelisk Icon">

  # Installation Guide

  Instructions for building and running Obelisk Launcher either through Flatpak or natively with Cargo.

  <p>
    <a href="README.md">Back to README</a> &bull;
    <a href="docs/SCREENSHOTS.md">Screenshots</a> &bull;
    <a href="https://github.com/Magnotec1/obelisk-launcher/issues">Issues</a>
  </p>
</div>

---

## Contents
- [Native Build (Cargo)](#native-build-cargo)
  - [System Dependencies](#system-dependencies)
  - [Build and Run](#build-and-run)
- [Flatpak Build](#flatpak-build)
  - [Prerequisites](#flatpak-prerequisites)
  - [Runtimes](#installing-gnome-sdk-and-platform-runtimes)
  - [Building and Installing](#building-and-installing-the-flatpak)
  - [Running](#running-the-flatpak)
  - [Creating an Offline Bundle](#creating-an-offline-bundle)
- [Runtime Requirements](#runtime-requirements)
- [Troubleshooting](#troubleshooting)

---

## Native Build (Cargo)

### System Dependencies

Building natively requires development headers for GTK4, Libadwaita, OpenSSL, and standard build tools.

#### Debian / Ubuntu / Linux Mint
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libgtk-4-dev libadwaita-1-dev unzip tar
```

#### Fedora / RHEL
```bash
sudo dnf groupinstall "Development Tools"
sudo dnf install pkgconf-pkg-config openssl-devel gtk4-devel libadwaita-devel unzip tar
```

#### Arch Linux / Manjaro
```bash
sudo pacman -Syu base-devel pkgconf openssl gtk4 libadwaita unzip tar
```

### Build and Run

With a current stable Rust toolchain (via [rustup](https://rustup.rs/)):

```bash
git clone https://github.com/Magnotec1/obelisk-launcher.git
cd obelisk-launcher

# Run in debug mode
cargo run

# Or build release binary
cargo build --release
./target/release/obelisk
```

---

## Flatpak Build

Flatpak isolates the launcher and bundles the required GNOME 50 runtime libraries.

### Flatpak Prerequisites

Install `flatpak` and `flatpak-builder` if they are not already installed:

- **Ubuntu / Debian**: `sudo apt install flatpak flatpak-builder`
- **Fedora**: `sudo dnf install flatpak flatpak-builder`
- **Arch Linux**: `sudo pacman -S flatpak flatpak-builder`

Ensure the Flathub remote is configured:
```bash
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

### Installing GNOME SDK and Platform Runtimes

Obelisk targets the GNOME 50 platform runtime:

```bash
flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50 org.freedesktop.Sdk.Extension.rust-stable//24.08
```

### Building and Installing the Flatpak

Build and install directly into your user environment:

```bash
flatpak-builder --user --install --force-clean build-dir flatpak/com.magnotec.obelisk.yaml
```

Once installed, Obelisk will show up in your desktop environment's app launcher.

### Running the Flatpak

```bash
flatpak run com.magnotec.obelisk
```

Or run directly out of the build directory without installing to the system:
```bash
flatpak-builder --run build-dir flatpak/com.magnotec.obelisk.yaml obelisk
```

### Creating an Offline Bundle

To package a standalone `.flatpak` file for distribution:

```bash
flatpak-builder --bundle build-dir flatpak/com.magnotec.obelisk.yaml com.magnotec.obelisk.flatpak
```

Install it with:
```bash
flatpak install com.magnotec.obelisk.flatpak
```

---

## Runtime Requirements

- **tar** and **unzip**: Needed to unpack downloaded Java runtimes and Minecraft game files.
- **Java**: Java 8 for legacy Minecraft versions, Java 17 for 1.18 to 1.20.4, Java 21+ for modern releases. The built-in Java installer in Settings can download and manage these.

---

## Troubleshooting

- **Missing Libadwaita**: If compilation complains about `libadwaita-1 not found`, check that your distribution's development package (`libadwaita-1-dev` or `libadwaita-devel`) is installed and up to date.
- **Flatpak filesystem access**: If you save instances on another drive or non-standard path, grant the Flatpak permission using [Flatseal](https://flathub.org/apps/com.github.tchx84.Flatseal) or override via CLI:
  ```bash
  flatpak override --user --filesystem=/path/to/instances com.magnotec.obelisk
  ```
