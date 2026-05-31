# Installation and Requirements

This document outlines the required system libraries and steps to build and install Obelisk Launcher on Linux, either natively or as a sandboxed Flatpak package.

## Required System Libraries

To build Obelisk Launcher natively, your system must have development files for GTK4, Libadwaita, and typical build utilities.

### Debian / Ubuntu / Fedora / Arch Linux Packages

Install the following dependencies depending on your package manager:

#### Debian / Ubuntu / Mint
```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev libgtk-4-dev libadwaita-1-dev unzip tar
```

#### Fedora / RHEL
```bash
sudo dnf groupinstall "Development Tools"
sudo dnf install pkgconf-pkg-config openssl-devel gtk4-devel libadwaita-devel unzip tar
```

#### Arch Linux
```bash
sudo pacman -Syu base-devel pkgconf openssl gtk4 libadwaita unzip tar
```

### Runtime Requirements

For launching Minecraft and managing runtime environments:
- **unzip** and **tar**: Required for extracting downloaded Java runtimes.
- **Java Runtime Environment (JRE)**: Standard Java versions like 8, 17, or 21 are required to launch Minecraft. You can use the built-in Java Installer in settings to download and set these up.

---

## Building and Installing as Flatpak

Obelisk Launcher can be packaged and run inside a sandbox container using Flatpak. This isolates the application from your host environment and guarantees that all required libraries are bundled correctly.

### Flatpak Prerequisites

Ensure you have Flatpak and flatpak-builder installed on your system.

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

Add the Flathub repository if you haven't already:
```bash
flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
```

### Installing GNOME SDK and Runtimes

Obelisk Launcher builds against the GNOME 50 platform. Install the required platform and SDK runtimes from Flathub:
```bash
flatpak install flathub org.gnome.Platform//50 org.gnome.Sdk//50
```

### Building the Flatpak

To build the launcher and output the sandboxed application files to a build directory:
```bash
flatpak-builder --force-clean build-dir flatpak/com.magnotec.obelisk.yaml
```

### Running the Flatpak Application

To run the application directly from the built files for testing:
```bash
flatpak-builder --run build-dir flatpak/com.magnotec.obelisk.yaml obelisk
```

### Installing the Flatpak

To install the built Flatpak application to your user environment:
```bash
flatpak-builder --user --install --force-clean build-dir flatpak/com.magnotec.obelisk.yaml
```
Once installed, the application will appear in your desktop environment's application menu.

### Creating a Flatpak Bundle

To export the built application into a single, offline installable `.flatpak` bundle file:
```bash
flatpak-builder --bundle build-dir flatpak/com.magnotec.obelisk.yaml com.magnotec.obelisk.flatpak
```
The resulting `com.magnotec.obelisk.flatpak` file can be shared and installed on any Flatpak-enabled Linux system using:
```bash
flatpak install com.magnotec.obelisk.flatpak
```
