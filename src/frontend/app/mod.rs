pub mod handlers;
pub mod msg;
pub mod state;

pub use msg::{AppMsg, InstanceStatus};
pub use state::AppModel;

use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::manager::{scan_instances, Instance};
use crate::backend::playtime::PlaytimeManager;
use crate::config::{Config, PreferredViewType};
use crate::frontend::dialogs::external::browser::{BrowserOutput, UnifiedBrowser};
use crate::frontend::dialogs::external::download::{
    DownloadDialog, DownloadDialogOutput, DownloadStatusBar, DownloadStatusBarOutput,
};
use crate::frontend::dialogs::instance::add::{AddInstanceDialog, AddInstanceOutput};
use crate::frontend::dialogs::instance::components::ComponentEditorDialog;
use crate::frontend::dialogs::instance::editor::InstanceEditorDialog;
use crate::frontend::dialogs::instance::mod_loader::ModLoaderDialog;
use crate::frontend::dialogs::instance::sharing::{
    ImportDialog, ImportOutput, InstanceSharerDialog, SharerOutput,
};
use crate::frontend::dialogs::system::java::{JavaSelectorDialog, JavaSelectorOutput};
use crate::frontend::dialogs::system::setup::{SetupDialog, SetupOutput};
use crate::frontend::dialogs::system::shortcuts::ShortcutsDialog;
use crate::frontend::views::account::AccountView;
use crate::frontend::views::assets::{AssetManagerView, AssetOutput};
use crate::frontend::views::discover::DiscoverView;
use crate::frontend::views::instance::{
    InstanceConsole, InstanceEditorTab, InstanceSettingsTab, InstanceSummary,
};
use crate::frontend::views::library::{LayoutMode, OverviewGrid};
use crate::frontend::views::playtime::PlaytimeView;
use crate::frontend::views::settings::{SettingsDialog, SettingsOutput};
use crate::frontend::views::sidebar::{SidebarList, SidebarPage};
use adw::prelude::*;
use gtk::glib;
use relm4::prelude::*;
use std::collections::HashMap;

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = Config;
    type Input = AppMsg;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("Obelisk Launcher"),
            set_default_width: 900,
            set_default_height: 600,
            set_width_request: 450,
            set_height_request: 400,

            #[wrap(Some)]
            #[name = "toast_overlay"]
            set_content = &adw::ToastOverlay {
                #[name = "main_content_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_vexpand: true,

                    adw::StatusPage {
                        #[watch]
                        set_visible: model.config.instances_path.is_none(),
                        set_title: "Workspace Setup Required",
                        set_description: Some("To start building, managing, and playing Minecraft instances, let's complete a brief setup walkthrough to configure your workspace directory, player account, and Java runtimes."),
                        set_icon_name: Some("applications-system-symbolic"),
                        set_vexpand: true,

                        gtk::Button {
                            set_label: "Start Setup Walkthrough",
                            set_halign: gtk::Align::Center,
                            set_css_classes: &["suggested-action", "pill"],
                            set_tooltip_text: Some("Open the launcher initial configuration walkthrough"),
                            connect_clicked => AppMsg::OpenSetup,
                        }
                    },

                    // ── Main layout: responsive overlay split view ───────
                    #[name = "split_view"]
                    adw::OverlaySplitView {
                        #[watch]
                        set_visible: model.config.instances_path.is_some(),
                        set_vexpand: true,
                        set_sidebar_width_fraction: 0.25,
                        set_min_sidebar_width: 180.0,
                        set_max_sidebar_width: 280.0,

                        // ── Sidebar ──────────────────────────────────────
                        #[wrap(Some)]
                        set_sidebar = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                #[wrap(Some)]
                                set_title_widget = &adw::WindowTitle {
                                    set_title: "Obelisk",
                                },
                                set_show_end_title_buttons: false,

                                pack_start = model.download_status_bar.widget(),

                                    pack_end = &gtk::MenuButton {
                                        set_icon_name: "open-menu-symbolic",
                                        set_tooltip_text: Some("Options"),
                                        #[wrap(Some)]
                                        set_popover: main_popover = &gtk::Popover {
                                            set_autohide: true,
                                            set_has_arrow: true,
                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_css_classes: &["menu-box"],
                                                set_width_request: 200,

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Label {
                                                            set_label: "Setup Walkthrough",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, main_popover] => move |_| {
                                                        main_popover.popdown();
                                                        sender.input(AppMsg::OpenSetup);
                                                    },
                                                },

                                                gtk::Separator {
                                                    set_margin_top: 4,
                                                    set_margin_bottom: 4,
                                                },

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Label {
                                                            set_label: "Preferences",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                        gtk::Label {
                                                            set_label: "Ctrl+,",
                                                            set_css_classes: &["dim-label"],
                                                        },
                                                    },
                                                    connect_clicked[sender, main_popover] => move |_| {
                                                        main_popover.popdown();
                                                        sender.input(AppMsg::OpenSettings);
                                                    },
                                                },

                                                gtk::Separator {
                                                    set_margin_top: 4,
                                                    set_margin_bottom: 4,
                                                },

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Label {
                                                            set_label: "Shortcuts",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                        gtk::Label {
                                                            set_label: "Ctrl+?",
                                                            set_css_classes: &["dim-label"],
                                                        },
                                                    },
                                                    connect_clicked[sender, main_popover] => move |_| {
                                                        main_popover.popdown();
                                                        sender.input(AppMsg::OpenShortcuts);
                                                    },
                                                },

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Label {
                                                            set_label: "About Obelisk",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, main_popover] => move |_| {
                                                        main_popover.popdown();
                                                        sender.input(AppMsg::OpenAbout);
                                                    },
                                                },
                                            },
                                        },
                                    },
                                },

                                #[wrap(Some)]
                                set_content = model.sidebar.widget(),
                            },

                        // ── Content Area ─────────────────────────────────
                        #[wrap(Some)]
                        set_content = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                    #[wrap(Some)]
                                    set_title_widget = &gtk::Stack {
                                        add_named[Some("normal")] = &gtk::Box {
                                            set_orientation: gtk::Orientation::Horizontal,
                                            set_spacing: 6,
                                            set_halign: gtk::Align::Center,

                                            #[name = "header_title"]
                                            adw::WindowTitle {
                                                #[watch]
                                                set_title: match model.active_sidebar_page {
                                                    SidebarPage::Library => if let Some(ref folder) = model.current_folder {
                                                        folder
                                                    } else {
                                                        "Library"
                                                    },
                                                    SidebarPage::Discover => if model.discover_details_open {
                                                        &model.discover_details_title
                                                    } else {
                                                        "Discover Modpacks"
                                                    },
                                                    SidebarPage::Accounts => "Account Management",
                                                    SidebarPage::Playtime => "Playtime Analytics",
                                                    SidebarPage::Assets => match &model.active_asset_subpage {
                                                        Some(sub) => sub.as_str(),
                                                        None => "Asset Manager",
                                                    },
                                                    SidebarPage::InstanceDetails => {
                                                        if let Some(idx) = model.selected_instance {
                                                            model.instances.get(idx).map(|i| i.name.as_str()).unwrap_or("Instance")
                                                        } else {
                                                            "Instance"
                                                        }
                                                    },
                                                },
                                                #[watch]
                                                set_subtitle: match model.active_sidebar_page {
                                                    SidebarPage::Library => if model.current_folder.is_some() {
                                                        "Group Folder"
                                                    } else {
                                                        ""
                                                    },
                                                    SidebarPage::Discover => if model.discover_details_open {
                                                        "Modrinth Modpack"
                                                    } else {
                                                        "Modrinth & External Catalog"
                                                    },
                                                    SidebarPage::Accounts => "Microsoft & Local Profiles",
                                                    SidebarPage::Playtime => "Gameplay Statistics & History",
                                                    SidebarPage::Assets => match &model.active_asset_subpage {
                                                        Some(_) => "Asset Manager",
                                                        None => "Shared & Local Content Storage",
                                                    },
                                                    SidebarPage::InstanceDetails => "",
                                                },
                                            },
                                        },

                                        #[name = "instance_switcher"]
                                        add_named[Some("switcher")] = &adw::ViewSwitcher {
                                            set_policy: adw::ViewSwitcherPolicy::Wide,
                                        },

                                        #[watch]
                                        set_visible_child_name: if model.active_sidebar_page == SidebarPage::InstanceDetails && !model.is_narrow && !model.split_view.is_collapsed() {
                                            "switcher"
                                        } else {
                                            "normal"
                                        },
                                    },

                                    #[name = "open_sidebar_btn"]
                                    pack_start = &gtk::ToggleButton {
                                        set_icon_name: "sidebar-show-symbolic",
                                        set_tooltip_text: Some("Toggle sidebar"),
                                    },

                                    pack_start = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::InstanceDetails || (model.active_sidebar_page == SidebarPage::Assets && model.active_asset_subpage.is_some()) || (model.active_sidebar_page == SidebarPage::Discover && model.discover_details_open) || (model.active_sidebar_page == SidebarPage::Library && model.current_folder.is_some()),
                                        set_icon_name: "go-previous-symbolic",
                                        set_tooltip_text: Some("Go back"),
                                        connect_clicked => AppMsg::GoBack,
                                    },

                                    pack_start = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::InstanceDetails && model.has_selected_mismatch(),
                                        set_icon_name: "dialog-warning-symbolic",
                                        set_has_frame: false,
                                        set_tooltip_text: Some("Mismatch detected between installed mods and the instance version/loader."),
                                        add_css_class: "warning",
                                    },

                                    // ── Right-aligned actions (ordered right-to-left from window controls) ──
                                    // 1. Refresh (always positioned directly adjacent to window controls)
                                    pack_end = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Library,
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Reload all instances from disk"),
                                        connect_clicked => AppMsg::RefreshInstances,
                                    },

                                    pack_end = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Discover && !model.discover_details_open,
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Refresh Discover modpacks"),
                                        connect_clicked => AppMsg::RefreshDiscover,
                                    },

                                    pack_end = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Assets,
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Scan and refresh assets"),
                                        connect_clicked => AppMsg::RefreshAssets,
                                    },

                                    pack_end = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Playtime,
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Refresh Playtime data"),
                                        connect_clicked => AppMsg::RefreshPlaytime,
                                    },

                                    pack_end = &gtk::Button {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Accounts,
                                        set_icon_name: "view-refresh-symbolic",
                                        set_tooltip_text: Some("Refresh accounts"),
                                        connect_clicked => AppMsg::RefreshAccountsRequest,
                                    },

                                    // 2. Discover search toggle
                                    pack_end = &gtk::ToggleButton {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Discover && !model.discover_details_open,
                                        set_icon_name: "edit-find-symbolic",
                                        set_tooltip_text: Some("Search modpacks"),
                                        #[watch]
                                        set_active: model.discover_search_visible,
                                        connect_toggled => AppMsg::ToggleDiscoverSearch,
                                    },

                                    // 3. Layout & Sort SplitButton (Library)
                                    pack_end = &adw::SplitButton {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Library && !model.is_narrow,
                                        #[watch]
                                        set_icon_name: if model.overview_layout == LayoutMode::Grid {
                                            "view-list-symbolic"
                                        } else {
                                            "view-grid-symbolic"
                                        },
                                        #[watch]
                                        set_tooltip_text: Some(if model.overview_layout == LayoutMode::Grid {
                                            "Switch to list view"
                                        } else {
                                            "Switch to grid view"
                                        }),
                                        set_dropdown_tooltip: "Sort instances",
                                        connect_clicked => AppMsg::ToggleOverviewLayout,

                                        #[wrap(Some)]
                                        set_popover: top_sort_popover = &gtk::Popover {
                                            set_autohide: true,
                                            set_position: gtk::PositionType::Bottom,
                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_css_classes: &["menu-box"],
                                                set_width_request: 180,

                                                gtk::Label {
                                                    set_label: "Sort",
                                                    set_halign: gtk::Align::Start,
                                                    set_css_classes: &["menu-subtitle"],
                                                    set_margin_start: 34,
                                                },

                                                #[name = "top_sort_name_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Name"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::Alphabetical,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::Alphabetical));
                                                        }
                                                    },
                                                },

                                                #[name = "top_sort_played_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Last Played"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    set_group: Some(&top_sort_name_rb),
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::LastPlayed,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::LastPlayed));
                                                        }
                                                    },
                                                },

                                                #[name = "top_sort_playtime_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Playtime"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    set_group: Some(&top_sort_name_rb),
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::Playtime,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::Playtime));
                                                        }
                                                    },
                                                },
                                            },
                                        },
                                    },

                                    // 5. Add new instance or group (Library)
                                    pack_end = &gtk::MenuButton {
                                        #[watch]
                                        set_visible: model.active_sidebar_page == SidebarPage::Library && !model.is_narrow,
                                        set_icon_name: "list-add-symbolic",
                                        set_tooltip_text: Some("Add instance or group"),
                                        #[wrap(Some)]
                                        set_popover: top_add_popover = &gtk::Popover {
                                            set_autohide: true,
                                            set_position: gtk::PositionType::Bottom,
                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_css_classes: &["menu-box"],
                                                set_width_request: 180,

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Image {
                                                            set_icon_name: Some("list-add-symbolic"),
                                                        },
                                                        gtk::Label {
                                                            set_label: "Add Instance...",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, top_add_popover] => move |_| {
                                                        top_add_popover.popdown();
                                                        sender.input(AppMsg::HeaderAddInstance);
                                                    },
                                                },

                                                gtk::Separator {
                                                    set_margin_top: 4,
                                                    set_margin_bottom: 4,
                                                },

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Image {
                                                            set_icon_name: Some("folder-new-symbolic"),
                                                        },
                                                        gtk::Label {
                                                            set_label: "New Group...",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, top_add_popover] => move |_| {
                                                        top_add_popover.popdown();
                                                        sender.input(AppMsg::CreateGroupRequest);
                                                    },
                                                },
                                            },
                                        },
                                    },
                                },

                                add_top_bar = &gtk::SearchBar {
                                    #[watch]
                                    set_search_mode: model.discover_search_visible && model.active_sidebar_page == SidebarPage::Discover && !model.discover_details_open,
                                    #[watch]
                                    set_visible: model.active_sidebar_page == SidebarPage::Discover && !model.discover_details_open,
                                    set_key_capture_widget: Some(&root),

                                    #[wrap(Some)]
                                    set_child: discover_search_entry = &gtk::SearchEntry {
                                        set_placeholder_text: Some("Search Modrinth modpacks..."),
                                        set_hexpand: true,
                                        set_margin_start: 12,
                                        set_margin_end: 12,
                                        set_margin_top: 4,
                                        set_margin_bottom: 4,
                                        connect_search_changed[sender] => move |entry| {
                                            sender.input(AppMsg::DiscoverSearchChanged(entry.text().to_string()));
                                        },
                                    },
                                },

                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_vexpand: true,

                                    gtk::Stack {
                                        set_vexpand: true,
                                        set_transition_type: gtk::StackTransitionType::Crossfade,
                                        set_transition_duration: 150,

                                        add_named[Some("library")] = model.overview_grid.widget(),
                                        add_named[Some("discover")] = model.discover_view.widget(),
                                        add_named[Some("accounts")] = model.account_view.widget(),
                                        add_named[Some("playtime")] = model.playtime_view.widget(),
                                        add_named[Some("assets")] = model.asset_view.widget(),

                                        add_named[Some("instance")] = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_vexpand: true,

                                            #[name = "detail_stack"]
                                            adw::ViewStack {
                                                set_vexpand: true,

                                                #[name = "summary_tab"]
                                                add_titled_with_icon[Some("summary"), "Summary", "dialog-information-symbolic"] = &adw::Bin {
                                                    set_tooltip_text: Some("Instance overview and quick actions"),
                                                    #[wrap(Some)]
                                                    set_child = model.instance_summary.widget(),
                                                },

                                                #[name = "editor_tab"]
                                                add_titled_with_icon[Some("editor"), "Editor", "document-edit-symbolic"] = &adw::Bin {
                                                    set_tooltip_text: Some("Edit instance files and configuration"),
                                                    #[wrap(Some)]
                                                    set_child = model.instance_editor_tab.widget(),
                                                },

                                                #[name = "settings_tab"]
                                                add_titled_with_icon[Some("settings"), "Settings", "emblem-system-symbolic"] = &adw::Bin {
                                                    set_tooltip_text: Some("Configure instance settings"),
                                                    #[wrap(Some)]
                                                    set_child = model.instance_settings_tab.widget(),
                                                },

                                                #[name = "console_tab"]
                                                add_titled_with_icon[Some("console"), "Console", "utilities-terminal-symbolic"] = &adw::Bin {
                                                    set_tooltip_text: Some("View instance console logs and output"),
                                                    #[wrap(Some)]
                                                    set_child = model.instance_console.widget(),
                                                },

                                                #[watch]
                                                set_visible_child_name: if model.active_tab.is_empty() { "summary" } else { &model.active_tab },
                                            }
                                        },

                                        #[watch]
                                        set_visible_child_name: match model.active_sidebar_page {
                                            SidebarPage::Library => "library",
                                            SidebarPage::Discover => "discover",
                                            SidebarPage::Accounts => "accounts",
                                            SidebarPage::Playtime => "playtime",
                                            SidebarPage::Assets => "assets",
                                            SidebarPage::InstanceDetails => "instance",
                                        },
                                    }
                                },

                                add_bottom_bar = &gtk::ActionBar {
                                    #[watch]
                                    set_revealed: model.active_sidebar_page == SidebarPage::Library && model.is_narrow,

                                    #[name = "bottom_add_btn"]
                                    pack_end = &gtk::MenuButton {
                                        set_icon_name: "list-add-symbolic",
                                        set_tooltip_text: Some("Add instance or group"),
                                        #[wrap(Some)]
                                        set_popover: bottom_add_popover = &gtk::Popover {
                                            set_autohide: true,
                                            set_position: gtk::PositionType::Top,
                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_css_classes: &["menu-box"],
                                                set_width_request: 180,

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Image {
                                                            set_icon_name: Some("list-add-symbolic"),
                                                        },
                                                        gtk::Label {
                                                            set_label: "Add Instance...",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, bottom_add_popover] => move |_| {
                                                        bottom_add_popover.popdown();
                                                        sender.input(AppMsg::HeaderAddInstance);
                                                    },
                                                },

                                                gtk::Separator {
                                                    set_margin_top: 4,
                                                    set_margin_bottom: 4,
                                                },

                                                gtk::Button {
                                                    set_has_frame: false,
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Box {
                                                        set_orientation: gtk::Orientation::Horizontal,
                                                        set_spacing: 12,
                                                        gtk::Image {
                                                            set_icon_name: Some("folder-new-symbolic"),
                                                        },
                                                        gtk::Label {
                                                            set_label: "New Group...",
                                                            set_hexpand: true,
                                                            set_halign: gtk::Align::Start,
                                                        },
                                                    },
                                                    connect_clicked[sender, bottom_add_popover] => move |_| {
                                                        bottom_add_popover.popdown();
                                                        sender.input(AppMsg::CreateGroupRequest);
                                                    },
                                                },
                                            },
                                        },
                                    },

                                    pack_end = &adw::SplitButton {
                                        #[watch]
                                        set_icon_name: if model.overview_layout == LayoutMode::Grid {
                                            "view-list-symbolic"
                                        } else {
                                            "view-grid-symbolic"
                                        },
                                        #[watch]
                                        set_tooltip_text: Some(if model.overview_layout == LayoutMode::Grid {
                                            "Switch to list view"
                                        } else {
                                            "Switch to grid view"
                                        }),
                                        set_dropdown_tooltip: "Sort instances",
                                        connect_clicked => AppMsg::ToggleOverviewLayout,

                                        #[wrap(Some)]
                                        set_popover: bottom_sort_popover = &gtk::Popover {
                                            set_autohide: true,
                                            set_position: gtk::PositionType::Top,
                                            #[wrap(Some)]
                                            set_child = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_css_classes: &["menu-box"],
                                                set_width_request: 180,

                                                gtk::Label {
                                                    set_label: "Sort",
                                                    set_halign: gtk::Align::Start,
                                                    set_css_classes: &["menu-subtitle"],
                                                    set_margin_start: 34,
                                                },

                                                #[name = "bottom_sort_name_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Name"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::Alphabetical,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::Alphabetical));
                                                        }
                                                    },
                                                },

                                                #[name = "bottom_sort_played_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Last Played"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    set_group: Some(&bottom_sort_name_rb),
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::LastPlayed,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::LastPlayed));
                                                        }
                                                    },
                                                },

                                                #[name = "bottom_sort_playtime_rb"]
                                                gtk::CheckButton {
                                                    set_label: Some("Playtime"),
                                                    set_css_classes: &["flat", "menu-btn"],
                                                    set_group: Some(&bottom_sort_name_rb),
                                                    #[watch]
                                                    set_active: model.config.sort_by == crate::config::SortBy::Playtime,
                                                    connect_toggled[sender] => move |btn| {
                                                        if btn.is_active() {
                                                            sender.input(AppMsg::SetOverviewSortBy(crate::config::SortBy::Playtime));
                                                        }
                                                    },
                                                },
                                            },
                                        },
                                    },
                                },

                                #[name = "instance_view_switcher_bar"]
                                add_bottom_bar = &adw::ViewSwitcherBar {
                                    #[watch]
                                    set_reveal: model.active_sidebar_page == SidebarPage::InstanceDetails && (model.is_narrow || model.split_view.is_collapsed()),
                                },
                            },
                        }
                    }
                }
            }
        }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let settings_dialog = SettingsDialog::builder().launch(config.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                SettingsOutput::ConfigUpdated(new_config) => AppMsg::ConfigUpdated(new_config),
                SettingsOutput::OpenAccountManager => AppMsg::AccountAction,
                SettingsOutput::DownloadProgress(msg) => AppMsg::DownloadProgress(msg),
            },
        );

        let setup_dialog =
            SetupDialog::builder()
                .launch(config.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    SetupOutput::StartMicrosoftLogin => AppMsg::LoginStart,
                    SetupOutput::ConfigUpdated(new_config) => AppMsg::ConfigUpdated(*new_config),
                });

        let add_instance_dialog = AddInstanceDialog::builder()
            .launch(config.instances_path.clone())
            .forward(sender.input_sender(), |msg| match msg {
                AddInstanceOutput::InstanceCreated(version, path, group) => {
                    AppMsg::InstanceCreated(version, path, group)
                }
            });

        let instance_editor = InstanceEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::EditorOutput);

        let download_dialog =
            DownloadDialog::builder()
                .launch(())
                .forward(sender.input_sender(), |out| match out {
                    DownloadDialogOutput::RemoveJob(id) => AppMsg::RemoveJob(id),
                    DownloadDialogOutput::ClearFinishedJobs => AppMsg::ClearFinishedJobs,
                    DownloadDialogOutput::RetryJob(id) => AppMsg::RetryJob(id),
                });

        let java_selector = JavaSelectorDialog::builder()
            .launch(Some(config.minecraft_data_path.join("java")))
            .forward(sender.input_sender(), |out| match out {
                JavaSelectorOutput::Selected(path) => AppMsg::SetInstanceJava(path),
            });

        let playtime_view = PlaytimeView::builder().launch(()).forward(
            sender.input_sender(),
            |output| match output {
                AppMsg::RefreshPlaytime => AppMsg::RefreshPlaytime,
                _ => AppMsg::RefreshPlaytime,
            },
        );

        let shortcuts_dialog = ShortcutsDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |_| unreachable!());

        let component_editor = ComponentEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::ComponentEditorOutput);

        let mod_loader_dialog = ModLoaderDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::ModLoaderOutput);

        let asset_view =
            AssetManagerView::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    AssetOutput::RefreshRequest => AppMsg::RefreshAssets,
                    AssetOutput::SubpageChanged(subtitle) => AppMsg::AssetSubpageChanged(subtitle),
                });

        let sharer_dialog =
            InstanceSharerDialog::builder()
                .launch(())
                .forward(sender.input_sender(), |out| match out {
                    SharerOutput::Generate(idx) => AppMsg::GenerateShareCode(idx),
                    SharerOutput::ExportZip(idx, path) => AppMsg::ExportZip(idx, path),
                });

        let import_dialog =
            ImportDialog::builder()
                .launch(())
                .forward(sender.input_sender(), |out| match out {
                    ImportOutput::Import(code) => AppMsg::ConfirmImportFromCode(code),
                    ImportOutput::ImportZip(path) => AppMsg::ImportZip(path),
                });

        let instances: Vec<Instance> = if config.is_demo {
            crate::backend::core::demo::create_demo_instances()
        } else {
            Vec::new()
        };
        let groups = if config.is_demo {
            crate::backend::core::demo::create_demo_groups()
        } else if let Some(path) = &config.instances_path {
            let g = InstanceGroups::load(path);
            let path_clone = path.clone();
            let sender_clone = sender.input_sender().clone();
            crate::backend::core::tasks::spawn_io(move || {
                let insts = scan_instances(&path_clone);
                let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
            });
            g
        } else {
            InstanceGroups::default()
        };

        let browser_dialog =
            UnifiedBrowser::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    BrowserOutput::InstallItems {
                        editor_type,
                        installs,
                    } => AppMsg::InstallBrowserItems(editor_type, installs),
                });

        let discover_view = DiscoverView::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::DiscoverEvent);

        let sidebar = SidebarList::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::SidebarEvent);

        let overview_grid = OverviewGrid::builder()
            .launch((
                match config.preferred_view_type {
                    PreferredViewType::Grid => LayoutMode::Grid,
                    PreferredViewType::List => LayoutMode::List,
                },
                config.sort_by,
            ))
            .forward(sender.input_sender(), AppMsg::OverviewEvent);

        let mut model = AppModel {
            config: config.clone(),
            instances: instances.clone(),
            groups,
            selected_instance: None,
            add_instance_dialog,
            instance_editor,
            download_dialog,
            java_selector,
            component_editor,
            mod_loader_dialog,
            browser_dialog,

            sharer_dialog,
            import_dialog,
            playtime_view,
            shortcuts_dialog,
            sidebar,
            overview_grid,
            asset_view,
            discover_view,
            settings_dialog,
            setup_dialog,
            active_sidebar_page: SidebarPage::Library,

            instance_summary: InstanceSummary::builder()
                .launch((None, InstanceStatus::NotRunning))
                .forward(sender.input_sender(), AppMsg::Summary),

            instance_editor_tab: InstanceEditorTab::builder()
                .launch((None, config.clone()))
                .forward(sender.input_sender(), AppMsg::Editor),

            instance_settings_tab: InstanceSettingsTab::builder()
                .launch((None, config.clone()))
                .forward(sender.input_sender(), AppMsg::SettingsTab),

            instance_console: InstanceConsole::builder()
                .launch((gtk::TextBuffer::new(None), InstanceStatus::NotRunning, false))
                .forward(sender.input_sender(), AppMsg::Console),

            account_view: AccountView::builder()
                .launch(config.clone())
                .forward(sender.input_sender(), |msg| msg),

            download_status_bar: DownloadStatusBar::builder().launch(()).forward(
                sender.input_sender(),
                |output| match output {
                    DownloadStatusBarOutput::Clicked => AppMsg::OpenDownloadDetails,
                    DownloadStatusBarOutput::Dismiss => AppMsg::DismissDownloadStatus,
                },
            ),

            window: root.clone(),
            split_view: adw::OverlaySplitView::new(),
            loading_instances: !config.is_demo,
            auth_in_progress: false,

            instance_statuses: HashMap::new(),
            instance_processes: HashMap::new(),
            instance_consoles: HashMap::new(),
            instance_logs: HashMap::new(),
            default_console_buffer: gtk::TextBuffer::new(None),
            active_tab: "summary".to_string(),
            console_search_query: String::new(),
            launch_after_download: false,
            toast_overlay: adw::ToastOverlay::new(),
            active_editor_type: None,
            is_narrow: false,
            overview_layout: match config.preferred_view_type {
                PreferredViewType::Grid => LayoutMode::Grid,
                PreferredViewType::List => LayoutMode::List,
            },
            current_folder: None,
            playtime_manager: if config.is_demo {
                crate::backend::core::demo::create_demo_playtime()
            } else {
                PlaytimeManager::load()
            },
            sharing_loading: false,
            import_loading: false,
            verifying_loading: false,
            installing_modpack: false,
            active_asset_subpage: None,
            discover_search_visible: false,
            discover_search_query: String::new(),
            discover_details_open: false,
            discover_details_title: String::new(),
        };

        let widgets = view_output!();

        model.split_view = widgets.split_view.clone();

        widgets
            .split_view
            .bind_property("collapsed", &widgets.open_sidebar_btn, "visible")
            .sync_create()
            .build();

        widgets
            .split_view
            .bind_property("show-sidebar", &widgets.open_sidebar_btn, "active")
            .bidirectional()
            .sync_create()
            .build();

        widgets.instance_switcher.set_stack(Some(&widgets.detail_stack));
        widgets.instance_view_switcher_bar.set_stack(Some(&widgets.detail_stack));

        {
            let s = sender.clone();
            widgets.detail_stack.connect_visible_child_name_notify(move |stack| {
                if let Some(name) = stack.visible_child_name() {
                    s.input(AppMsg::SwitchTab(name.to_string()));
                }
            });
        }
        let bp_condition = adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            680.0,
            adw::LengthUnit::Sp,
        );
        let bp = adw::Breakpoint::new(bp_condition);
        {
            let split = widgets.split_view.clone();
            let sender_apply = sender.clone();
            bp.connect_apply(move |_| {
                split.set_collapsed(true);
                sender_apply.input(AppMsg::SetNarrow(true));
            });
        }
        {
            let split = widgets.split_view.clone();
            let sender_unapply = sender.clone();
            bp.connect_unapply(move |_| {
                split.set_collapsed(false);
                sender_unapply.input(AppMsg::SetNarrow(false));
            });
        }
        root.add_breakpoint(bp);

        let shortcut_controller = gtk::ShortcutController::new();
        shortcut_controller.set_scope(gtk::ShortcutScope::Global);

        shortcut_controller.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>comma"),
            Some(gtk::CallbackAction::new(glib::clone!(
                #[strong]
                sender,
                move |_, _| {
                    sender.input(AppMsg::OpenSettings);
                    glib::Propagation::Stop
                }
            ))),
        ));

        shortcut_controller.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>question"),
            Some(gtk::CallbackAction::new(glib::clone!(
                #[strong]
                sender,
                move |_, _| {
                    sender.input(AppMsg::OpenShortcuts);
                    glib::Propagation::Stop
                }
            ))),
        ));

        root.add_controller(shortcut_controller);

        widgets.detail_stack.set_visible_child_name("summary");

        model.toast_overlay = widgets
            .main_content_box
            .parent()
            .unwrap()
            .downcast::<adw::ToastOverlay>()
            .unwrap();

        {
            use crate::backend::auth::account::{
                refresh_all_accounts, verify_account_status, AccountStatus,
            };
            use crate::backend::auth::microsoft::AccountType;
            let needs_refresh = !config.is_demo && config.accounts.iter().any(|a| {
                a.account_type == AccountType::Microsoft
                    && !a.refresh_token.is_empty()
                    && matches!(
                        verify_account_status(a),
                        AccountStatus::Expired | AccountStatus::ExpiringSoon
                    )
            });
            if needs_refresh {
                let mut config_clone = config.clone();
                let sender_clone = sender.input_sender().clone();
                crate::backend::core::tasks::spawn_io(move || {
                    let _ = refresh_all_accounts(&mut config_clone);
                    let _ = sender_clone.send(AppMsg::RefreshAccountsAll(config_clone));
                });
            }
        }

        if config.is_demo {
            let sender_clone = sender.clone();
            let demo_instances = instances.clone();
            sender_clone.input(AppMsg::InstancesUpdated(demo_instances));
            sender_clone.input(AppMsg::RefreshPlaytime);
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Summary(out) => self.handle_summary_output(out, sender),
            AppMsg::Editor(out) => self.handle_editor_output(out, sender),
            AppMsg::SettingsTab(out) => self.handle_settings_output(out, sender),
            AppMsg::Console(out) => self.handle_console_output(out, sender),

            AppMsg::SidebarEvent(out) => self.handle_sidebar_event(&sender, out),
            AppMsg::OverviewEvent(out) => self.handle_overview_event(&sender, out),
            AppMsg::OverviewBack => self.handle_overview_back(),
            AppMsg::ToggleDiscoverSearch => self.handle_toggle_discover_search(),
            AppMsg::DiscoverSearchChanged(query) => self.handle_discover_search_changed(query),
            AppMsg::GoBack => self.handle_go_back(&sender),
            AppMsg::ShowOverview => self.handle_show_overview(),
            AppMsg::SetNarrow(narrow) => self.handle_set_narrow(narrow),
            AppMsg::SetOverviewLayout(mode) => self.handle_set_overview_layout(mode),
            AppMsg::SetOverviewSortBy(sort_by) => self.handle_set_overview_sort_by(sort_by),
            AppMsg::InstanceLaunched(path, timestamp) => {
                self.handle_instance_launched(path, timestamp)
            }
            AppMsg::ToggleOverviewLayout => self.handle_toggle_overview_layout(&sender),
            AppMsg::ToggleSidebar => self.handle_toggle_sidebar(),
            AppMsg::CreateGroupRequest => self.handle_create_group_request(&sender),
            AppMsg::ConfirmCreateGroup(name) => self.handle_confirm_create_group(name),
            AppMsg::MoveToGroupRequest(idx) => self.handle_move_to_group_request(&sender, idx),
            AppMsg::CreateGroupWithMove(idx) => self.handle_create_group_with_move(&sender, idx),
            AppMsg::MoveInstanceToGroup(idx, group) => {
                self.handle_move_instance_to_group(idx, group)
            }
            AppMsg::RemoveInstanceFromGroup(idx) => self.handle_remove_instance_from_group(idx),
            AppMsg::RenameGroupRequest(old_name) => {
                self.handle_rename_group_request(&sender, old_name)
            }
            AppMsg::ConfirmRenameGroup(old_name, new_name) => {
                self.handle_confirm_rename_group(old_name, new_name)
            }
            AppMsg::DeleteGroupRequest(name) => self.handle_delete_group_request(&sender, name),
            AppMsg::ConfirmDeleteGroup(name) => self.handle_confirm_delete_group(name),
            AppMsg::OpenSettings => self.handle_open_settings(),
            AppMsg::OpenSetup => self.handle_open_setup(),
            AppMsg::OpenAccountSettings => self.handle_open_account_settings(),
            AppMsg::ShowAddOfflineDialog => self.handle_show_add_offline_dialog(&sender),
            AppMsg::OpenAbout => self.handle_open_about(),
            AppMsg::OpenShortcuts => self.handle_open_shortcuts(),
            AppMsg::RefreshInstances => self.handle_refresh_instances(&sender),
            AppMsg::OpenAssetManager => self.handle_open_asset_manager(&sender),
            AppMsg::RefreshAssets => self.handle_refresh_assets(&sender),
            AppMsg::RefreshDiscover => self.handle_refresh_discover(),
            AppMsg::AssetsReady(result) => self.handle_assets_ready(result),
            AppMsg::AssetSubpageChanged(subtitle) => self.handle_asset_subpage_changed(subtitle),
            AppMsg::OpenPlaytime => self.handle_open_playtime(&sender),
            AppMsg::RefreshPlaytime => self.handle_refresh_playtime(&sender),
            AppMsg::PlaytimeDataReady(manager, data) => {
                self.handle_playtime_data_ready(manager, data)
            }
            AppMsg::ConfigUpdated(new_config) => self.handle_config_updated(&sender, new_config),
            AppMsg::InstancesUpdated(instances) => {
                self.handle_instances_updated(&sender, instances)
            }
            AppMsg::RefreshSelectedInstance => self.handle_refresh_selected_instance(&sender),
            AppMsg::SelectedInstanceUpdated(updated_inst) => {
                self.handle_selected_instance_updated(updated_inst)
            }
            AppMsg::SelectInstance(index) => self.handle_select_instance(&sender, index),
            AppMsg::AddInstance(target_group) => self.handle_add_instance(target_group),
            AppMsg::HeaderAddInstance => self.handle_header_add_instance(&sender),
            AppMsg::InstanceCreated(version, path, group) => {
                self.handle_instance_created(&sender, version, path, group)
            }
            AppMsg::ShareInstance(idx) => self.handle_share_instance(idx),
            AppMsg::GenerateShareCode(idx) => self.handle_generate_share_code(&sender, idx),
            AppMsg::ExportZip(idx, path) => self.handle_export_zip(&sender, idx, path),
            AppMsg::DisplayShareCode(code) => self.handle_display_share_code(code),
            AppMsg::ImportRequest => self.handle_import_request(),
            AppMsg::ConfirmImportFromCode(code) => {
                self.handle_confirm_import_from_code(&sender, code)
            }
            AppMsg::PerformImport(manifest, code) => {
                self.handle_perform_import(&sender, manifest, code)
            }
            AppMsg::ImportZip(path) => self.handle_import_zip(&sender, path),
            AppMsg::SetSharingLoading(loading, title, subtitle, show_progress) => {
                self.handle_set_sharing_loading(loading, title, subtitle, show_progress)
            }
            AppMsg::UpdateSharingProgress(p, s) => self.handle_update_sharing_progress(p, s),
            AppMsg::SetImportLoading(loading) => self.handle_set_import_loading(loading),
            AppMsg::SetVerifyingLoading(loading) => self.handle_set_verifying_loading(loading),
            AppMsg::UpdateImportStatus(status) => self.handle_update_import_status(status),
            AppMsg::EditComponents => self.handle_edit_components(),
            AppMsg::EditMods => self.handle_edit_mods(),
            AppMsg::EditResourcePacks => self.handle_edit_resource_packs(),
            AppMsg::EditShaderPacks => self.handle_edit_shader_packs(),
            AppMsg::EditWorlds => self.handle_edit_worlds(),
            AppMsg::EditorOutput(output) => self.handle_editor_output_action(&sender, output),
            AppMsg::ConfirmMoveItems(editor_type, ids, target_idx) => {
                self.handle_confirm_move_items(&sender, editor_type, ids, target_idx)
            }
            AppMsg::ConfirmCopyItems(editor_type, ids, target_idx) => {
                self.handle_confirm_copy_items(&sender, editor_type, ids, target_idx)
            }
            AppMsg::OpenModsFolder => self.handle_open_mods_folder(),
            AppMsg::OpenResourcePacksFolder => self.handle_open_resource_packs_folder(),
            AppMsg::OpenShaderPacksFolder => self.handle_open_shader_packs_folder(),
            AppMsg::OpenScreenshotsFolder => self.handle_open_screenshots_folder(),
            AppMsg::OpenWorldsFolder => self.handle_open_worlds_folder(),
            AppMsg::OpenInstanceFolder => self.handle_open_instance_folder(),
            AppMsg::BrowseModrinth(editor_type) => self.handle_browse_modrinth(editor_type),
            AppMsg::ModUpdatesResult(result) => self.handle_mod_updates_result(result),
            AppMsg::ModUpdateSuccess(filename) => self.handle_mod_update_success(filename),
            AppMsg::ModUpdateAllSuccess(filenames) => self.handle_mod_update_all_success(filenames),
            AppMsg::InstallBrowserItems(editor_type, installs) => {
                self.handle_install_browser_items(&sender, editor_type, installs)
            }
            AppMsg::ModrinthInstallResult(editor_type, result) => {
                self.handle_modrinth_install_result(editor_type, result)
            }
            AppMsg::RenameInstanceRequest(index) => {
                self.handle_rename_instance_request(&sender, index)
            }
            AppMsg::ConfirmRename(index, new_name) => self.handle_confirm_rename(index, new_name),
            AppMsg::DeleteInstanceRequest(index) => {
                self.handle_delete_instance_request(&sender, index)
            }
            AppMsg::ConfirmDelete(index) => self.handle_confirm_delete(index),
            AppMsg::ChangeInstanceIconFromFile(idx) => {
                self.handle_change_instance_icon_from_file(&sender, idx)
            }
            AppMsg::ApplyDefaultIcon(idx) => self.handle_apply_default_icon(&sender, idx),
            AppMsg::ApplyIconPath(idx, source_path) => {
                self.handle_apply_icon_path(&sender, idx, source_path)
            }
            AppMsg::AccountAction => self.handle_account_action(),
            AppMsg::SwitchAccount(uuid) => self.handle_switch_account(uuid),
            AppMsg::RemoveAccount(uuid) => self.handle_remove_account(uuid),
            AppMsg::AddOfflineAccount(username) => self.handle_add_offline_account(username),
            AppMsg::VerifyAccount(_) => {}
            AppMsg::VerifyAccountResult(_, _) => {}
            AppMsg::RefreshAccount(_) => {}
            AppMsg::RefreshAccountResult(result) => self.handle_refresh_account_result(result),
            AppMsg::RefreshAccountsRequest => self.handle_refresh_accounts_request(),
            AppMsg::RefreshAccountsAll(new_config) => self.handle_refresh_accounts_all(new_config),
            AppMsg::OpenJavaSelector => self.handle_open_java_selector(),
            AppMsg::SetInstanceJava(path) => self.handle_set_instance_java(&sender, path),
            AppMsg::SetInstanceJavaDefault => self.handle_set_instance_java_default(&sender),
            AppMsg::OpenComponentSwap(uid) => self.handle_open_component_swap(uid),
            AppMsg::RemoveComponent(uid) => self.handle_remove_component(&sender, uid),
            AppMsg::SelectModLoaderRequest => self.handle_select_mod_loader_request(),
            AppMsg::ModLoaderOutput(output) => self.handle_mod_loader_output(&sender, output),
            AppMsg::ComponentEditorOutput(output) => {
                self.handle_component_editor_output(&sender, output)
            }
            AppMsg::LoginStart => self.handle_login_start(&sender),
            AppMsg::LoginDeviceCode(code, uri) => self.handle_login_device_code(code, uri),
            AppMsg::LoginResult(result) => self.handle_login_result(result),
            AppMsg::Logout => self.handle_logout(),
            AppMsg::SwitchTab(name) => self.handle_switch_tab(name),
            AppMsg::SetInstanceFeralGameMode(enabled) => {
                self.handle_set_instance_feral_game_mode(enabled)
            }
            AppMsg::SetInstanceDiscreteGpu(enabled) => {
                self.handle_set_instance_discrete_gpu(enabled)
            }
            AppMsg::SetInstanceZinkVulkan(enabled) => self.handle_set_instance_zink_vulkan(enabled),
            AppMsg::SetInstanceUseWayland(enabled) => self.handle_set_instance_use_wayland(enabled),
            AppMsg::KillInstance => self.handle_kill_instance(&sender),
            AppMsg::KillInstanceFromIndex(idx) => self.handle_kill_instance_from_index(idx),
            AppMsg::VerifyInstance => self.handle_verify_instance(&sender),
            AppMsg::LaunchInstance => self.handle_launch_instance(&sender),
            AppMsg::LaunchInstanceFromIndex(idx) => {
                self.handle_launch_instance_from_index(&sender, idx)
            }
            AppMsg::ConsoleLog(path, msg) => self.handle_console_log(path, msg),
            AppMsg::ProcessFinished(path, duration, start, end) => {
                self.handle_process_finished(&sender, path, duration, start, end)
            }
            AppMsg::ClearConsole(path) => self.handle_clear_console(path),
            AppMsg::SetConsoleSearchQuery(query) => self.handle_set_console_search_query(query),
            AppMsg::DiscoverEvent(out) => self.handle_discover_event(&sender, out),
            AppMsg::DownloadStart(raw_version, loader, loader_version) => {
                self.handle_download_start(&sender, raw_version, loader, loader_version)
            }
            AppMsg::DownloadProgress(msg) => self.handle_download_progress(&sender, msg),
            AppMsg::DownloadFinished => self.handle_download_finished(&sender),
            AppMsg::DownloadError(err) => self.handle_download_error(err),
            AppMsg::DismissDownloadStatus => self.handle_dismiss_download_status(),
            AppMsg::RemoveJob(id) => self.handle_remove_job(id),
            AppMsg::ClearFinishedJobs => self.handle_clear_finished_jobs(),
            AppMsg::RetryJob(id) => self.handle_retry_job(&sender, id),
            AppMsg::OpenDownloadDetails => self.handle_open_download_details(),
        }
    }
}
