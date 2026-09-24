# Obelisk Screenshots & UI Gallery

<div align="center">
  <img src="../data/com.magnotec.obelisk.svg" width="96" height="96" alt="Obelisk Icon">

  <p>Visual showcase of Obelisk Launcher's interface, built natively with GTK4 and Libadwaita.</p>

  <p>
    <a href="../README.md"><strong>« Back to README</strong></a> &bull;
    <a href="../INSTALL.md"><strong>Installation Guide</strong></a> &bull;
    <a href="https://github.com/Magnotec1/obelisk-launcher/issues"><strong>Report Issue</strong></a>
  </p>
</div>

---

## Table of Contents
- [Main Showcase](#main-showcase)
- [Library & Instance Grid](#library--instance-grid)
- [Instance Editor](#instance-editor)
- [Modrinth Browser](#modrinth-browser)
- [Playtime Analytics](#playtime-analytics)
- [Account Management](#account-management)
- [Settings & Java Management](#settings--java-management)
- [Download Manager](#download-manager)
- [First-Run Setup Experience](#first-run-setup-experience)
- [Adaptive & Compact Layout](#adaptive--compact-layout)

---

## Main Showcase

A multi-window composition highlighting Obelisk's desktop integration, Adwaita view switchers, playtime tracker, instance editor, and download manager.

<div align="center">
  <img src="screenshots/readme/screenshot-combined.png" alt="Obelisk Showcase" width="850">
</div>

---

## Library & Instance Grid

The heart of Obelisk. Organize your Minecraft instances in customizable folders, browse with high-resolution instance cards, and launch with a single click.

<div align="center">
  <img src="screenshots/gallery/library.png" alt="Obelisk Library View" width="850">
</div>

- **Folder Grouping**: Keep modpacks, vanilla testbeds, and custom servers separated in neat directories.
- **Instance Metadata**: See Minecraft version, mod loader (Fabric, Forge, NeoForge, Quilt), installed mod count, and total playtime at a glance.
- **Action Bar**: Fast instance creation, view layout toggles, refresh, and global menu.

---

## Instance Editor

Deep per-instance configuration without leaving the application.

<div align="center">
  <img src="screenshots/gallery/instanceeditor.png" alt="Instance Editor" width="850">
</div>

- **Component Management**: Inspect Minecraft version, mod loader version, and custom Java runtime overrides.
- **Content Lists**: Browse and toggle installed mods, resource packs, shader packs, and worlds.
- **Drag-and-Drop**: Easily drop downloaded `.jar` or `.zip` files straight into the editor to install.
- **Screenshots & Logs**: Access in-game screenshots and live console outputs directly.

---

## Modrinth Browser

Integrated Modrinth discovery platform for searching and installing mods, modpacks, resource packs, and shaders.

<div align="center">
  <img src="screenshots/gallery/modrinthbrowser.png" alt="Modrinth Browser" width="850">
</div>

- **Search & Filtering**: Search through tens of thousands of mods with automatic version compatibility checks.
- **Rich Details**: View galleries, descriptions, download counts, and changelogs.
- **One-Click Install**: Automatically fetch and link dependencies into your selected instance.

---

## Playtime Analytics

Comprehensive playtime tracking that persists across sessions and instances.

<div align="center">
  <img src="screenshots/gallery/playtime.png" alt="Playtime Analytics" width="850">
</div>

- **Global & Per-Instance Statistics**: See your all-time playtime and breakdowns by individual profile.
- **Recent Session History**: Detailed timeline of your latest gameplay sessions.
- **Persistent Metrics**: Playtime data remains intact even if an instance is deleted or modified.

---

## Account Management

Seamless profile switching between Microsoft accounts and local offline profiles.

<div align="center">
  <img src="screenshots/gallery/accounts.png" alt="Account Management" width="850">
</div>

- **Microsoft OAuth**: Official Microsoft login flow with personal Azure Client ID support.
- **Multi-Account**: Store multiple profiles and switch active identities with a single click.
- **Offline Profiles**: Easy testing and local play without external network dependencies.

---

## Settings & Java Management

Global launcher configuration, memory allocation, and automated Java runtime setup.

<div align="center">
  <img src="screenshots/gallery/settings.png" alt="Launcher Settings" width="850">
</div>

- **Memory Controls**: Tune minimum and maximum JVM heap allocation with intuitive steppers.
- **Managed Runtimes**: Automatically download, isolate, and maintain Java 8, 17, 21, or 25 environments.
- **System Detection**: Auto-detect existing system Java installations.

---

## Download Manager

Real-time task tracking for background downloads, asset verification, and extraction.

<div align="center">
  <img src="screenshots/gallery/downloadqueue.png" alt="Download Queue" width="850">
</div>

- **Active Transfers**: Monitor file progress, speeds, and queued assets.
- **Non-blocking Operations**: Downloads run completely in the background without freezing the UI.

---

## First-Run Setup Experience

A step-by-step onboarding wizard to get the launcher configured in seconds.

### Step 0: Welcome
<div align="center">
  <img src="screenshots/gallery/setup-step0-welcome.png" alt="Setup Step 0 - Welcome" width="850">
</div>

### Step 2: Azure Client ID Configuration
<div align="center">
  <img src="screenshots/gallery/setup-step2-clientid.png" alt="Setup Step 2 - Azure Client ID" width="850">
</div>

### Step 4: Java Runtime Installation
<div align="center">
  <img src="screenshots/gallery/setup-step4-javainstallation.png" alt="Setup Step 4 - Java Runtime Installation" width="850">
</div>

### Step 5: Setup Complete
<div align="center">
  <img src="screenshots/gallery/setup-step5-completed.png" alt="Setup Step 5 - Completed" width="850">
</div>

---

## Adaptive & Compact Layout

Obelisk is designed with Libadwaita responsive design principles, adapting fluidly to mobile screens or tiled half-screen windows.

<div align="center">
  <img src="screenshots/gallery/smallwidth.png" alt="Adaptive Narrow View" width="850">
</div>

- **Bottom Navigation**: Sidebar collapses gracefully into an adaptive bottom bar / slide-out drawer on narrow viewports.
- **Tiling Support**: Perfect for GNOME 50 tiling window managers and small handheld devices (e.g. Steam Deck).
