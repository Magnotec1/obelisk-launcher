pub mod backend;
pub mod config;
pub mod frontend;

use crate::config::Config;
use crate::frontend::app::AppModel;
use relm4::prelude::*;

fn main() {
    let config = Config::load();
    let app = RelmApp::new("com.magnotec.obelisk");

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
            padding: 4px 12px;
            font-size: 13px;
            font-weight: 500;
        }

        /* ── Overview Grid ────────────────────────────────────────────── */

        .overview-root {
            background-color: transparent;
        }

        .overview-back-bar {
            padding: 8px 4px;
        }

        .overview-back-btn {
            min-width: 36px;
            min-height: 36px;
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
            background-color: alpha(@card_bg_color, 0.5);
            border-radius: 12px;
            min-height: 80px;
            box-shadow: 0 2px 6px alpha(black, 0.08);
            transition: all 0.2s;
        }

        .overview-card:hover {
            background-color: @card_bg_color;
            box-shadow: 0 4px 12px alpha(black, 0.1);
        }

        /* Custom focus borders removed to match native Adwaita flowboxchild focus */
        .overview-grid-mode flowboxchild.menu-open > box {
            background-color: alpha(@card_bg_color, 0.85);
            box-shadow: 0 8px 24px alpha(black, 0.2);
        }

        .overview-card-info {
            padding: 16px;
            border-radius: 12px;
        }

        .overview-card-title {
            font-weight: 600;
            font-size: 14px;
        }

        .overview-card-mini-icon {
            opacity: 0.8;
            color: @accent_color;
        }

        .overview-card:hover .overview-card-mini-icon {
            opacity: 1.0;
        }

        .overview-card-subtitle {
            font-size: 12px;
            opacity: 0.55;
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

        .overview-list-mode flowboxchild:hover {
            background-color: alpha(currentColor, 0.07);
        }

        .overview-list-mode adwactionrow {
            padding: 8px 16px;
            background: transparent;
        }

        .overview-list-row {
            border-radius: 12px;
        }

        .overview-list-mode flowboxchild.menu-open {
            background-color: alpha(currentColor, 0.07); /* matches hover */
        }

        .overview-grid flowboxchild:selected {
            background-color: transparent;
        }

        .caption-heading {
            font-size: 0.75rem;
            font-weight: 700;
            color: alpha(currentColor, 0.8);
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
            border-radius: 6px;
        }
        .account-status-button:hover {
            background-color: alpha(currentColor, 0.08);
        }

        /* ── Icon Chooser Recents ─────────────────────────────────────── */
        .icon-chooser-recents flowboxchild {
            background: none;
            border: none;
            transition: none;
        }

        .icon-chooser-recent-btn {
            padding: 0;
            margin: 0;
            border-radius: 10px;
            background: none;
            border: none;
            box-shadow: none;
            transition: background-color 0.2s;
        }

        .icon-chooser-recent-btn:hover {
            background-color: alpha(currentColor, 0.1);
        }
    ",
    );

    app.run::<AppModel>(config);
}
