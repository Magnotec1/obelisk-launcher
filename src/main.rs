pub mod backend;
pub mod config;
pub mod frontend;

use crate::config::Config;
use crate::frontend::app::AppModel;
use relm4::prelude::*;

fn main() {
    // ── Register custom icon GResource bundle ────────────────────────────
    // The binary blob is compiled into the executable at build time so no
    // external file access is needed at runtime.
    let resources = gtk::gio::Resource::from_data(&gtk::glib::Bytes::from_static(include_bytes!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/resources.gresource")
    )))
    .expect("Failed to load GResource bundle");
    gtk::gio::resources_register(&resources);

    let config = Config::load();
    let app = RelmApp::new("com.magnotec.obelisk");

    // ── Register icon search path so GTK finds our bundled SVGs ──────────
    // GTK must be initialised (i.e. RelmApp created) before touching the
    // default display / icon theme.
    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        icon_theme.add_resource_path("/com/magnotec/obelisk/icons");
    }

    relm4::set_global_css(
        "
        .console-view {
            background-color: #1e1e1e;
            color: #d4d4d4;
            border-radius: 12px;
            border: 1px solid rgba(255, 255, 255, 0.1);
        }
        .console-text {
            background-color: transparent;
            color: inherit;
            font-family: 'JetBrains Mono', 'Fira Code', monospace;
            font-size: 13px;
        }
        .console-box {
            padding: 16px;
            background-color: #141414;
        }
        .filter-bar {
            background-color: rgba(255, 255, 255, 0.03);
            border-radius: 8px;
            padding: 4px;
            margin: 0 8px 12px 8px;
        }
        .filter-btn {
            border-radius: 6px;
            padding: 4px 12px;
            font-size: 12px;
            font-weight: bold;
        }
        .dim-label {
            opacity: 0.5;
        }
        .menu-box {
            padding: 0px;
        }
        .menu-btn {
            border-radius: 8px;
            padding: 4px 10px;
            margin: 0px 0px;
            font-weight: normal;
            transition: none;
        }
        .menu-btn:hover {
            background-color: alpha(currentColor, 0.08);
        }
        .menu-separator {
            margin: 4px 0;
            background-color: alpha(currentColor, 0.1);
        }
        .settings-sidebar row {
            padding: 0px 0px;
        }
        .shortcut-label {
            font-size: 0.9em;
            opacity: 0.5;
            font-weight: normal;
        }
        .menu-shortcut {
            font-size: 0.8em;
            opacity: 0.4;
            font-weight: normal;
            margin-left: 12px;
        }
        .shortcut-badge {
            background-color: alpha(currentColor, 0.08);
            border-radius: 6px;
            padding: 2px 8px;
        }
        .status-bar-container {
            padding: 2px 4px;
        }
        .status-bar {
            border-radius: 8px;
            padding: 4px 12px;
        }
        .clickable-bar-container {
            padding: 2px 4px;
        }
        .clickable-bar {
            transition: background 0.2s;
            border-radius: 8px;
            padding: 4px 12px;
        }
        .clickable-bar:hover {
            background-color: alpha(currentColor, 0.08);
        }
        .pill-badge {
            background-color: alpha(currentColor, 0.08);
            border-radius: 999px;
            padding: 2px 8px;
            font-size: 13px;
            font-weight: 500;
        }

        .numeric {
            font-family: monospace;
            font-variant-numeric: tabular-nums;
        }

        .button-more-description {
            padding: 0px 16px;
        }

        /* ── Overview Grid ────────────────────────────────────────────── */

        .overview-root {
            background-color: transparent;
        }

        .overview-folder-header {
            min-height: 48px;
            background-color: transparent;
        }

        .header-separator {
            opacity: 1;
        }

        .overview-card-child {
            background: transparent;
            border: none;
            box-shadow: none;
            padding: 0;
            margin: 0;
            transition: none;
        }

        .overview-card {
            background-color: @card_bg_color;
            border: none;
            border-radius: 12px;
            transition: background-color 200ms ease;
        }

        .overview-card:hover {
            background-color: mix(@card_bg_color, @window_fg_color, 0.05);
        }

        .overview-card:active {
            background-color: mix(@card_bg_color, @accent_color, 0.15);
        }

        .overview-grid-mode flowboxchild:hover .overview-card {
            background-color: mix(@card_bg_color, @window_fg_color, 0.05);
        }

        .overview-grid-mode flowboxchild:selected .overview-card {
            background-color: mix(@card_bg_color, @accent_color, 0.15);
        }

        /* Prevent double backgrounds on Discover flowbox items */
        .overview-grid flowboxchild {
            background: transparent;
            border: none;
            box-shadow: none;
            padding: 0;
            margin: 0;
        }
        .overview-grid flowboxchild:hover {
            background: transparent;
            border: none;
            box-shadow: none;
        }
        .overview-grid flowboxchild:selected {
            background: transparent;
            border: none;
            box-shadow: none;
        }

        /* Featured Carousel Container & Items */
        .featured-carousel-container {
            background-color: @card_bg_color;
            border-radius: 12px;
            border: none;
            transition: background-color 300ms ease;
        }

        .featured-carousel-item {
            background-color: transparent;
            background-image: none;
            border: none;
            box-shadow: none;
            border-radius: 12px;
            border-radius: 0px;
            transition: background-color 200ms ease;
        }
        .featured-carousel-item:hover {
            background-color: rgba(255, 255, 255, 0.05);
            background-image: none;
            border: none;
            box-shadow: none;
        }
        .featured-carousel-item:active {
            background-color: rgba(255, 255, 255, 0.08);
            background-image: none;
            border: none;
            box-shadow: none;
        }

        /* Carousel Navigation Overlay Buttons */
        .carousel-nav-btn {
            background-color: transparent;
            color: @window_fg_color;
            border-radius: 999px;
            padding: 4px;
            margin: 0 12px;
            min-width: 28px;
            min-height: 28px;
            transition: background-color 200ms, opacity 200ms;
            opacity: 0.7;
        }
        .carousel-nav-btn:hover {
            background-color: alpha(@window_fg_color, 0.15);
            color: @window_fg_color;
            opacity: 1.0;
        }
        .carousel-nav-btn:active {
            background-color: alpha(@window_fg_color, 0.25);
            color: @window_fg_color;
            opacity: 1.0;
        }

        .overview-card-title {
            font-weight: 600;
            font-size: 14px;
            color: @window_fg_color;
        }

        .overview-card-subtitle {
            font-size: 12px;
            opacity: 0.55;
        }

        .overview-card-icon {
            border-radius: 8px;
        }

        .screenshot-img {
            border-radius: 8px;
        }

        .overview-card-stats {
            font-size: 11px;
            opacity: 0.45;
            margin-top: 2px;
        }

        /* ── Badges ───────────────────────────────────────────────────── */

        .overview-badge {
            font-size: 11px;
            font-weight: 600;
            padding: 2px 8px;
            border-radius: 999px;
            min-height: 18px;
        }

        .overview-version-badge {
            background-color: alpha(@accent_color, 0.15);
            color: @accent_color;
        }

        .overview-loader-fabric {
            background-color: alpha(#5CB3A5, 0.18);
            color: #5CB3A5;
        }

        .overview-loader-forge {
            background-color: alpha(#DBA154, 0.18);
            color: #DBA154;
        }

        .overview-loader-quilt {
            background-color: alpha(#A477C8, 0.18);
            color: #A477C8;
        }

        .overview-loader-neoforge {
            background-color: alpha(#E05A50, 0.18);
            color: #E05A50;
        }

        .overview-loader-generic {
            background-color: alpha(currentColor, 0.1);
        }

        /* ── List Mode Refinements ─────────────────────────────────────── */
        .overview-list-mode flowboxchild {
            padding: 0;
            margin: 0;
            border-radius: 12px;
        }

        .overview-list-card {
            background-color: @card_bg_color;
            border: none;
            border-radius: 12px;
            padding: 12px 16px;
            transition: background-color 200ms ease;
        }

        .overview-list-mode flowboxchild:hover .overview-list-card {
            background-color: mix(@card_bg_color, @window_fg_color, 0.05);
        }

        .overview-list-mode flowboxchild:selected .overview-list-card {
            background-color: mix(@card_bg_color, @accent_color, 0.15);
        }

        .overview-list-mode flowboxchild.menu-open .overview-list-card {
            background-color: mix(@card_bg_color, @window_fg_color, 0.05);
        }

        .overview-list-row {
            border-radius: 12px;
        }

        .overview-grid flowboxchild:selected {
            background-color: transparent;
        }

        /* ── Stat Chips ──────────────────────────────────────────────── */
        .overview-stat-chip {
            font-size: 11px;
            opacity: 0.6;
            padding: 1px 0;
        }

        .overview-stat-separator {
            opacity: 0.3;
            font-size: 10px;
        }

        .overview-stats-row {
            margin-top: 4px;
        }

        .caption-heading {
            font-size: 0.75rem;
            font-weight: normal;
            color: alpha(currentColor, 0.8);
        }

        .overview-folder-header {
            background-color: @window_bg_color;
            padding: 4px 8px;
        }

        .header-separator {
            background-color: alpha(@window_fg_color, 0.1);
        }

        .status-dot {
            border-radius: 999px;
        }
        .dot-green { background-color: #2ec27e; }
        .dot-red { background-color: #e01b24; }
        .dot-grey { background-color: #9a9996; }
        .dot-blue { background-color: #3584e4; }

        .account-status-button {
            padding: 2px 6px;
            border-radius: 999px;
        }
        .account-status-button:hover {
            background-color: alpha(currentColor, 0.08);
        }

        .playtime-button {
            padding: 2px 6px;
            border-radius: 999px;
        }
        .playtime-button:hover {
            background-color: alpha(currentColor, 0.08);
        }

        /* ── Monospace Terminal Log View ───────────────────────────────── */
        .terminal-log-view {
            font-family: 'JetBrains Mono', 'Fira Code', monospace;
            font-size: 11px;
            background-color: #121212;
            color: #d4d4d4;
            padding: 8px;
            border-radius: 6px;
        }

        /* ── Nautilus-style Floating OSD bubble ─────────────────────── */
        .floating-bar {
            background-color: @popover_bg_color;
            border-radius: 12px;
            padding: 4px 12px;
            margin: 2px;
        }
        .floating-bar label {
            font-size: 13px;
        }

        /* ── Selected Sidebar List Item Highlight ─────────────────────── */
        .navigation-sidebar row.selected {
            background-color: alpha(@accent_color, 0.15);
            color: @accent_color;
        }
        .download-progress-circle {
            color: @accent_bg_color;
            transform: scale(1.25);
        }

        .single-carousel-dot {
            background-color: @window_fg_color;
            border-radius: 999px;
            min-width: 6px;
            min-height: 6px;
            margin: 6px 0;
            opacity: 0.85;
        }
    ",
    );

    app.run::<AppModel>(config);
}
