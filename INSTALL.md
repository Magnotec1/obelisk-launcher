<div align="center">
  <img src="data/com.magnotec.obelisk.svg" width="96" height="96" alt="Obelisk Icon">

  # Obelisk Launcher &mdash; Installation & Setup Guide

  <p>Instructions for building, running, and installing Obelisk Launcher either natively or as a sandboxed Flatpak package.</p>

  <p>
    <a href="README.md"><strong>« Back to README</strong></a> &bull;
    <a href="docs/SCREENSHOTS.md"><strong>Screenshots Gallery</strong></a> &bull;
    <a href="https://github.com/Magnotec1/obelisk-launcher/issues"><strong>Report Issue</strong></a>
  </p>
</div>

---

## Table of Contents
- [Prerequisites & System Libraries](#prerequisites--system-libraries)
  - [Debian / Ubuntu / Linux Mint](#debian--ubuntu--linux-mint)
  - [Fedora / RHEL](#fedora--rhel)
  - [Arch Linux / Manjaro](#arch-linux--manjaro)
- [Runtime Requirements](#runtime-requirements)
- [Method 1: Native Build with Cargo (Development)](#method-1-native-build-with-cargo-development)
- [Method 2: Sandboxed Flatpak (Recommended)](#method-2-sandboxed-flatpak-recommended)
  - [Flatpak Prerequisites](#flatpak-prerequisites)
  - [Installing GNOME SDK and Platform Runtimes](#installing-gnome-sdk-and-platform-runtimes)
  - [Building and Installing the Flatpak](#building-and-installing-the-flatpak)
  - [Running the Flatpak Application](#running-the-flatpak-application)
  - [Creating an Offline Flatpak Bundle](#creating-an-offline-flatpak-bundle)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites & System Libraries

To build Obelisk Launcher natively, your host system must have the development header files for GTK4, Libadwaita, OpenSSL, and standard compilation tools.

### Debian / Ubuntu / Linux Mint
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libgtk-4-dev libadwaita-1-dev unzip tar
```

### Fedora / RHEL
```bash
sudo dnf groupinstall "Development Tools"
sudo dnf install pkgconf-pkg-config openssl-devel gtk4-devel libadwaita-devel unzip tar
```

### Arch Linux / Manjaro
```bash
sudo pacman -Syu base-devel pkgconf openssl gtk4 libadwaita unzip tar
```

---

## Runtime Requirements

To launch Minecraft and manage runtime environments:
- **tar** and **unzip**: Required to extract downloaded Java runtimes and Minecraft game archives.
- **Java Runtime Environment (JRE)**: Minecraft requires a compatible JRE (Java 8 for legacy releases, Java 17 for 1.18–1.20.4, Java 21+ for modern releases). Obelisk includes an automated Java Installer in **Settings** to download and manage isolated runtimes effortlessly.

---

## Method 1: Native Build with Cargo (Development)

Ensure you have a recent stable Rust toolchain installed (via [rustup](https://rustup.rs/)):

```bash
# Clone the repository
git clone https://github.com/Magnotec1/obelisk-launcher.git
cd obelisk-launcher

# Run in debug mode
cargo run

# Build optimized release binary
cargo build --release

# Run release binary
cargo run --release
```

The compiled binary will be located at `target/release/obelisk`.

---

## Method 2: Sandboxed Flatpak (Recommended)

Flatpak isolates Obelisk from your host system and bundles all required GNOME 50 platform libraries and dependencies.

### Flatpak Prerequisites

Ensure `flatpak` and `flatpak-builder` are installed:

#### Ubuntu / Debian
```bash
sudo apt install flatpak flatpak-builder
```

#### Fedora
```bash
sudo dnf install flatpak flatpak-builder
```

#### Arch Linux
```bash
sudo pacman -S flatpak flatpak-builder
```

Enable the Flathub remote repository if not already configured:
```bash
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

### Installing GNOME SDK and Platform Runtimes

Obelisk builds against the GNOME 50 platform. Install the platform, SDK, and stable Rust extension from Flathub:

```bash
flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50 org.freedesktop.Sdk.Extension.rust-stable//24.08
```

### Building and Installing the Flatpak

To build the launcher and install it directly into your user environment:

```bash
flatpak-builder --user --install --force-clean build-dir flatpak/com.magnotec.obelisk.yaml
```

Once installed, Obelisk will appear in your desktop environment's application menu.

### Running the Flatpak Application

Run the installed Flatpak via terminal:
```bash
flatpak run com.magnotec.obelisk
```

Alternatively, to run directly from the build directory without installing:
```bash
flatpak-builder --run build-dir flatpak/com.magnotec.obelisk.yaml obelisk
```

### Creating an Offline Flatpak Bundle

To export the built application into a single `.flatpak` bundle file that can be distributed and installed on any Flatpak-enabled machine offline:

```bash
flatpak-builder --bundle build-dir flatpak/com.magnotec.obelisk.yaml com.magnotec.obelisk.flatpak
```

Install the bundle on any system using:
```bash
flatpak install com.magnotec.obelisk.flatpak
```

---

## Troubleshooting

- **Missing Libadwaita**: If you see errors about `libadwaita-1 not found`, verify that `libadwaita-1-dev` (or `libadwaita-devel`) is installed and matches your distribution's GNOME version.
- **Java Extraction Issues**: Ensure both `tar` and `unzip` are in your `$PATH`.
- **Sandbox Permissions**: If playing in a non-standard directory under Flatpak, verify permissions using [Flatseal](https://flathub.org/apps/com.github.tchx84.Flatseal) or add filesystem overrides:
  ```bash
  flatpak override --user --filesystem=/path/to/custom/dir com.magnotec.obelisk
  ```
