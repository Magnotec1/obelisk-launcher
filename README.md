> [!WARNING]
> To be transparent, development has been assisted by AI
> In development, expect the possibility of data loss

# Obelisk Launcher
A minecraft launcher built for the GNOME desktop. **Obelisk** focuses heavily on UI design and feature completeness, 
using the same format as Prism Launcher, MultiMC, and PolyMC.

### Architecture: 
- [Rust](https://www.rust-lang.org/)
- [GTK4](https://gtk.org)
- [Relm4](https://relm4.org)
- [Libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/)

## Features:
- Instance Management: Easily create, manage, and launch multiple Minecraft versions, with their own settings, mods, worlds, and more.
- Folders: Group instances together and display them neatly with multiple views.
- Asset Management: Display a detailed rundown of all storage usage from instances and minecraft assets, and clear space easily.
- Microsoft Authentication: Fully supported Microsoft authentication with the launcher.
- Multiple Accounts: Add multiple Microsoft/Offline accounts and switch between them easily.
- Playtime: Track playtime overall and per instance, persisting even through instance deletion.
- Modrinth Integration: Allows mod download/installation/version selection from inside the launcher, via [Modrinth](https://modrinth.com/).
- Java Management: Manage java versions and install new ones easily.
- Sharing: Easily share instances either via code or file with your friends!

## Installation and Requirements

Obelisk Launcher requires system packages for GTK4 and Libadwaita to run natively on your desktop. It can also be built and run as a sandboxed Flatpak package.

For detailed instructions on installing development libraries, configuring dependencies, and building or installing as a Flatpak, see the [Installation Guide](INSTALL.md).

