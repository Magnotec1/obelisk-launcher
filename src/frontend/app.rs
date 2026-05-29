#![allow(unused_assignments)]
use crate::backend::auth::account::{
    add_account, create_offline_account, get_active_account,
    remove_account, switch_account,
};
use crate::backend::auth::microsoft::{self as auth, Account};
use crate::backend::download::manager::DownloadMsg;
use crate::backend::instance::launcher::{check_instance_assets, launch_instance, LaunchOptions};
use crate::backend::instance::manager::{
    add_instance_item, delete_instance, is_loader_component, remove_component,
    remove_instance_item, remove_mod_loader, rename_instance, scan_instances,
    scan_single_instance, set_component_version, set_instance_java,
    set_mod_loader_with_version, Instance, ModLoader,
};
use crate::backend::playtime::{PlaytimeManager, PlaySession};
use chrono::Utc;
use crate::backend::runtime::versions::{find_version_by_id, MinecraftVersion, RawVersion};
use crate::config::Config;
use crate::frontend::dialogs::external::download::{
    DownloadDialog, DownloadDialogInput, DownloadDialogOutput, DownloadStatusBar, DownloadStatusBarInput,
    DownloadStatusBarOutput, DownloadState,
};
use crate::frontend::dialogs::external::modrinth::{BrowserInput, BrowserOutput, ModrinthBrowser};
use crate::frontend::dialogs::instance::add::{
    AddInstanceDialog, AddInstanceInput, AddInstanceOutput,
};
use crate::frontend::dialogs::instance::components::{
    ComponentEditorDialog, ComponentEditorInput, ComponentEditorOutput,
};
use crate::frontend::dialogs::instance::mod_loader::{
    ModLoaderDialog, ModLoaderDialogInput, ModLoaderDialogOutput,
};
use crate::frontend::dialogs::instance::editor::{
    EditorInput, EditorItem, EditorOutput, EditorType, InstanceEditorDialog,
};

use crate::frontend::dialogs::instance::sharing::{
    ImportDialog, ImportInput, ImportOutput, InstanceSharerDialog, SharerInput, SharerOutput,
    ImportStep,
};
use crate::frontend::dialogs::system::java::{
    JavaSelectorDialog, JavaSelectorInput, JavaSelectorOutput,
};
use crate::frontend::dialogs::system::shortcuts::ShortcutsDialog;
use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::sharing::{export_instance, import_shared_instance, SharedInstance, export_instance_to_zip, import_instance_from_zip};
pub use crate::frontend::views::instance::tabs::console::{LogLevel, LogLine};
use crate::frontend::views::account::{AccountInput, AccountView};
use crate::frontend::views::library::{LayoutMode, OverviewGrid, OverviewInput, OverviewOutput};
use crate::frontend::views::playtime::{PlaytimeInput, PlaytimeView};
use crate::frontend::views::assets::{AssetInput, AssetOutput, AssetManagerView};
use crate::frontend::views::settings::{SettingsInput, SettingsOutput, SettingsDialog};
use crate::frontend::views::sidebar::{SidebarInput, SidebarList, SidebarOutput, SidebarPage};
use crate::frontend::views::instance::{
    ConsoleInput, ConsoleOutput, EditorTabInput, EditorTabOutput,
    InstanceConsole, InstanceEditorTab, InstanceSettingsTab, InstanceSummary,
    SummaryInput, SummaryOutput, SettingsTabInput, SettingsTabOutput,
};
use adw::prelude::*;
use gtk::glib;
use relm4::prelude::*;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;



pub struct AppModel {
    config: Config,
    instances: Vec<Instance>,
    groups: InstanceGroups,
    selected_instance: Option<usize>,
    add_instance_dialog: Controller<AddInstanceDialog>,
    instance_editor: Controller<InstanceEditorDialog>,
    download_dialog: Controller<DownloadDialog>,
    java_selector: Controller<JavaSelectorDialog>,
    component_editor: Controller<ComponentEditorDialog>,
    mod_loader_dialog: Controller<ModLoaderDialog>,
    modrinth_browser: Controller<ModrinthBrowser>,

    sharer_dialog: Controller<InstanceSharerDialog>,
    import_dialog: Controller<ImportDialog>,
    playtime_view: Controller<PlaytimeView>,
    shortcuts_dialog: Controller<ShortcutsDialog>,

    // Sidebar + overview
    sidebar: Controller<SidebarList>,
    overview_grid: Controller<OverviewGrid>,
    active_sidebar_page: SidebarPage,

    // Views
    instance_summary: Controller<InstanceSummary>,
    instance_editor_tab: Controller<InstanceEditorTab>,
    instance_console: Controller<InstanceConsole>,
    instance_settings_tab: Controller<InstanceSettingsTab>,
    account_view: Controller<AccountView>,
    asset_view: Controller<AssetManagerView>,
    settings_dialog: Controller<SettingsDialog>,
    download_status_bar: Controller<DownloadStatusBar>,

    window: adw::Window,
    split_view: adw::OverlaySplitView,

    // Auth state
    auth_in_progress: bool,

    // Launch state
    running_instances: HashSet<PathBuf>,
    instance_processes: HashMap<PathBuf, Arc<Mutex<Option<Child>>>>,
    instance_consoles: HashMap<PathBuf, gtk::TextBuffer>,
    instance_logs: HashMap<PathBuf, Vec<LogLine>>,
    default_console_buffer: gtk::TextBuffer,
    active_tab: String,
    console_filter: LogLevel,

    // Download state
    loading_instances: bool,
    launch_after_download: bool,
    toast_overlay: adw::ToastOverlay,
    active_editor_type: Option<EditorType>,
    is_narrow: bool,
    overview_layout: LayoutMode,
    current_folder: Option<String>,
    playtime_manager: PlaytimeManager,
    sharing_loading: bool,
    import_loading: bool,
    verifying_loading: bool,
}

#[derive(Debug)]
pub enum AppMsg {
    OpenSettings,
    OpenAbout,
    OpenShortcuts,
    OpenAssetManager,
    RefreshAssets,
    AssetsReady(crate::backend::download::assets::AssetScanResult),
    OpenPlaytime,
    RefreshPlaytime,
    PlaytimeDataReady(PlaytimeManager, Vec<(String, String, u64)>), // manager, (id, name, seconds)
    ConfigUpdated(Config),
    SelectInstance(usize),
    AddInstance,
    RefreshInstances,
    RefreshSelectedInstance,
    SelectedInstanceUpdated(Instance),
    EditComponents,
    EditMods,
    EditorOutput(EditorOutput),
    OpenModsFolder,
    EditResourcePacks,
    OpenResourcePacksFolder,
    EditShaderPacks,
    OpenShaderPacksFolder,
    EditWorlds,
    OpenWorldsFolder,
    OpenScreenshotsFolder,
    OpenInstanceFolder,
    BrowseModrinth(EditorType),
    OpenJavaSelector,
    SetInstanceJava(std::path::PathBuf),
    SetInstanceJavaDefault,
    OpenComponentSwap(String), // UID
    RemoveComponent(String),   // UID
    SelectModLoaderRequest,
    ModLoaderOutput(ModLoaderDialogOutput),
    ComponentEditorOutput(ComponentEditorOutput),
    InstallModrinthMods(Vec<(String, String)>), // (Project ID, Version ID)
    ModrinthInstallResult(Result<usize, String>), // Number of mods installed or error

    // Instance management
    RenameInstanceRequest(usize),
    DeleteInstanceRequest(usize),
    InstanceCreated(MinecraftVersion, std::path::PathBuf),
    ConfirmDelete(usize),
    ConfirmRename(usize, String),
    InstancesUpdated(Vec<Instance>),
    /// Open the native file picker for an instance icon (from the icon chooser).
    ChangeInstanceIconFromFile(usize),
    /// Apply the global default icon to an instance.
    ApplyDefaultIcon(usize),
    /// Apply a specific icon file to an instance (e.g. from recents).
    ApplyIconPath(usize, PathBuf),

    // Sidebar / group management
    SidebarEvent(SidebarOutput),
    OverviewEvent(OverviewOutput),
    OverviewBack,
    GoBack,
    ShowOverview,
    ToggleSidebar,
    CreateGroupRequest,
    ConfirmCreateGroup(String),
    MoveToGroupRequest(usize),
    CreateGroupWithMove(usize),
    MoveInstanceToGroup(usize, String),
    RemoveInstanceFromGroup(usize),
    RenameGroupRequest(String),
    ConfirmRenameGroup(String, String),
    DeleteGroupRequest(String),
    ConfirmDeleteGroup(String),

    // Tab Outputs
    Summary(SummaryOutput),
    Editor(EditorTabOutput),
    SettingsTab(SettingsTabOutput),
    Console(ConsoleOutput),

    // Auth
    AccountAction,
    LoginStart,
    LoginDeviceCode(String, String),
    LoginResult(Result<Account, String>),
    Logout,
    SwitchAccount(String),       // UUID
    RemoveAccount(String),       // UUID
    AddOfflineAccount(String),   // Username
    VerifyAccount(String),       // UUID — now handled in AccountView, kept for compat
    VerifyAccountResult(String, String), // UUID, status message
    RefreshAccount(String),      // UUID — now handled in AccountView, kept for compat
    RefreshAccountResult(Result<Account, String>),
    RefreshAccountsAll(Config),  // Full config with all refreshed accounts
    RefreshAccountsRequest,
    ShowAddOfflineDialog,
    OpenAccountSettings,

    // Launching
    LaunchInstance,
    VerifyInstance,
    KillInstance,
    ConsoleLog(PathBuf, String),
    ClearConsole(PathBuf),
    ClearActiveConsole,
    SetConsoleFilter(LogLevel),
    ProcessFinished(PathBuf, u64, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>),
    SwitchTab(String),

    // Performance Tweaks
    SetInstanceFeralGameMode(bool),
    SetInstanceDiscreteGpu(bool),
    SetInstanceZinkVulkan(bool),

    // Downloading
    DownloadStart(RawVersion, ModLoader, Option<String>),
    DownloadProgress(DownloadMsg),
    RemoveJob(String),
    ClearFinishedJobs,
    OpenDownloadDetails,
    DownloadFinished,
    DownloadError(String),
    DismissDownloadStatus,
    SetNarrow(bool),
    SetOverviewLayout(LayoutMode),

    // Sharing
    ShareInstance(usize),
    GenerateShareCode(usize),
    DisplayShareCode(String),
    ImportRequest,
    ConfirmImportFromCode(String),
    PerformImport(String, Option<String>),
    ExportZip(usize, PathBuf),
    ImportZip(PathBuf),

    SetSharingLoading(bool, String, String, bool),
    UpdateSharingProgress(f64, String),
    SetImportLoading(bool),
    SetVerifyingLoading(bool),
    UpdateImportStatus(String),
    ConfirmMoveItems(EditorType, Vec<String>, usize), // type, IDs, target_instance_index
    ConfirmCopyItems(EditorType, Vec<String>, usize), // type, IDs, target_instance_index
    ToggleOverviewLayout,
}

impl AppModel {
    fn get_active_console_buffer(&self) -> gtk::TextBuffer {
        if let Some(index) = self.selected_instance {
            if let Some(inst) = self.instances.get(index) {
                if let Some(buf) = self.instance_consoles.get(&inst.path) {
                    return buf.clone();
                }
            }
        }
        self.default_console_buffer.clone()
    }

    fn get_active_instance_path(&self) -> Option<PathBuf> {
        self.selected_instance
            .and_then(|i| self.instances.get(i))
            .map(|inst| inst.path.clone())
    }

    fn get_console_buffer(&mut self, path: &std::path::Path) -> gtk::TextBuffer {
        self.instance_logs.entry(path.to_path_buf()).or_default();
        self.instance_consoles
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                let buf = gtk::TextBuffer::new(None);
                buf.set_text(&format!("Console initialized for {}\n", path.display()));
                buf
            })
            .clone()
    }

    fn rebuild_console_buffer(&mut self, path: &PathBuf) {
        let filter = self.console_filter;
        let buf = self.get_console_buffer(path);
        buf.set_text("");

        if let Some(logs) = self.instance_logs.get(path) {
            let mut iter = buf.end_iter();
            for line in logs {
                if filter == LogLevel::All || line.level == filter {
                    buf.insert(&mut iter, &line.content);
                }
            }
        }
    }

    fn is_active_instance_running(&self) -> bool {
        if let Some(path) = self.get_active_instance_path() {
            return self.running_instances.contains(&path);
        }
        false
    }

    fn get_game_process(&mut self, path: &std::path::Path) -> Arc<Mutex<Option<Child>>> {
        self.instance_processes
            .entry(path.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    fn handle_summary_output(&mut self, output: SummaryOutput, sender: ComponentSender<Self>) {
        match output {
            SummaryOutput::Launch => sender.input(AppMsg::LaunchInstance),
            SummaryOutput::Verify => sender.input(AppMsg::VerifyInstance),
            SummaryOutput::Kill => sender.input(AppMsg::KillInstance),
            SummaryOutput::OpenFolder => sender.input(AppMsg::OpenInstanceFolder),
            SummaryOutput::SwitchToConsole => sender.input(AppMsg::SwitchTab("console".to_string())),
            SummaryOutput::Share => {
                if let Some(idx) = self.selected_instance {
                    sender.input(AppMsg::ShareInstance(idx));
                }
            }
        }
    }

    fn handle_editor_output(&mut self, output: EditorTabOutput, sender: ComponentSender<Self>) {
        match output {
            EditorTabOutput::EditMods => sender.input(AppMsg::EditMods),
            EditorTabOutput::ExploreMods => sender.input(AppMsg::BrowseModrinth(EditorType::Mods)),
            EditorTabOutput::EditComponents => sender.input(AppMsg::EditComponents),
            EditorTabOutput::EditResourcePacks => sender.input(AppMsg::EditResourcePacks),
            EditorTabOutput::EditShaderPacks => sender.input(AppMsg::EditShaderPacks),
            EditorTabOutput::EditWorlds => sender.input(AppMsg::EditWorlds),
            EditorTabOutput::OpenScreenshotsFolder => sender.input(AppMsg::OpenScreenshotsFolder),
            EditorTabOutput::OpenJavaSelector => sender.input(AppMsg::OpenJavaSelector),
            EditorTabOutput::SetInstanceJavaDefault => sender.input(AppMsg::SetInstanceJavaDefault),
            EditorTabOutput::OpenComponentSwap(uid) => sender.input(AppMsg::OpenComponentSwap(uid)),
            EditorTabOutput::RemoveComponent(uid) => sender.input(AppMsg::RemoveComponent(uid)),
            EditorTabOutput::SelectModLoaderRequest => sender.input(AppMsg::SelectModLoaderRequest),
        }
    }

    fn handle_settings_output(&mut self, output: SettingsTabOutput, sender: ComponentSender<Self>) {
        match output {
            SettingsTabOutput::SetFeralGameMode(e) => sender.input(AppMsg::SetInstanceFeralGameMode(e)),
            SettingsTabOutput::SetDiscreteGpu(e) => sender.input(AppMsg::SetInstanceDiscreteGpu(e)),
            SettingsTabOutput::SetZinkVulkan(e) => sender.input(AppMsg::SetInstanceZinkVulkan(e)),
        }
    }

    fn handle_console_output(&mut self, output: ConsoleOutput, sender: ComponentSender<Self>) {
        match output {
            ConsoleOutput::Launch => sender.input(AppMsg::LaunchInstance),
            ConsoleOutput::Kill => sender.input(AppMsg::KillInstance),
            ConsoleOutput::Clear => sender.input(AppMsg::ClearActiveConsole),
            ConsoleOutput::SetFilter(level) => sender.input(AppMsg::SetConsoleFilter(level)),
        }
    }

    fn has_selected_mismatch(&self) -> bool {
        if let Some(idx) = self.selected_instance {
            if let Some(inst) = self.instances.get(idx) {
                return inst.has_mismatch;
            }
        }
        false
    }
}

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
                            set_title: "No Instance Folder",
                            set_description: Some("Please set your instances directory in settings."),
                            set_icon_name: Some("folder-open-symbolic"),
                            set_vexpand: true,

                            gtk::Button {
                                set_label: "Configure Settings",
                                set_halign: gtk::Align::Center,
                                set_css_classes: &["suggested-action"],
                                connect_clicked => AppMsg::OpenSettings,
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
                            set_sidebar = &adw::NavigationPage {
                                set_title: "Obelisk",
                                #[wrap(Some)]
                                set_child = &adw::ToolbarView {
                                    add_top_bar = &adw::HeaderBar {
                                        #[wrap(Some)]
                                        set_title_widget = &adw::WindowTitle {
                                            set_title: "Obelisk",
                                        },
                                        set_show_end_title_buttons: false,

                                         #[name = "close_sidebar_btn"]
                                         pack_start = &gtk::Button {
                                             set_icon_name: "go-previous-symbolic",
                                             set_tooltip_text: Some("Close sidebar"),
                                             set_has_frame: false,
                                             connect_clicked => AppMsg::ToggleSidebar,
                                         },

                                        pack_end = &gtk::MenuButton {
                                            set_icon_name: "view-more-symbolic",
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
                                                                set_label: "About",
                                                                set_hexpand: true,
                                                                set_halign: gtk::Align::Start,
                                                            },
                                                        },
                                                        connect_clicked[sender, main_popover] => move |_| {
                                                            main_popover.popdown();
                                                            sender.input(AppMsg::OpenAbout);
                                                        },
                                                    },
                                                }
                                            }
                                        },
                                    },

                                    // Sidebar content
                                    #[wrap(Some)]
                                    set_content = model.sidebar.widget(),

                                     add_bottom_bar = model.download_status_bar.widget(),
                                }
                            },

                            // ── Content pane ─────────────────────────────────
                            #[wrap(Some)]
                            set_content = &adw::NavigationPage {
                                set_title: "Details",
                                #[wrap(Some)]
                                set_child = &adw::ToolbarView {
                                    add_top_bar = &adw::HeaderBar {
                                        // Sidebar toggle (only shown when sidebar is collapsed)
                                        pack_start = &gtk::Button {
                                            set_icon_name: "sidebar-show-symbolic",
                                            set_tooltip_text: Some("Show Sidebar"),
                                            connect_clicked[sender] => move |_| {
                                                sender.input(AppMsg::ToggleSidebar);
                                            },
                                        },

                                        pack_start = &gtk::Button {
                                            set_icon_name: "go-previous-symbolic",
                                            #[watch]
                                            set_tooltip_text: Some(if model.active_sidebar_page == SidebarPage::InstanceDetails { "Back to library" } else { "Back to all instances" }),
                                            set_has_frame: false,
                                            #[watch]
                                            set_visible: (model.active_sidebar_page == SidebarPage::Library && model.current_folder.is_some())
                                                || model.active_sidebar_page == SidebarPage::InstanceDetails,
                                            connect_clicked[sender] => move |_| {
                                                sender.input(AppMsg::GoBack);
                                            },
                                        },

                                        #[wrap(Some)]
                                        #[name = "title_widget"]
                                        set_title_widget = &gtk::Stack {
                                            add_named[Some("library")] = &adw::WindowTitle {
                                                set_title: "Library",
                                                #[watch]
                                                set_subtitle: model.current_folder.as_deref().unwrap_or(""),
                                            },
                                            add_named[Some("accounts")] = &adw::WindowTitle {
                                                set_title: "Accounts",
                                            },
                                            add_named[Some("playtime")] = &adw::WindowTitle {
                                                set_title: "Playtime",
                                            },
                                            add_named[Some("assets")] = &adw::WindowTitle {
                                                set_title: "Assets",
                                            },
                                            add_named[Some("details")] = &adw::ViewSwitcher {
                                                set_policy: adw::ViewSwitcherPolicy::Narrow,
                                                #[watch]
                                                set_stack: Some(&detail_stack),
                                            },
                                            // Must come after add_named so children exist on first render
                                            #[watch]
                                            set_visible_child_name: match model.active_sidebar_page {
                                                SidebarPage::Library => "library",
                                                SidebarPage::Accounts => "accounts",
                                                SidebarPage::Playtime => "playtime",
                                                SidebarPage::Assets => "assets",
                                                SidebarPage::InstanceDetails => "details",
                                            },
                                        },

                                        pack_end = &gtk::Stack {
                                            set_hhomogeneous: false,
                                            set_vhomogeneous: false,

                                            add_named[Some("library")] = &gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_spacing: 4,

                                                gtk::MenuButton {
                                                    set_icon_name: "list-add-symbolic",
                                                    set_tooltip_text: Some("Add"),
                                                    set_css_classes: &["flat"],
                                                    #[wrap(Some)]
                                                    set_popover: add_popover = &gtk::Popover {
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
                                                                        set_label: "Create Instance",
                                                                        set_hexpand: true,
                                                                        set_halign: gtk::Align::Start,
                                                                    },
                                                                },
                                                                connect_clicked[sender, add_popover] => move |_| {
                                                                    add_popover.popdown();
                                                                    sender.input(AppMsg::AddInstance);
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
                                                                        set_label: "Create Group",
                                                                        set_hexpand: true,
                                                                        set_halign: gtk::Align::Start,
                                                                    },
                                                                },
                                                                connect_clicked[sender, add_popover] => move |_| {
                                                                    add_popover.popdown();
                                                                    sender.input(AppMsg::CreateGroupRequest);
                                                                },
                                                            },

                                                            gtk::Separator {
                                                                set_css_classes: &["menu-separator"],
                                                            },

                                                            gtk::Button {
                                                                set_has_frame: false,
                                                                set_css_classes: &["flat", "menu-btn"],
                                                                #[wrap(Some)]
                                                                set_child = &gtk::Box {
                                                                    set_orientation: gtk::Orientation::Horizontal,
                                                                    set_spacing: 12,
                                                                    gtk::Label {
                                                                        set_label: "Import Instance",
                                                                        set_hexpand: true,
                                                                        set_halign: gtk::Align::Start,
                                                                    },
                                                                },
                                                                connect_clicked[sender, add_popover] => move |_| {
                                                                    add_popover.popdown();
                                                                    sender.input(AppMsg::ImportRequest);
                                                                },
                                                            },
                                                        }
                                                    }
                                                },

                                                gtk::Button {
                                                    #[watch]
                                                    set_icon_name: if model.overview_layout == LayoutMode::Grid { "view-list-symbolic" } else { "view-grid-symbolic" },
                                                    #[watch]
                                                    set_tooltip_text: Some(if model.overview_layout == LayoutMode::Grid { "List View" } else { "Grid View" }),
                                                    set_css_classes: &["flat"],
                                                    connect_clicked => AppMsg::ToggleOverviewLayout,
                                                },

                                                gtk::Button {
                                                    set_icon_name: "view-refresh-symbolic",
                                                    set_tooltip_text: Some("Refresh Instances"),
                                                    set_css_classes: &["flat"],
                                                    #[watch]
                                                    set_sensitive: !model.loading_instances,
                                                    connect_clicked => AppMsg::RefreshInstances,
                                                },
                                            },

                                            add_named[Some("accounts")] = &gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_spacing: 4,

                                                gtk::MenuButton {
                                                    set_icon_name: "list-add-symbolic",
                                                    set_tooltip_text: Some("Add Account"),
                                                    set_css_classes: &["flat"],
                                                    #[wrap(Some)]
                                                    set_popover: add_account_popover = &gtk::Popover {
                                                        gtk::Box {
                                                            set_orientation: gtk::Orientation::Vertical,
                                                            set_css_classes: &["menu-box"],

                                                            gtk::Button {
                                                                set_css_classes: &["flat", "menu-btn"],
                                                                gtk::Box {
                                                                    set_spacing: 12,
                                                                    gtk::Image::from_icon_name("web-browser-symbolic"),
                                                                    gtk::Label::new(Some("Microsoft Account")),
                                                                },
                                                                connect_clicked[sender, add_account_popover] => move |_| {
                                                                    add_account_popover.popdown();
                                                                    sender.input(AppMsg::LoginStart);
                                                                }
                                                            },

                                                            gtk::Button {
                                                                set_css_classes: &["flat", "menu-btn"],
                                                                gtk::Box {
                                                                    set_spacing: 12,
                                                                    gtk::Image::from_icon_name("network-offline-symbolic"),
                                                                    gtk::Label::new(Some("Offline Account")),
                                                                },
                                                                connect_clicked[sender, add_account_popover] => move |_| {
                                                                    add_account_popover.popdown();
                                                                    sender.input(AppMsg::ShowAddOfflineDialog);
                                                                }
                                                            },
                                                        }
                                                    }
                                                },

                                                gtk::Button {
                                                    set_icon_name: "view-refresh-symbolic",
                                                    set_tooltip_text: Some("Refresh Accounts"),
                                                    set_css_classes: &["flat"],
                                                    connect_clicked => AppMsg::RefreshAccountsRequest,
                                                },
                                            },

                                            add_named[Some("playtime")] = &gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_spacing: 4,

                                                gtk::Button {
                                                    set_icon_name: "view-refresh-symbolic",
                                                    set_tooltip_text: Some("Refresh Playtime"),
                                                    set_css_classes: &["flat"],
                                                    connect_clicked => AppMsg::RefreshPlaytime,
                                                },
                                            },
                                            
                                            add_named[Some("assets")] = &gtk::Box {
                                                set_orientation: gtk::Orientation::Horizontal,
                                                set_spacing: 4,

                                                gtk::Button {
                                                    set_icon_name: "view-refresh-symbolic",
                                                    set_tooltip_text: Some("Refresh Assets"),
                                                    set_css_classes: &["flat"],
                                                    connect_clicked => AppMsg::RefreshAssets,
                                                },
                                            },

                                            add_named[Some("empty")] = &gtk::Box {},

                                            // Must come after add_named so children exist on first render
                                            #[watch]
                                            set_visible_child_name: match model.active_sidebar_page {
                                                SidebarPage::Library => "library",
                                                SidebarPage::Accounts => "accounts",
                                                SidebarPage::Playtime => "playtime",
                                                SidebarPage::Assets => "assets",
                                                _ => "empty",
                                            },
                                        },
                                    },

                                    #[wrap(Some)]
                                    set_content = &gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_vexpand: true,

                                        adw::Banner {
                                            set_title: "Warning! Both 'minecraft' and '.minecraft' folders exist.",
                                            set_button_label: Some("Open Instance Folder"),
                                            set_use_markup: false,
                                            #[watch]
                                            set_revealed: model.has_selected_mismatch(),
                                            connect_button_clicked => AppMsg::OpenInstanceFolder,
                                        },

                                        gtk::Stack {
                                            set_vexpand: true,

                                            add_named[Some("library")] = model.overview_grid.widget(),

                                            add_named[Some("accounts")] = model.account_view.widget(),

                                            add_named[Some("playtime")] = model.playtime_view.widget(),

                                            add_named[Some("assets")] = model.asset_view.widget(),

                                            add_named[Some("instance")] = &gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_vexpand: true,

                                                // Welcome page (no instance selected)
                                                gtk::Box {
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_valign: gtk::Align::Center,
                                                    set_halign: gtk::Align::Center,
                                                    set_spacing: 24,
                                                    set_vexpand: true,
                                                    #[watch]
                                                    set_visible: model.selected_instance.is_none(),


                                                },

                                                // Instance detail tabs
                                                #[name = "detail_stack"]
                                                adw::ViewStack {
                                                    #[watch]
                                                    set_visible: model.selected_instance.is_some(),
                                                    set_vexpand: true,
                                                    connect_visible_child_name_notify[sender] => move |stack| {
                                                        if let Some(name) = stack.visible_child_name() {
                                                            sender.input(AppMsg::SwitchTab(name.to_string()));
                                                        }
                                                    },

                                                    #[name = "summary_tab"]
                                                    add_titled_with_icon[Some("summary"), "Summary", "go-home-symbolic"] = &adw::Bin {
                                                        #[wrap(Some)]
                                                        set_child = model.instance_summary.widget(),
                                                    },

                                                    #[name = "editor_tab"]
                                                    add_titled_with_icon[Some("editor"), "Editor", "document-edit-symbolic"] = &adw::Bin {
                                                        #[wrap(Some)]
                                                        set_child = model.instance_editor_tab.widget(),
                                                    },

                                                    #[name = "settings_tab"]
                                                    add_titled_with_icon[Some("settings"), "Settings", "emblem-system-symbolic"] = &adw::Bin {
                                                        #[wrap(Some)]
                                                        set_child = model.instance_settings_tab.widget(),
                                                    },

                                                    #[name = "console_tab"]
                                                    add_titled_with_icon[Some("console"), "Console", "utilities-terminal-symbolic"] = &adw::Bin {
                                                        #[wrap(Some)]
                                                        set_child = model.instance_console.widget(),
                                                    },

                                                    // Must come after add_titled_with_icon so children exist on first render
                                                    #[watch]
                                                    set_visible_child_name: if model.active_tab.is_empty() { "summary" } else { &model.active_tab },
                                                }
                                            },

                                            // Must come after add_named so children exist on first render
                                            #[watch]
                                            set_visible_child_name: match model.active_sidebar_page {
                                                SidebarPage::Library => "library",
                                                SidebarPage::Accounts => "accounts",
                                                SidebarPage::Playtime => "playtime",
                                                SidebarPage::Assets => "assets",
                                                SidebarPage::InstanceDetails => "instance",
                                            },
                                        }
                                    },
                                }
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
            },
        );

        let add_instance_dialog = AddInstanceDialog::builder()
            .launch(config.instances_path.clone())
            .forward(sender.input_sender(), |msg| match msg {
                AddInstanceOutput::InstanceCreated(version, path) => AppMsg::InstanceCreated(version, path),
            });

        let instance_editor = InstanceEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::EditorOutput);

        let download_dialog = DownloadDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| match out {
                DownloadDialogOutput::RemoveJob(id) => AppMsg::RemoveJob(id),
                DownloadDialogOutput::ClearFinishedJobs => AppMsg::ClearFinishedJobs,
            });

        let java_selector = JavaSelectorDialog::builder()
            .launch(Some(config.minecraft_data_path.join("java")))
            .forward(sender.input_sender(), |out| match out {
                JavaSelectorOutput::Selected(path) => AppMsg::SetInstanceJava(path),
            });

        let playtime_view = PlaytimeView::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                AppMsg::RefreshPlaytime => AppMsg::RefreshPlaytime,
                _ => AppMsg::RefreshPlaytime, // Fallback
            });

        let shortcuts_dialog = ShortcutsDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |_| unreachable!());

        let component_editor = ComponentEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::ComponentEditorOutput);

        let mod_loader_dialog = ModLoaderDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| AppMsg::ModLoaderOutput(out));

        let asset_view = AssetManagerView::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                AssetOutput::RefreshRequest => AppMsg::RefreshAssets,
            });



        let sharer_dialog = InstanceSharerDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| match out {
                SharerOutput::Generate(idx) => AppMsg::GenerateShareCode(idx),
                SharerOutput::ExportZip(idx, path) => AppMsg::ExportZip(idx, path),
            });

        let import_dialog = ImportDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| match out {
                ImportOutput::Import(code) => AppMsg::ConfirmImportFromCode(code),
                ImportOutput::ImportZip(path) => AppMsg::ImportZip(path),
            });

        // Load instances and groups asynchronously
        let instances: Vec<Instance> = Vec::new();
        let groups = if let Some(path) = &config.instances_path {
            let g = InstanceGroups::load(path);
            let path_clone = path.clone();
            let sender_clone = sender.input_sender().clone();
            thread::spawn(move || {
                let insts = scan_instances(&path_clone);
                let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
            });
            g
        } else {
            InstanceGroups::default()
        };

        let modrinth_browser =
            ModrinthBrowser::builder()
                .launch(())
                .forward(sender.input_sender(), |output| match output {
                    BrowserOutput::InstallMods(installs) => AppMsg::InstallModrinthMods(installs),
                });

        // Build sidebar and overview controllers
        let sidebar = SidebarList::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::SidebarEvent);

        let overview_grid = OverviewGrid::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::OverviewEvent);

        let mut model = AppModel {
            config: config.clone(),
            instances,
            groups,
            selected_instance: None,
            add_instance_dialog,
            instance_editor,
            download_dialog,
            java_selector,
            component_editor,
            mod_loader_dialog,
            modrinth_browser,

            sharer_dialog,
            import_dialog,
            playtime_view,
            shortcuts_dialog,
            sidebar,
            overview_grid,
            asset_view,
            settings_dialog,
            active_sidebar_page: SidebarPage::Library,

            instance_summary: InstanceSummary::builder()
                .launch((None, false))
                .forward(sender.input_sender(), AppMsg::Summary),

            instance_editor_tab: InstanceEditorTab::builder()
                .launch((None, config.clone()))
                .forward(sender.input_sender(), AppMsg::Editor),

            instance_settings_tab: InstanceSettingsTab::builder()
                .launch((None, config.clone()))
                .forward(sender.input_sender(), AppMsg::SettingsTab),

            instance_console: InstanceConsole::builder()
                .launch((gtk::TextBuffer::new(None), false))
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
            loading_instances: true,
            auth_in_progress: false,

            running_instances: HashSet::new(),
            instance_processes: HashMap::new(),
            instance_consoles: HashMap::new(),
            instance_logs: HashMap::new(),
            default_console_buffer: gtk::TextBuffer::new(None),
            active_tab: "summary".to_string(),
            console_filter: LogLevel::Info,
            launch_after_download: false,
            toast_overlay: adw::ToastOverlay::new(),
            active_editor_type: None,
            is_narrow: false,
            overview_layout: LayoutMode::Grid,
            current_folder: None,
            playtime_manager: PlaytimeManager::load(),
            sharing_loading: false,
            import_loading: false,
            verifying_loading: false,
        };

        let widgets = view_output!();

        if let Some(parent) = model.download_status_bar.widget().parent() {
            parent.add_css_class("download-footer");
        }

        // Store reference to the real OverlaySplitView widget
        model.split_view = widgets.split_view.clone();

        widgets.split_view.bind_property("collapsed", &widgets.close_sidebar_btn, "visible")
            .sync_create()
            .build();

        // Single breakpoint at 700sp: collapses sidebar and enables narrow mode.
        // Using one breakpoint avoids dual-breakpoint conflicts that can cause
        // the sidebar to uncollapse when the window shrinks further.
        let bp_condition = adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            550.0,
            adw::LengthUnit::Sp,
        );
        let bp = adw::Breakpoint::new(bp_condition);
        {
            let split = widgets.split_view.clone();
            let sender = sender.clone();
            bp.connect_apply(move |_| {
                split.set_collapsed(true);
                sender.input(AppMsg::SetNarrow(true));
            });
        }
        {
            let split = widgets.split_view.clone();
            let sender = sender.clone();
            bp.connect_unapply(move |_| {
                split.set_collapsed(false);
                sender.input(AppMsg::SetNarrow(false));
            });
        }
        root.add_breakpoint(bp);

        // ── Keyboard Shortcuts ───────────────────────────────────────────
        let shortcut_controller = gtk::ShortcutController::new();
        shortcut_controller.set_scope(gtk::ShortcutScope::Global);

        // Ctrl+,: Settings
        shortcut_controller.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>comma"),
            Some(gtk::CallbackAction::new(glib::clone!(#[strong] sender, move |_, _| {
                sender.input(AppMsg::OpenSettings);
                glib::Propagation::Stop
            }))),
        ));

        // Ctrl+?: Keyboard Shortcuts
        shortcut_controller.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("<Control>question"),
            Some(gtk::CallbackAction::new(glib::clone!(#[strong] sender, move |_, _| {
                sender.input(AppMsg::OpenShortcuts);
                glib::Propagation::Stop
            }))),
        ));

        root.add_controller(shortcut_controller);

        // Set initial stack page
        widgets.detail_stack.set_visible_child_name("summary");


        model.toast_overlay = widgets
            .main_content_box
            .parent()
            .unwrap()
            .downcast::<adw::ToastOverlay>()
            .unwrap();

        // ── Startup token refresh ──────────────────────────────────────────
        // Minecraft access tokens expire after ~24h. If any Microsoft account
        // has a stale token but still has a valid MS refresh token, silently
        // renew all tokens in the background so the UI never briefly shows
        // "Expired" on first load.
        {
            use crate::backend::auth::microsoft::AccountType;
            use crate::backend::auth::account::{
                verify_account_status, AccountStatus, refresh_all_accounts,
            };
            let needs_refresh = config.accounts.iter().any(|a| {
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
                thread::spawn(move || {
                    // Best-effort: ignore errors — individual accounts that
                    // truly fail will still show Expired in the UI after the
                    // next manual refresh.
                    let _ = refresh_all_accounts(&mut config_clone);
                    let _ = sender_clone.send(AppMsg::RefreshAccountsAll(config_clone));
                });
            }
        }

        ComponentParts { model, widgets }
    }
    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            AppMsg::Summary(out) => self.handle_summary_output(out, _sender),
            AppMsg::Editor(out) => self.handle_editor_output(out, _sender),
            AppMsg::SettingsTab(out) => self.handle_settings_output(out, _sender),
            AppMsg::Console(out) => self.handle_console_output(out, _sender),

            // ── Sidebar / overview ────────────────────────────────────────────
            AppMsg::SidebarEvent(out) => match out {
                SidebarOutput::Navigate(page) => {
                    self.active_sidebar_page = page;
                    self.sidebar.emit(SidebarInput::SetSelected(page));
                    
                    match page {
                        SidebarPage::Library => self.selected_instance = None,
                        SidebarPage::Assets => _sender.input(AppMsg::RefreshAssets),
                        SidebarPage::Playtime => _sender.input(AppMsg::RefreshPlaytime),
                        _ => {}
                    }
                }
            },
            AppMsg::OverviewEvent(out) => match out {
                OverviewOutput::SelectInstance(idx) => _sender.input(AppMsg::SelectInstance(idx)),
                OverviewOutput::RenameInstance(idx) => _sender.input(AppMsg::RenameInstanceRequest(idx)),
                OverviewOutput::DeleteInstance(idx) => _sender.input(AppMsg::DeleteInstanceRequest(idx)),
                OverviewOutput::MoveToGroupRequest(idx) => _sender.input(AppMsg::MoveToGroupRequest(idx)),
                OverviewOutput::RemoveFromGroup(idx) => _sender.input(AppMsg::RemoveInstanceFromGroup(idx)),
                OverviewOutput::RenameGroup(name) => _sender.input(AppMsg::RenameGroupRequest(name)),
                OverviewOutput::DeleteGroup(name) => _sender.input(AppMsg::DeleteGroupRequest(name)),
                OverviewOutput::ChangeIconFromFile(idx) => _sender.input(AppMsg::ChangeInstanceIconFromFile(idx)),
                OverviewOutput::ApplyDefaultIcon(idx) => _sender.input(AppMsg::ApplyDefaultIcon(idx)),
                OverviewOutput::ShareInstance(idx) => _sender.input(AppMsg::ShareInstance(idx)),
                OverviewOutput::LayoutModeChanged(mode) => self.overview_layout = mode,
                OverviewOutput::AddInstance => _sender.input(AppMsg::AddInstance),
                OverviewOutput::CreateGroup => _sender.input(AppMsg::CreateGroupRequest),
                OverviewOutput::FolderChanged(folder_opt) => {
                    self.current_folder = folder_opt;
                }
            },
            AppMsg::OverviewBack => {
                self.overview_grid.emit(OverviewInput::GoBack);
            }
            AppMsg::GoBack => {
                if self.active_sidebar_page == SidebarPage::InstanceDetails {
                    _sender.input(AppMsg::ShowOverview);
                } else if self.active_sidebar_page == SidebarPage::Library {
                    _sender.input(AppMsg::OverviewBack);
                }
            }
            AppMsg::ShowOverview => {
                self.active_sidebar_page = SidebarPage::Library;
                self.selected_instance = None;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Library));
            }
            AppMsg::SetNarrow(narrow) => {
                self.is_narrow = narrow;
                self.instance_summary.emit(SummaryInput::SetNarrow(narrow));
                self.overview_grid.emit(OverviewInput::SetNarrow(narrow));
            }
            AppMsg::SetOverviewLayout(mode) => {
                self.overview_layout = mode;
                self.overview_grid.emit(OverviewInput::SetLayoutMode(mode));
            }
            AppMsg::ToggleOverviewLayout => {
                let new_mode = if self.overview_layout == LayoutMode::Grid {
                    LayoutMode::List
                } else {
                    LayoutMode::Grid
                };
                _sender.input(AppMsg::SetOverviewLayout(new_mode));
            }
            AppMsg::ToggleSidebar => {
                let current = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!current);
            }
            AppMsg::CreateGroupRequest => {
                self.show_create_group_dialog_then_move(&_sender, None);
            }
            AppMsg::ConfirmCreateGroup(name) => {
                let name = name.trim().to_string();
                if !name.is_empty() {
                    if let Some(path) = &self.config.instances_path {
                        self.groups.create_group(&name);
                        let _ = self.groups.save(path);
                    }
                    self.rebuild_overview();
                }
            }
            AppMsg::MoveToGroupRequest(idx) => {
                self.show_move_to_group_dialog(&_sender, idx);
            }
            AppMsg::CreateGroupWithMove(idx) => {
                self.show_create_group_dialog_then_move(&_sender, Some(idx));
            }
            AppMsg::MoveInstanceToGroup(idx, group) => {
                if let Some(inst) = self.instances.get(idx) {
                    let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if let Some(path) = &self.config.instances_path {
                        self.groups.set_instance_group(&folder, &group);
                        let _ = self.groups.save(path);
                    }
                    self.rebuild_overview();
                }
            }
            AppMsg::RemoveInstanceFromGroup(idx) => {
                if let Some(inst) = self.instances.get(idx) {
                    let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if let Some(path) = &self.config.instances_path {
                        self.groups.remove_instance_from_groups(&folder);
                        let _ = self.groups.save(path);
                    }
                    self.rebuild_overview();
                }
            }
            AppMsg::RenameGroupRequest(old_name) => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Rename Group")
                    .body("Enter a new name for the group:")
                    .close_response("cancel")
                    .default_response("rename")
                    .build();
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("rename", "Rename");
                dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
                let entry = gtk::Entry::builder()
                    .text(&old_name)
                    .activates_default(true)
                    .build();
                dialog.set_extra_child(Some(&entry));
                let sender_clone = _sender.input_sender().clone();
                let old = old_name.clone();
                dialog.choose(
                    &self.window,
                    None::<&gtk::gio::Cancellable>,
                    move |response| {
                        if response == "rename" {
                            let new_name = entry.text().to_string();
                            sender_clone.send(AppMsg::ConfirmRenameGroup(old.clone(), new_name)).ok();
                        }
                    },
                );
            }
            AppMsg::ConfirmRenameGroup(old_name, new_name) => {
                let new_name = new_name.trim().to_string();
                if !new_name.is_empty() && new_name != old_name {
                    if let Some(path) = &self.config.instances_path {
                        self.groups.rename_group(&old_name, &new_name);
                        let _ = self.groups.save(path);
                    }
                    self.rebuild_overview();
                }
            }
            AppMsg::DeleteGroupRequest(name) => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Delete Group?")
                    .body(format!("Delete group '{}'? Instances will become ungrouped.", name))
                    .close_response("cancel")
                    .default_response("cancel")
                    .build();
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("delete", "Delete");
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
                let sender_clone = _sender.input_sender().clone();
                let name_clone = name.clone();
                dialog.choose(
                    &self.window,
                    None::<&gtk::gio::Cancellable>,
                    move |response| {
                        if response == "delete" {
                            let _ = sender_clone.send(AppMsg::ConfirmDeleteGroup(name_clone.clone()));
                        }
                    },
                );
            }
            AppMsg::ConfirmDeleteGroup(name) => {
                if let Some(path) = &self.config.instances_path {
                    self.groups.delete_group(&name);
                    let _ = self.groups.save(path);
                }
                self.rebuild_overview();
            }
            AppMsg::OpenSettings => {
                self.settings_dialog.emit(SettingsInput::UpdateConfig(self.config.clone()));
                self.settings_dialog.widget().present(Some(&self.window));
            }
            AppMsg::OpenAccountSettings => {
                self.settings_dialog.emit(SettingsInput::UpdateConfig(self.config.clone()));
                self.settings_dialog.emit(SettingsInput::SetPage("accounts".to_string()));
                self.settings_dialog.widget().present(Some(&self.window));
            }
            AppMsg::ShowAddOfflineDialog => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Add Offline Account")
                    .body("Enter a username for offline play.")
                    .close_response("cancel")
                    .default_response("add")
                    .build();

                dialog.add_response("cancel", "Cancel");
                dialog.add_response("add", "Add Account");
                dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);

                let entry = adw::EntryRow::builder()
                    .title("Username")
                    .build();

                let clamp = adw::Clamp::new();
                clamp.set_maximum_size(400);
                clamp.set_margin_start(12);
                clamp.set_margin_end(12);

                let list = gtk::ListBox::new();
                list.set_css_classes(&["boxed-list"]);
                list.set_selection_mode(gtk::SelectionMode::None);
                list.append(&entry);
                clamp.set_child(Some(&list));

                dialog.set_extra_child(Some(&clamp));

                let sender_clone = _sender.input_sender().clone();
                let entry_clone = entry.clone();
                dialog.choose(self.account_view.widget(), None::<&gtk::gio::Cancellable>, move |response| {
                    if response == "add" {
                        let username = entry_clone.text().to_string();
                        let username = username.trim().to_string();
                        if !username.is_empty() {
                            let _ = sender_clone.send(AppMsg::AddOfflineAccount(username));
                        }
                    }
                });
            }
            AppMsg::OpenAbout => {
                let about = adw::AboutDialog::builder()
                    .application_name("Obelisk Launcher")
                    .version("50-rc1")
                    .developer_name("Magnotec")
                    .license_type(gtk::License::Gpl30)
                    .website("https://github.com/magnotec/obelisk-launcher")
                    .issue_url("https://github.com/magnotec/obelisk-launcher/issues")
                    .comments(
                        "A modern Minecraft instance manager built with Rust and GTK4/Libadwaita. Designed around the same format as MultiMC/PolyMC/Prism Launcher, for compatibility.",
                    )
                    .build();
                about.present(Some(&self.window));
            }
            AppMsg::OpenShortcuts => {
                self.shortcuts_dialog.widget().present(Some(&self.window));
            }
            AppMsg::RefreshInstances => {
                self.loading_instances = true;
                self.overview_grid.emit(OverviewInput::SetLoading(true));
                self.active_sidebar_page = SidebarPage::Library;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Library));
                if let Some(path) = &self.config.instances_path {
                    let path_clone = path.clone();
                    let sender_clone = _sender.input_sender().clone();
                    thread::spawn(move || {
                        let insts = scan_instances(&path_clone);
                        let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
                    });
                }
            }
            AppMsg::OpenAssetManager => {
                self.active_sidebar_page = SidebarPage::Assets;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Assets));
                _sender.input(AppMsg::RefreshAssets);
            }
            AppMsg::RefreshAssets => {
                let data_path = self.config.minecraft_data_path.clone();
                let shared_path = self.config.shared_data_path.clone();
                let instances_path = self.config.instances_path.clone();
                self.asset_view.emit(AssetInput::Loading(true));
                
                let sender_clone = _sender.input_sender().clone();
                thread::spawn(move || {
                    let result = crate::backend::download::assets::scan_assets(&data_path, shared_path.as_deref(), instances_path.as_deref());
                    let _ = sender_clone.send(AppMsg::AssetsReady(result));
                });
            }
            AppMsg::AssetsReady(result) => {
                self.asset_view.emit(AssetInput::UpdateData(
                    result,
                    self.config.minecraft_data_path.clone(),
                    self.config.shared_data_path.clone(),
                    self.config.instances_path.clone(),
                ));
            }
            AppMsg::OpenPlaytime => {
                self.active_sidebar_page = SidebarPage::Playtime;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Playtime));
                _sender.input(AppMsg::RefreshPlaytime);
            }
            AppMsg::RefreshPlaytime => {
                let sender_clone = _sender.input_sender().clone();
                let instances = self.instances.clone();
                
                thread::spawn(move || {
                    let manager = crate::backend::playtime::PlaytimeManager::load();
                    let mut instance_data = Vec::new();
                    let mut seen_ids = std::collections::HashSet::new();
                    
                    for inst in &instances {
                        seen_ids.insert(inst.id.clone());
                        let playtime = manager.instance_playtime.get(&inst.id).cloned().unwrap_or(0);
                        instance_data.push((inst.id.clone(), inst.name.clone(), playtime));
                    }
                    
                    // Include instances from history that are no longer on disk
                    for (id, playtime) in &manager.instance_playtime {
                        if !seen_ids.contains(id) {
                            // Use ID as name for missing instances
                            instance_data.push((id.clone(), id.clone(), *playtime));
                        }
                    }
                    
                    let _ = sender_clone.send(AppMsg::PlaytimeDataReady(manager, instance_data));
                });
            }
            AppMsg::PlaytimeDataReady(manager, data) => {
                self.playtime_manager = manager.clone();
                self.playtime_view.emit(PlaytimeInput::UpdateData(manager, data));
            }
            AppMsg::ConfigUpdated(new_config) => {
                self.config = new_config.clone();
                self.account_view
                    .emit(AccountInput::UpdateConfig(new_config.clone()));
                self.instance_editor_tab.emit(EditorTabInput::Update(
                    self.selected_instance
                        .and_then(|i| self.instances.get(i).cloned()),
                    new_config.clone(),
                ));
                self.instance_settings_tab.emit(SettingsTabInput::Update(
                    self.selected_instance
                        .and_then(|i| self.instances.get(i).cloned()),
                    new_config.clone(),
                ));

                self.add_instance_dialog
                    .emit(AddInstanceInput::UpdateInstancesPath(
                        self.config.instances_path.clone(),
                    ));

                if let Some(path) = &self.config.instances_path {
                    let path_clone = path.clone();
                    let sender_clone = _sender.input_sender().clone();
                    self.overview_grid.emit(OverviewInput::SetLoading(true));
                    self.overview_grid.emit(OverviewInput::GoBack);
                    thread::spawn(move || {
                        let insts = scan_instances(&path_clone);
                        let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
                    });
                } else {
                    self.instances.clear();
                    self.selected_instance = None;
                    self.rebuild_overview();
                }
            }
            AppMsg::InstancesUpdated(instances) => {
                self.loading_instances = false;
                self.overview_grid.emit(OverviewInput::SetLoading(false));
                let old_selection = self.selected_instance;
                self.instances = instances;
                self.playtime_manager.ensure_initialized(&self.instances);

                // Re-load groups (they may have changed on disk)
                if let Some(path) = &self.config.instances_path {
                    self.groups = InstanceGroups::load(path);
                }

                self.rebuild_overview();

                if let Some(idx) = old_selection {
                    if idx < self.instances.len() {
                        _sender.input(AppMsg::SelectInstance(idx));
                    } else {
                        self.selected_instance = None;
                        self.active_sidebar_page = SidebarPage::Library;
                        self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Library));
                    }
                } else {
                    self.selected_instance = None;
                }

                if let Some(index) = self.selected_instance {
                    let inst = self.instances.get(index).cloned();
                    let running = self.is_active_instance_running();
                    self.instance_summary
                        .emit(SummaryInput::Update(inst.clone(), running));
                    self.instance_editor_tab
                        .emit(EditorTabInput::Update(inst.clone(), self.config.clone()));
                    self.instance_settings_tab.emit(SettingsTabInput::Update(inst.clone(), self.config.clone()));
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        running,
                    ));
                }
            }

            AppMsg::RefreshSelectedInstance => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let path = inst.path.clone();
                        let sender_clone = _sender.input_sender().clone();
                        thread::spawn(move || {
                            if let Some(updated) = scan_single_instance(&path, true) {
                                let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
                            }
                        });
                    }
                }
            }
            AppMsg::SelectedInstanceUpdated(updated_inst) => {
                // Find and update just this instance in-place — no sidebar rebuild
                if let Some(index) = self.selected_instance {
                    if let Some(existing) = self.instances.get_mut(index) {
                        if existing.path == updated_inst.path {
                            *existing = updated_inst.clone();

                            let running = self.is_active_instance_running();
                            self.instance_summary
                                .emit(SummaryInput::Update(Some(updated_inst.clone()), running));
                            self.instance_editor_tab.emit(EditorTabInput::Update(
                                Some(updated_inst.clone()),
                                self.config.clone(),
                            ));
                            self.instance_settings_tab.emit(SettingsTabInput::Update(
                                Some(updated_inst.clone()),
                                self.config.clone(),
                            ));

                            if let Some(active_type) = &self.active_editor_type {
                                let items = match active_type {
                                    EditorType::Mods => updated_inst.mods.iter().map(|m| EditorItem { id: m.filename.clone(), name: m.name.clone(), version: m.version.clone(), filename: m.filename.clone(), description: m.description.clone(), homepage: m.homepage.clone(), sources: None, icon_path: m.icon_path.clone(), is_checked: false, size: None, seed: None, last_played: None, enabled: m.enabled }).collect(),
                                    EditorType::Components => updated_inst.components.iter().map(|c| EditorItem { id: c.uid.clone(), name: c.name.clone(), version: c.version.clone(), filename: c.uid.clone(), description: None, homepage: None, sources: None, icon_path: None, is_checked: false, size: None, seed: None, last_played: None, enabled: true }).collect(),
                                    EditorType::ResourcePacks => updated_inst.resource_packs.iter().map(|rp| EditorItem {
                                        id: rp.filename.clone(),
                                        name: rp.name.clone(),
                                        version: rp.format.map(|f| format!("Format {}", f)).unwrap_or_default(),
                                        filename: rp.filename.clone(),
                                        description: rp.description.clone(),
                                        homepage: None,
                                        sources: None,
                                        icon_path: rp.icon_path.clone(),
                                        is_checked: false,
                                        size: Some(crate::frontend::utils::format_size(rp.size)),
                                        seed: None,
                                        last_played: None,
                                        enabled: true
                                    }).collect(),
                                    EditorType::ShaderPacks => updated_inst.shader_packs.iter().map(|sp| EditorItem {
                                        id: sp.filename.clone(),
                                        name: sp.name.clone(),
                                        version: String::new(),
                                        filename: sp.filename.clone(),
                                        description: sp.description.clone(),
                                        homepage: None,
                                        sources: None,
                                        icon_path: sp.icon_path.clone(),
                                        is_checked: false,
                                        size: Some(crate::frontend::utils::format_size(sp.size)),
                                        seed: None,
                                        last_played: None,
                                        enabled: true
                                    }).collect(),
                                    EditorType::Worlds => updated_inst.worlds.iter().map(|w| {
                                        let size_str = crate::frontend::utils::format_size(w.file_size);
                                        EditorItem {
                                            id: w.folder_name.clone(),
                                            name: w.name.clone(),
                                            version: w.mc_version.clone().unwrap_or_default(),
                                            filename: w.folder_name.clone(),
                                            description: None,
                                            homepage: None,
                                            sources: None,
                                            icon_path: None,
                                            is_checked: false,
                                            size: Some(size_str),
                                            seed: w.seed.map(|s| s.to_string()),
                                            last_played: w.last_played.map(crate::frontend::utils::format_timestamp),
                                            enabled: true,
                                        }
                                    }).collect(),
                                };
                                self.instance_editor.emit(EditorInput::UpdateItems(items));
                            }
                        }
                    }
                }
                // Fallback: path didn't match (shouldn't happen), do full update
                for (i, inst) in self.instances.iter_mut().enumerate() {
                    if inst.path == updated_inst.path {
                        *inst = updated_inst.clone();
                        if self.selected_instance == Some(i) {
                            let running = self.is_active_instance_running();
                            self.instance_summary
                                .emit(SummaryInput::Update(Some(updated_inst.clone()), running));
                            self.instance_editor_tab.emit(EditorTabInput::Update(
                                Some(updated_inst.clone()),
                                self.config.clone(),
                            ));
                            self.instance_settings_tab.emit(SettingsTabInput::Update(
                                Some(updated_inst),
                                self.config.clone(),
                            ));
                        }
                        break;
                    }
                }
                self.rebuild_overview();
            }
            AppMsg::SelectInstance(index) => {
                self.selected_instance = Some(index);
                self.active_sidebar_page = SidebarPage::InstanceDetails;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::InstanceDetails));
                let inst_opt = self.instances.get(index).cloned();
                let running = self.is_active_instance_running();

                self.instance_summary
                    .emit(SummaryInput::Update(inst_opt.clone(), running));
                self.instance_summary.emit(SummaryInput::SetSharingLoading(self.sharing_loading));
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(self.verifying_loading));
                self.instance_editor_tab.emit(EditorTabInput::Update(
                    inst_opt.clone(),
                    self.config.clone(),
                ));
                self.instance_settings_tab.emit(SettingsTabInput::Update(
                    inst_opt.clone(),
                    self.config.clone(),
                ));
                self.instance_console.emit(ConsoleInput::Update(
                    self.get_active_console_buffer(),
                    running,
                ));

                if let Some(_inst) = inst_opt {
                    _sender.input(AppMsg::RefreshSelectedInstance);
                }
            }
            AppMsg::AddInstance => {
                self.add_instance_dialog.emit(AddInstanceInput::Open);
                self.add_instance_dialog.widget().present(Some(&self.window));
            }
            AppMsg::ShareInstance(idx) => {
                self.sharer_dialog.emit(SharerInput::Open(idx));
                self.sharer_dialog.widget().present(Some(&self.window));
            }
            AppMsg::GenerateShareCode(idx) => {
                if let Some(inst) = self.instances.get(idx) {
                    let sender_clone = _sender.input_sender().clone();
                    let inst_clone = inst.clone();
                    self.sharing_loading = true;
                    // Note: sharing_loading still used for summary grey-out
                    std::thread::spawn(move || {
                        match export_instance(&inst_clone) {
                            Ok(shared) => {
                                if let Ok(code) = shared.to_code() {
                                    let _ = sender_clone.send(AppMsg::DisplayShareCode(code));
                                } else {
                                    let _ = sender_clone.send(AppMsg::DownloadError("Failed to encode instance".to_string()));
                                }
                            }
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::DownloadError(format!("Export failed: {}", e)));
                            }
                        }
                        let _ = sender_clone.send(AppMsg::SetSharingLoading(false, String::new(), String::new(), false));
                    });
                }
            }
            AppMsg::ExportZip(idx, path) => {
                if let Some(inst) = self.instances.get(idx) {
                    let sender_clone = _sender.input_sender().clone();
                    let inst_clone = inst.clone();
                    self.sharing_loading = true;
                    std::thread::spawn(move || {
                        let s_clone = sender_clone.clone();
                        match export_instance_to_zip(&inst_clone, &path, move |p, s| {
                            let _ = s_clone.send(AppMsg::UpdateSharingProgress(p, s));
                        }) {
                            Ok(_) => {}
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::DownloadError(format!("Zip export failed: {}", e)));
                            }
                        }
                        let _ = sender_clone.send(AppMsg::SetSharingLoading(false, String::new(), String::new(), false));
                    });
                }
            }
            AppMsg::DisplayShareCode(code) => {
                self.sharer_dialog.emit(SharerInput::Close);
                
                let entry = gtk::Entry::builder()
                    .text(&code)
                    .editable(false)
                    .halign(gtk::Align::Center)
                    .width_request(300)
                    .css_classes(vec!["body".to_string()])
                    .build();

                let dialog = adw::AlertDialog::builder()
                    .heading("Instance Code Ready")
                    .body("Share this code with friends to let them import your exact setup.")
                    .extra_child(&entry)
                    .close_response("close")
                    .default_response("copy")
                    .build();

                dialog.add_response("close", "Close");
                dialog.add_response("copy", "Copy to Clipboard");
                dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);

                let code_clone = code.clone();
                dialog.choose(&self.window, None::<&gtk::gio::Cancellable>, move |resp| {
                    if resp == "copy" {
                        let display = gtk::gdk::Display::default().expect("Could not get default display");
                        let clipboard = display.clipboard();
                        clipboard.set_text(&code_clone);
                    }
                });

                entry.select_region(0, -1);
            }
            AppMsg::ImportRequest => {
                self.import_dialog.emit(ImportInput::Open);
                self.import_dialog.widget().present(Some(&self.window));
            }
            AppMsg::ConfirmImportFromCode(code) => {
                if let Ok(shared) = SharedInstance::from_code(&code) {
                    let folder_name = shared.name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect::<String>();
                    if let Some(instances_path) = self.config.instances_path.clone() {
                        let instance_dir = instances_path.join(&folder_name);
                        if instance_dir.exists() {
                            let entry = gtk::Entry::builder()
                                .text(&shared.name)
                                .placeholder_text("New instance name")
                                .halign(gtk::Align::Center)
                                .width_request(300)
                                .build();
                                
                            let dialog = adw::AlertDialog::builder()
                                .heading("Instance Already Exists")
                                .body(format!("A folder for '{}' already exists at your instances path. Please provide a new name for this import.", shared.name))
                                .extra_child(&entry)
                                .close_response("cancel")
                                .default_response("import")
                                .build();
                            dialog.add_response("cancel", "Cancel");
                            dialog.add_response("import", "Import with New Name");
                            dialog.set_response_appearance("import", adw::ResponseAppearance::Suggested);
                            
                            let sender_clone = _sender.input_sender().clone();
                            let code_clone = code.clone();
                            dialog.choose(self.import_dialog.widget(), None::<&gtk::gio::Cancellable>, move |resp| {
                                if resp == "import" {
                                    let new_name = entry.text().to_string();
                                    if !new_name.is_empty() {
                                        let _ = sender_clone.send(AppMsg::PerformImport(code_clone, Some(new_name)));
                                    } else {
                                        let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                                    }
                                } else {
                                    let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                                }
                            });
                            return;
                        }
                    }
                }
                _sender.input(AppMsg::PerformImport(code, None));
            }
            AppMsg::PerformImport(code, new_name) => {
                self.import_loading = true;
                if let Ok(mut shared) = SharedInstance::from_code(&code) {
                    if let Some(name) = new_name {
                        shared.name = name;
                    }
                    if let Some(instances_path) = self.config.instances_path.clone() {
                        let sender_clone = _sender.input_sender().clone();
                        std::thread::spawn(move || {
                            let s_clone = sender_clone.clone();
                            match import_shared_instance(shared, &instances_path, move |s| {
                                let _ = s_clone.send(AppMsg::UpdateImportStatus(s));
                            }) {
                                Ok(_) => {
                                    let _ = sender_clone.send(AppMsg::RefreshInstances);
                                    let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                                }
                                Err(e) => {
                                    let _ = sender_clone.send(AppMsg::DownloadError(format!("Import failed: {}", e)));
                                    let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                                }
                            }
                        });
                    } else {
                        self.import_loading = false;
                        self.import_dialog.emit(ImportInput::Close);
                    }
                } else {
                    self.import_loading = false;
                    self.toast_overlay.add_toast(adw::Toast::new("Invalid sharing code"));
                    self.import_dialog.emit(ImportInput::Close);
                }
            }
            AppMsg::ImportZip(path) => {
                if let Some(instances_path) = self.config.instances_path.clone() {
                    let sender_clone = _sender.input_sender().clone();
                    self.import_loading = true;
                    self.import_dialog.emit(ImportInput::SetStep(ImportStep::Progress));
                    std::thread::spawn(move || {
                        let s_clone = sender_clone.clone();
                        match import_instance_from_zip(&path, &instances_path, move |p, s| {
                            let _ = s_clone.send(AppMsg::UpdateImportStatus(format!("{:.0}% - {}", p * 100.0, s)));
                        }) {
                            Ok(_) => {
                                let _ = sender_clone.send(AppMsg::RefreshInstances);
                                let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                            }
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::DownloadError(format!("Zip import failed: {}", e)));
                                let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                            }
                        }
                    });
                }
            }
            AppMsg::SetSharingLoading(loading, title, subtitle, show_progress) => {
                self.sharing_loading = loading;
                self.instance_summary.emit(SummaryInput::SetSharingLoading(loading));
                self.sharer_dialog.emit(SharerInput::SetLoading(loading, title, subtitle, show_progress));
                if !loading {
                    self.sharer_dialog.emit(SharerInput::Close);
                }
            }
            AppMsg::UpdateSharingProgress(p, s) => {
                self.sharer_dialog.emit(SharerInput::SetProgress(p));
                self.sharer_dialog.emit(SharerInput::SetLoading(true, "Exporting Zip".to_string(), s, true));
            }
            AppMsg::SetImportLoading(loading) => {
                self.import_loading = loading;
                if !loading {
                    self.import_dialog.emit(ImportInput::SetLoading(false));
                    self.import_dialog.emit(ImportInput::Close);
                }
            }
            AppMsg::SetVerifyingLoading(loading) => {
                self.verifying_loading = loading;
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(loading));
            }
            AppMsg::UpdateImportStatus(status) => {
                self.import_dialog.emit(ImportInput::AddLog(status));
            }
            // Old RefreshInstances handler removed — handled above at line 1411
            AppMsg::EditComponents => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.active_editor_type = Some(EditorType::Components);
                    let items = inst
                        .components
                        .iter()
                        .map(|c| EditorItem {
                            id: c.uid.clone(),
                            name: c.name.clone(),
                            version: c.version.clone(),
                            filename: c.uid.clone(),
                            description: None,
                            homepage: None,
                            sources: None,
                            icon_path: None,
                            is_checked: false,
                            size: None,
                            seed: None,
                            last_played: None,
                            enabled: true,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Components,
                        "Edit Components".to_string(),
                        items,
                    ));
                    self.instance_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::EditMods => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.active_editor_type = Some(EditorType::Mods);
                    let items = inst
                        .mods
                        .iter()
                        .map(|m| EditorItem {
                            id: m.filename.clone(),
                            name: m.name.clone(),
                            version: m.version.clone(),
                            filename: m.filename.clone(),
                            description: m.description.clone(),
                            homepage: m.homepage.clone(),
                            sources: None,
                            icon_path: m.icon_path.clone(),
                            is_checked: false,
                            size: None,
                            seed: None,
                            last_played: None,
                            enabled: m.enabled,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Mods,
                        "Edit Mods".to_string(),
                        items,
                    ));
                    self.instance_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::EditResourcePacks => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.active_editor_type = Some(EditorType::ResourcePacks);
                    let items = inst
                        .resource_packs
                        .iter()
                        .map(|rp| EditorItem {
                            id: rp.filename.clone(),
                            name: rp.name.clone(),
                            version: rp.format.map(|f| format!("Format {}", f)).unwrap_or_default(),
                            filename: rp.filename.clone(),
                            description: rp.description.clone(),
                            homepage: None,
                            sources: None,
                            icon_path: rp.icon_path.clone(),
                            is_checked: false,
                            size: Some(crate::frontend::utils::format_size(rp.size)),
                            seed: None,
                            last_played: None,
                            enabled: true,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::ResourcePacks,
                        "Edit Resource Packs".to_string(),
                        items,
                    ));
                    self.instance_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::EditShaderPacks => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.active_editor_type = Some(EditorType::ShaderPacks);
                    let items = inst
                        .shader_packs
                        .iter()
                        .map(|sp| EditorItem {
                            id: sp.filename.clone(),
                            name: sp.name.clone(),
                            version: String::new(),
                            filename: sp.filename.clone(),
                            description: sp.description.clone(),
                            homepage: None,
                            sources: None,
                            icon_path: sp.icon_path.clone(),
                            is_checked: false,
                            size: Some(crate::frontend::utils::format_size(sp.size)),
                            seed: None,
                            last_played: None,
                            enabled: true,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::ShaderPacks,
                        "Edit Shader Packs".to_string(),
                        items,
                    ));
                    self.instance_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::EditWorlds => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.active_editor_type = Some(EditorType::Worlds);
                    let items = inst
                        .worlds
                        .iter()
                        .map(|w| {
                            let size_str = crate::frontend::utils::format_size(w.file_size);
                            EditorItem {
                                id: w.folder_name.clone(),
                                name: w.name.clone(),
                                version: w.mc_version.clone().unwrap_or_default(),
                                filename: w.folder_name.clone(),
                                description: None,
                                homepage: None,
                                sources: None,
                                icon_path: None,
                                is_checked: false,
                                size: Some(size_str),
                                seed: w.seed.map(|s| s.to_string()),
                                last_played: w.last_played.map(crate::frontend::utils::format_timestamp),
                                enabled: true,
                            }
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Worlds,
                        "Edit Worlds".to_string(),
                        items,
                    ));
                    self.instance_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::EditorOutput(output) => {
                println!("AppMsg::EditorOutput received: {:?}", output);
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index).cloned() {
                        match output {
                            EditorOutput::RemoveMods(filenames) => {
                                for filename in &filenames {
                                    if let Err(e) =
                                        remove_instance_item(&inst.path, "mods", filename)
                                    {
                                        eprintln!("Failed to remove mod {}: {}", filename, e);
                                    }
                                }
                            }
                            EditorOutput::RemoveComponents(uids) => {
                                for uid in &uids {
                                    if let Err(e) = remove_component(&inst.path, uid) {
                                        eprintln!("Failed to remove component {}: {}", uid, e);
                                    }
                                }
                            }
                            EditorOutput::RemoveResourcePacks(filenames) => {
                                for f in filenames {
                                    if let Err(e) =
                                        remove_instance_item(&inst.path, "resourcepacks", &f)
                                    {
                                        eprintln!("Failed to remove resource pack {}: {}", f, e);
                                    }
                                }
                            }
                            EditorOutput::RemoveShaderPacks(filenames) => {
                                for f in filenames {
                                    if let Err(e) =
                                        remove_instance_item(&inst.path, "shaderpacks", &f)
                                    {
                                        eprintln!("Failed to remove shader pack {}: {}", f, e);
                                    }
                                }
                            }
                            EditorOutput::RemoveWorlds(folders) => {
                                for f in folders {
                                    if let Err(e) = remove_instance_item(&inst.path, "saves", &f) {
                                        eprintln!("Failed to remove world {}: {}", f, e);
                                    }
                                }
                            }
                            EditorOutput::SetModsEnabled(filenames, enable) => {
                                let mut changed = false;
                                if let Some(existing) = self.instances.get_mut(index) {
                                    for filename in filenames {
                                        if let Ok(new_filename) = crate::backend::instance::manager::toggle_mod_enabled(&existing.path, &filename, enable) {
                                            if let Some(m) = existing.mods.iter_mut().find(|m| m.filename == filename) {
                                                m.filename = new_filename;
                                                m.enabled = enable;
                                                changed = true;
                                            }
                                        }
                                    }
                                }
                                if changed {
                                    let updated = self.instances[index].clone();
                                    _sender.input(AppMsg::SelectedInstanceUpdated(updated));
                                }
                            }
                            EditorOutput::AddItems(editor_type, paths) => {
                                let subfolder = match editor_type {
                                    EditorType::Mods => "mods",
                                    EditorType::ResourcePacks => "resourcepacks",
                                    EditorType::ShaderPacks => "shaderpacks",
                                    EditorType::Worlds => "saves",
                                    _ => "mods",
                                };
                                for p in paths {
                                    let _ = add_instance_item(&inst.path, subfolder, &p);
                                }
                            }
                            EditorOutput::OpenFolder(editor_type) => {
                                let subfolder = match editor_type {
                                    EditorType::Mods => "mods",
                                    EditorType::ResourcePacks => "resourcepacks",
                                    EditorType::ShaderPacks => "shaderpacks",
                                    EditorType::Worlds => "saves",
                                    _ => "",
                                };
                                if !subfolder.is_empty() {
                                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, subfolder);
                                }
                            }
                            EditorOutput::BrowseModrinth(editor_type) => {
                                _sender.input(AppMsg::BrowseModrinth(editor_type));
                            }
                            EditorOutput::RenameWorld(folder, new_name) => {
                                if let Err(e) = crate::backend::instance::manager::rename_world(&inst.path, &folder, &new_name) {
                                    eprintln!("Failed to rename world: {}", e);
                                }
                            }
                            EditorOutput::MoveItems(editor_type, ids) => {
                                self.show_target_instance_selector(editor_type, ids, false, _sender.input_sender().clone());
                            }
                            EditorOutput::CopyItems(editor_type, ids) => {
                                self.show_target_instance_selector(editor_type, ids, true, _sender.input_sender().clone());
                            }
                        }
                        // Refresh only the selected instance
                        _sender.input(AppMsg::RefreshSelectedInstance);
                        // Reselect - inline to avoid ambiguous self.update call
                        self.selected_instance = Some(index);
                    }
                }
            }
            AppMsg::ConfirmMoveItems(editor_type, ids, target_idx) => {
                if let Some(source_idx) = self.selected_instance {
                    let source_inst = &self.instances[source_idx];
                    let target_inst = &self.instances[target_idx];
                    
                    let subfolder = match editor_type {
                        EditorType::Mods => "mods",
                        EditorType::ResourcePacks => "resourcepacks",
                        EditorType::ShaderPacks => "shaderpacks",
                        EditorType::Worlds => "saves",
                        _ => "mods",
                    };
                    
                    for id in ids {
                        let source_path = source_inst.minecraft_dir.join(subfolder).join(&id);
                        if let Err(e) = crate::backend::instance::manager::add_instance_item(&target_inst.path, subfolder, &source_path) {
                            eprintln!("Failed to copy item for move: {}", e);
                        } else {
                            // Remove from source after successful copy
                            if let Err(e) = crate::backend::instance::manager::remove_instance_item(&source_inst.path, subfolder, &id) {
                                eprintln!("Failed to remove source item after move: {}", e);
                            }
                        }
                    }
                    _sender.input(AppMsg::RefreshInstances);
                }
            }
            AppMsg::ConfirmCopyItems(editor_type, ids, target_idx) => {
                if let Some(source_idx) = self.selected_instance {
                    let source_inst = &self.instances[source_idx];
                    let target_inst = &self.instances[target_idx];
                    
                    let subfolder = match editor_type {
                        EditorType::Mods => "mods",
                        EditorType::ResourcePacks => "resourcepacks",
                        EditorType::ShaderPacks => "shaderpacks",
                        EditorType::Worlds => "saves",
                        _ => "mods",
                    };
                    
                    for id in ids {
                        let source_path = source_inst.minecraft_dir.join(subfolder).join(&id);
                        if let Err(e) = crate::backend::instance::manager::add_instance_item(&target_inst.path, subfolder, &source_path) {
                            eprintln!("Failed to copy item: {}", e);
                        }
                    }
                    _sender.input(AppMsg::RefreshInstances);
                }
            }
            AppMsg::OpenModsFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "mods");
                }
            }
            AppMsg::OpenResourcePacksFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "resourcepacks");
                }
            }
            AppMsg::OpenShaderPacksFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "shaderpacks");
                }
            }
            AppMsg::OpenScreenshotsFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "screenshots");
                }
            }
            AppMsg::OpenWorldsFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.minecraft_dir, "saves");
                }
            }
            AppMsg::OpenInstanceFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    use std::process::Command;
                    let _ = Command::new("xdg-open").arg(&inst.path).spawn();
                }
            }
            AppMsg::BrowseModrinth(_editor_type) => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    let (loader, _) = inst.get_loader_info();
                    self.modrinth_browser.emit(BrowserInput::Open(
                        inst.minecraft_version.clone().unwrap_or_default(),
                        loader,
                    ));
                    self.modrinth_browser.widget().present(Some(&self.window));
                }
            }
            AppMsg::InstallModrinthMods(installs) => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let gv = inst
                            .minecraft_version
                            .clone()
                            .unwrap_or_else(|| "1.20.1".to_string());
                        let (loader, _) = inst.get_loader_info();

                        let minecraft_dir = inst.minecraft_dir.clone();
                        let sender_clone = _sender.input_sender().clone();
                        
                        let mods_dir = minecraft_dir.join("mods");
                        if !mods_dir.exists() {
                            let _ = std::fs::create_dir_all(&mods_dir);
                        }

                        // Only show non-intrusive status bar starting progress
                        self.download_status_bar
                            .emit(DownloadStatusBarInput::Update(
                                DownloadState::Starting,
                                true,
                            ));

                        let mut tasks = Vec::new();
                        let installs_len = installs.len();
                        for (project_id, version_id) in installs {
                            tasks.push(crate::backend::download::manager::NetworkTask::ModrinthDownload {
                                project_id,
                                version_id: if version_id.is_empty() { None } else { Some(version_id) },
                                game_version: gv.clone(),
                                loader: loader.clone(),
                                mods_dir: mods_dir.clone(),
                            });
                        }

                        let job = crate::backend::download::manager::NetworkJob {
                            id: format!("mods-{}", uuid::Uuid::new_v4()),
                            title: format!("Mods for {}", inst.name),
                            tasks,
                            status: crate::backend::download::manager::NetworkJobStatus::Pending,
                            log: Vec::new(),
                        };

                        let (tx, rx) = std::sync::mpsc::channel::<crate::backend::download::manager::DownloadMsg>();
                        
                        // Queue the job in DOWNLOAD_QUEUE
                        crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);

                        // Spawn a thread to forward messages to AppMsg::DownloadProgress
                        thread::spawn(move || {
                            let mut success_count = 0;
                            let mut last_error = None;
                            
                            while let Ok(msg) = rx.recv() {
                                let is_finished = matches!(
                                    msg,
                                    crate::backend::download::manager::DownloadMsg::Finished
                                        | crate::backend::download::manager::DownloadMsg::Error(_)
                                );
                                
                                match &msg {
                                    crate::backend::download::manager::DownloadMsg::Finished => {
                                        success_count = installs_len; // treat all as succeeded on completion
                                    }
                                    crate::backend::download::manager::DownloadMsg::Error(err) => {
                                        last_error = Some(err.clone());
                                    }
                                    _ => {}
                                }
                                
                                let app_msg = AppMsg::DownloadProgress(msg);
                                if sender_clone.send(app_msg).is_err() {
                                    break;
                                }
                                if is_finished {
                                    break;
                                }
                            }
                            
                            // Send installation results and refreshes after job completes/fails
                            if success_count > 0 {
                                sender_clone.send(AppMsg::ModrinthInstallResult(Ok(success_count))).ok();
                                sender_clone.send(AppMsg::RefreshSelectedInstance).ok();
                            } else if let Some(e) = last_error {
                                sender_clone.send(AppMsg::ModrinthInstallResult(Err(e))).ok();
                            }
                        });
                    }
                }
            }
            AppMsg::ModrinthInstallResult(result) => match result {
                Ok(count) => {
                    let msg = if count == 1 {
                        "Successfully installed 1 mod".to_string()
                    } else {
                        format!("Successfully installed {} mods", count)
                    };
                    self.instance_editor.emit(EditorInput::ShowToast(msg));
                }
                Err(e) => {
                    self.instance_editor.emit(EditorInput::ShowToast(format!("Installation failed: {}", e)));
                }
            },
            AppMsg::RenameInstanceRequest(index) => {
                if let Some(inst) = self.instances.get(index) {
                    let current_name = inst.name.clone();
                    let dialog = adw::AlertDialog::builder()
                        .heading("Rename Instance")
                        .body("Enter a new name for the instance:")
                        .close_response("cancel")
                        .default_response("rename")
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("rename", "Rename");
                    dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);

                    let entry = gtk::Entry::builder()
                        .text(&current_name)
                        .activates_default(true)
                        .build();
                    dialog.set_extra_child(Some(&entry));

                    let sender_clone = _sender.input_sender().clone();
                    dialog.choose(
                        &self.window,
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "rename" {
                                let new_name = entry.text().to_string();
                                sender_clone.send(AppMsg::ConfirmRename(index, new_name)).ok();
                            }
                        },
                    );
                }
            }
            AppMsg::DeleteInstanceRequest(index) => {
                if let Some(inst) = self.instances.get(index) {
                    let name = inst.name.clone();
                    let dialog = adw::AlertDialog::builder()
                        .heading("Delete Instance?")
                        .body(format!("This will permanently remove '{}' and all its files. This cannot be undone.", name))
                        .close_response("cancel")
                        .default_response("cancel")
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("delete", "Delete");
                    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
 
                    let sender_clone = _sender.input_sender().clone();
                    dialog.choose(
                        &self.window,
                        None::<&gtk::gio::Cancellable>,
                        move |response| {
                            if response == "delete" {
                                sender_clone.send(AppMsg::ConfirmDelete(index)).ok();
                            }
                        },
                    );
                }
            }

            AppMsg::ChangeInstanceIconFromFile(idx) => {
                if let Some(_inst) = self.instances.get(idx) {
                    let s = _sender.input_sender().clone();
                    
                    let dialog = gtk::FileDialog::builder()
                        .title("Select Instance Icon")
                        .accept_label("Select")
                        .modal(true)
                        .build();

                    let filters = gtk::FileFilter::new();
                    filters.add_mime_type("image/png");
                    filters.add_mime_type("image/jpeg");
                    filters.set_name(Some("Images"));
                    let list_store = gtk::gio::ListStore::new::<gtk::FileFilter>();
                    list_store.append(&filters);
                    dialog.set_filters(Some(&list_store));

                    dialog.open(Some(&self.window), None::<&gtk::gio::Cancellable>, move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                let _ = s.send(AppMsg::ApplyIconPath(idx, path));
                            }
                        }
                    });
                }
            }
            AppMsg::ApplyDefaultIcon(idx) => {
                if let Some(default_path) = self.config.default_instance_icon.clone() {
                    _sender.input(AppMsg::ApplyIconPath(idx, default_path));
                }
            }
            AppMsg::ApplyIconPath(idx, source_path) => {
                if let Some(inst) = self.instances.get(idx) {
                    let inst_path = inst.path.clone();
                    let target = inst_path.join("icon.png");
                    if std::fs::copy(&source_path, &target).is_ok() {
                        self.overview_grid.emit(OverviewInput::ClearTextureCache(target));
                        let _ = crate::backend::instance::manager::update_cfg_key(&inst_path, "iconKey", "custom");
                        // Targeted refresh instead of full scan
                        let sender_clone = _sender.input_sender().clone();
                        thread::spawn(move || {
                            if let Some(updated) = scan_single_instance(&inst_path, true) {
                                let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
                                let _ = sender_clone.send(AppMsg::RefreshInstances);
                            }
                        });
                    }
                }
            }
            AppMsg::ConfirmRename(index, new_name) => {
                let new_name = new_name.trim().to_string();
                if !new_name.is_empty() {
                    if let Some(inst) = self.instances.get_mut(index) {
                        let _ = rename_instance(&inst.path, &new_name);
                        inst.name = new_name;
                        self.instances.sort_by(|a, b| a.name.cmp(&b.name));
                        self.rebuild_overview();
                    }
                }
            }
            AppMsg::ConfirmDelete(index) => {
                if let Some(inst) = self.instances.get(index) {
                    let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    let _ = delete_instance(&inst.path);
                    // Remove from groups
                    if let Some(path) = &self.config.instances_path {
                        self.groups.remove_instance_from_groups(&folder);
                        let _ = self.groups.save(path);
                    }
                }
                if index < self.instances.len() {
                    self.instances.remove(index);
                }
                self.selected_instance = None;
                self.active_sidebar_page = SidebarPage::Library;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Library));
                self.rebuild_overview();
            }
            AppMsg::AccountAction => {
                self.active_sidebar_page = SidebarPage::Accounts;
                self.sidebar.emit(SidebarInput::SetSelected(SidebarPage::Accounts));
            }
            AppMsg::SwitchAccount(uuid) => {
                if let Ok(_) = switch_account(&mut self.config, &uuid) {
                    let _ = self.config.save();
                    self.account_view
                        .emit(AccountInput::UpdateConfig(self.config.clone()));
                        self.toast_overlay
                        .add_toast(adw::Toast::new("Switched account"));
                }
            }
            AppMsg::RemoveAccount(uuid) => {
                remove_account(&mut self.config, &uuid);
                let _ = self.config.save();
                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
                self.toast_overlay
                    .add_toast(adw::Toast::new("Account removed"));
            }
            AppMsg::AddOfflineAccount(username) => {
                let account = create_offline_account(&username);
                let name = account.username.clone();
                add_account(&mut self.config, account);
                let _ = self.config.save();
                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
                self.toast_overlay
                    .add_toast(adw::Toast::new(&format!("Added offline account: {}", name)));
            }
            AppMsg::VerifyAccount(_) => {}
            AppMsg::VerifyAccountResult(_, _) => {}
            AppMsg::RefreshAccount(_) => {}
            AppMsg::RefreshAccountResult(result) => {
                if let Ok(account) = result {
                    add_account(&mut self.config, account);
                    let _ = self.config.save();
                    self.account_view
                        .emit(AccountInput::UpdateConfig(self.config.clone()));
                    }
            }
            AppMsg::RefreshAccountsRequest => {
                self.account_view.emit(AccountInput::RefreshAll);
            }
            AppMsg::RefreshAccountsAll(new_config) => {
                // Replace accounts with the freshly-refreshed set
                self.config.accounts = new_config.accounts;
                let _ = self.config.save();
                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
            }
            AppMsg::OpenJavaSelector => {
                self.java_selector.emit(JavaSelectorInput::Open);
                self.java_selector.widget().present(Some(&self.window));
            }
            AppMsg::SetInstanceJava(path) => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let _ = set_instance_java(&inst.path, &path);
                        _sender.input(AppMsg::RefreshSelectedInstance);
                    }
                }
            }
            AppMsg::SetInstanceJavaDefault => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        let _ = crate::backend::instance::manager::remove_instance_java(&inst.path);
                        _sender.input(AppMsg::RefreshSelectedInstance);
                    }
                }
            }
            AppMsg::OpenComponentSwap(uid) => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    let current_version = inst
                        .components
                        .iter()
                        .find(|c| c.uid == uid)
                        .map(|c| c.version.clone());

                    self.component_editor.emit(ComponentEditorInput::Open(
                        uid,
                        inst.minecraft_version.clone(),
                        current_version,
                    ));
                    self.component_editor.widget().present(Some(&self.window));
                }
            }
            AppMsg::RemoveComponent(uid) => {
                if let Some(index) = self.selected_instance {
                    if let Some(inst) = self.instances.get(index) {
                        // Check if it's a mod loader component group
                        if is_loader_component(&uid) {
                            let _ = remove_mod_loader(&inst.path);
                        } else {
                            let _ = remove_component(&inst.path, &uid);
                        }
                        _sender.input(AppMsg::RefreshSelectedInstance);
                    }
                }
            }
            AppMsg::SelectModLoaderRequest => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    self.mod_loader_dialog.emit(ModLoaderDialogInput::Open(
                        inst.minecraft_version.clone(),
                    ));
                    self.mod_loader_dialog.widget().present(Some(&self.window));
                }
            }
            AppMsg::ModLoaderOutput(output) => match output {
                ModLoaderDialogOutput::InstallModLoader(loader, version) => {
                    if let Some(index) = self.selected_instance {
                        if let Some(inst) = self.instances.get(index) {
                            let _ = set_mod_loader_with_version(&inst.path, &loader, &version);
                            _sender.input(AppMsg::RefreshSelectedInstance);
                        }
                    }
                }
            },
            AppMsg::ComponentEditorOutput(output) => match output {
                ComponentEditorOutput::SetVersion(uid, version) => {
                    if let Some(index) = self.selected_instance {
                        if let Some(inst) = self.instances.get(index) {
                            let _ = set_component_version(&inst.path, &uid, &version);
                            _sender.input(AppMsg::RefreshSelectedInstance);
                        }
                    }
                }
            },
            AppMsg::LoginStart => {
                if let Some(client_id) = self.config.microsoft_client_id.clone() {
                    self.auth_in_progress = true;
                    let sender_clone = _sender.input_sender().clone();

                    thread::spawn(move || {
                        match auth::start_device_code_flow(&client_id) {
                            Ok(dc) => {
                                let _ = sender_clone.send(AppMsg::LoginDeviceCode(
                                    dc.user_code.clone(),
                                    dc.verification_uri.clone(),
                                ));

                                // Open browser
                                let _ = std::process::Command::new("xdg-open")
                                    .arg(&dc.verification_uri)
                                    .spawn();

                                // Poll for MS token (blocks until user completes or timeout)
                                match auth::poll_for_ms_token(
                                    &client_id,
                                    &dc.device_code,
                                    dc.interval,
                                    dc.expires_in,
                                ) {
                                    Ok((access_token, refresh_token)) => {
                                        match auth::complete_auth(&access_token, &refresh_token) {
                                            Ok(account) => {
                                                let _ = sender_clone
                                                    .send(AppMsg::LoginResult(Ok(account)));
                                            }
                                            Err(e) => {
                                                let _ =
                                                    sender_clone.send(AppMsg::LoginResult(Err(e)));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = sender_clone.send(AppMsg::LoginResult(Err(e)));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::LoginResult(Err(e)));
                            }
                        }
                    });
                } else {
                    let dialog = adw::AlertDialog::builder()
                        .heading("No Client ID")
                        .body("Please set your Microsoft Azure Client ID in the app settings before signing in.")
                        .close_response("ok")
                        .default_response("ok")
                        .build();
                    dialog.add_response("ok", "OK");
                    dialog.choose(self.account_view.widget(), None::<&gtk::gio::Cancellable>, |_| {});
                }
            }
            AppMsg::LoginDeviceCode(code, uri) => {
                let entry = gtk::Entry::builder()
                    .text(&code)
                    .editable(false)
                    .halign(gtk::Align::Center)
                    .width_request(200)
                    .css_classes(vec!["title-2".to_string()])
                    .build();

                let dialog = adw::AlertDialog::builder()
                    .heading("Sign in with Microsoft")
                    .body(format!(
                        "A browser window has been opened.\n\nGo to:\n{}\n\nAnd enter this code below:",
                        uri
                    ))
                    .extra_child(&entry)
                    .close_response("ok")
                    .default_response("ok")
                    .build();

                dialog.add_response("ok", "OK");
                dialog.choose(self.account_view.widget(), None::<&gtk::gio::Cancellable>, |_| {});

                // Select the text for easy copying
                entry.select_region(0, -1);
            }
            AppMsg::LoginResult(result) => {
                self.auth_in_progress = false;
                match result {
                    Ok(account) => {
                        let name = account.username.clone();
                        add_account(&mut self.config, account);
                        let _ = self.config.save();

                        // Propagate config to account view and settings so new account appears immediately
                        self.account_view
                            .emit(AccountInput::UpdateConfig(self.config.clone()));
                        self.settings_dialog
                            .emit(SettingsInput::RefreshJava); // This also refreshes accounts in settings

                        let dialog = adw::AlertDialog::builder()
                            .heading("Login Successful")
                            .body(format!("Welcome, {}!", name))
                            .close_response("ok")
                            .default_response("ok")
                            .build();
                        dialog.add_response("ok", "OK");
                        dialog.choose(self.account_view.widget(), None::<&gtk::gio::Cancellable>, |_| {});
                    }
                    Err(err) => {
                        let dialog = adw::AlertDialog::builder()
                            .heading("Login Failed")
                            .body(&err)
                            .close_response("ok")
                            .default_response("ok")
                            .build();
                        dialog.add_response("ok", "OK");
                        dialog.choose(self.account_view.widget(), None::<&gtk::gio::Cancellable>, |_| {});
                    }
                }
            }
            AppMsg::Logout => {
                self.config.accounts.clear();
                self.config.active_account_uuid = None;
                let _ = self.config.save();
                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
            }
            AppMsg::SwitchTab(name) => {
                self.active_tab = name;
            }
            AppMsg::SetInstanceFeralGameMode(enabled) => {
                if let Some(path) = self.get_active_instance_path() {
                    let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                        &path,
                        "FeralGameMode",
                        enabled,
                    );
                    if let Some(idx) = self.selected_instance {
                        if let Some(inst) = self.instances.get_mut(idx) {
                            inst.feral_gamemode = enabled;
                        }
                    }
                }
            }
            AppMsg::SetInstanceDiscreteGpu(enabled) => {
                if let Some(path) = self.get_active_instance_path() {
                    let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                        &path,
                        "DiscreteGpu",
                        enabled,
                    );
                    if let Some(idx) = self.selected_instance {
                        if let Some(inst) = self.instances.get_mut(idx) {
                            inst.discrete_gpu = enabled;
                        }
                    }
                }
            }
            AppMsg::SetInstanceZinkVulkan(enabled) => {
                if let Some(path) = self.get_active_instance_path() {
                    let _ = crate::backend::instance::manager::set_instance_performance_tweak(
                        &path,
                        "ZinkVulkan",
                        enabled,
                    );
                    if let Some(idx) = self.selected_instance {
                        if let Some(inst) = self.instances.get_mut(idx) {
                            inst.zink_vulkan = enabled;
                        }
                    }
                }
            }
            AppMsg::KillInstance => {
                if let Some(path) = self.get_active_instance_path() {
                    let game_process = self.get_game_process(&path);
                    let mut guard = game_process.lock().unwrap();
                    if let Some(mut child) = guard.take() {
                        let _ = child.kill();
                        self.running_instances.remove(&path);
                        
                        // Update UI
                        if Some(&path) == self.get_active_instance_path().as_ref() {
                            let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                            self.instance_summary.emit(SummaryInput::Update(inst, false));
                        }

                        let buf = self.get_console_buffer(&path);
                        let mut iter = buf.end_iter();
                        buf.insert(&mut iter, "\nProcess killed by user.\n");
                    }
                }
            }
            AppMsg::VerifyInstance => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    let instance = inst.clone();
                    self.launch_after_download = false;
                    self.verifying_loading = true;
                    self.instance_summary.emit(SummaryInput::SetVerifyingLoading(true));
                    let sender_clone = _sender.input_sender().clone();
                    thread::spawn(move || {
                        if let Some(mc_version) = &instance.minecraft_version {
                            match find_version_by_id(mc_version) {
                                Ok(Some(v)) => {
                                    let (loader, loader_ver) = instance.get_loader_info();
                                    let _ = sender_clone
                                        .send(AppMsg::DownloadStart(v.raw, loader, loader_ver));
                                }
                                _ => {
                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance.path.clone(),
                                        format!(
                                            "Could not resolve Minecraft version {} for verify.\n",
                                            mc_version
                                        ),
                                    ));
                                    let _ = sender_clone.send(AppMsg::DownloadError(
                                        "Version resolution failed".into(),
                                    ));
                                }
                            }
                        }
                    });
                }
            }
            AppMsg::LaunchInstance => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    let instance = inst.clone();
                    let mut options = LaunchOptions::default();
                    if let Some(shared_path) = self.config.shared_data_path.clone() {
                        options.shared_data_path = shared_path;
                    }
                    options.mc_data_path = self.config.minecraft_data_path.clone();

                    // Priority: Instance specific java > Global java > "java"
                    let java_path = inst
                        .java_path
                        .clone()
                        .or_else(|| self.config.java_path.clone())
                        .unwrap_or_else(|| std::path::PathBuf::from("java"));

                    options.java_path = java_path;
                    options.max_memory = self.config.max_memory;
                    options.min_memory = self.config.min_memory;
                    options.account = get_active_account(&self.config).cloned();

                    // Check if everything is downloaded
                    if !check_instance_assets(&instance, &options) {
                        self.launch_after_download = true;
                        let sender_clone = _sender.input_sender().clone();
                        thread::spawn(move || {
                            if let Some(mc_version) = &instance.minecraft_version {
                                match find_version_by_id(mc_version) {
                                    Ok(Some(v)) => {
                                        let (loader, loader_ver) = instance.get_loader_info();
                                        let _ = sender_clone
                                            .send(AppMsg::DownloadStart(v.raw, loader, loader_ver));
                                    }
                                    _ => {
                                        let _ = sender_clone.send(AppMsg::ConsoleLog(instance.path.clone(), format!("Could not resolve Minecraft version {} for download.\n", mc_version)));
                                        let _ = sender_clone.send(AppMsg::DownloadError(
                                            "Version resolution failed".into(),
                                        ));
                                    }
                                }
                            }
                        });
                        return;
                    }

                    self.running_instances.insert(instance.path.clone());
                    
                    // Update Summary Tab immediately
                    self.instance_summary.emit(SummaryInput::Update(Some(instance.clone()), true));

                    let buf = self.get_console_buffer(&instance.path);
                    buf.set_text(&format!("Starting launch for {}...\n", instance.name));

                    let sender_clone = _sender.input_sender().clone();
                    let game_process = self.get_game_process(&instance.path);
                    let instance_path = instance.path.clone();
                    thread::spawn(move || {
                        let start_time_chrono = Utc::now();
                        let start_time = std::time::Instant::now();
                        let mut options = options;
                        let max_mem = options.max_memory;
                        let min_mem = options.min_memory;

                        // Check if running within a Flatpak sandbox
                        let is_flatpak = std::path::Path::new("/.flatpak-info").exists() || std::env::var("FLATPAK_ID").is_ok();
                        if is_flatpak {
                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance_path.clone(),
                                "ℹ️  Flatpak sandbox environment detected. Sandboxed isolation is active.\n".to_string(),
                            ));
                        }

                        // --- 1. Pre-flight health and Java version checks ---
                        let mc_version = instance.minecraft_version.as_deref().unwrap_or("1.21.8");
                        let required_ver = crate::backend::runtime::java::get_required_java_version(mc_version);
                        let mut selected_probed = crate::backend::runtime::java::probe_java(&options.java_path);

                        let mut java_ok = false;
                        let mut compatibility_msg = String::new();

                        if let Some(ref j) = selected_probed {
                            if let Some(actual_ver) = crate::backend::runtime::java::get_java_major_version(&j.version) {
                                if actual_ver == required_ver {
                                    java_ok = true;
                                    compatibility_msg = format!("Java {} (compatible)", actual_ver);
                                } else {
                                    compatibility_msg = format!("Java {} (recommended: Java {})", actual_ver, required_ver);
                                }
                            } else {
                                compatibility_msg = "Unknown version".to_string();
                            }
                        } else {
                            compatibility_msg = "Invalid or missing executable".to_string();
                        }

                        // Try to auto-switch if not fully compatible/working
                        if !java_ok {
                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance_path.clone(),
                                format!(
                                    "Checking Java compatibility... Selected path: {:?} ({})\n",
                                    options.java_path, compatibility_msg
                                ),
                            ));

                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance_path.clone(),
                                format!("Scanning system for a fully compatible Java {} runtime...\n", required_ver),
                            ));

                            // Find all system Java installations
                            let system_javas = crate::backend::runtime::java::find_java_versions(None);
                            let mut found_compatible = None;

                            // 1st priority: Find exact match for required Java version
                            for sj in &system_javas {
                                if let Some(ver) = crate::backend::runtime::java::get_java_major_version(&sj.version) {
                                    if ver == required_ver {
                                        found_compatible = Some(sj.clone());
                                        break;
                                    }
                                }
                            }

                            // 2nd priority: If required version is 8 but we didn't find any Java 8, or if we need a working JVM
                            if found_compatible.is_none() && selected_probed.is_none() {
                                // If the selected Java path doesn't work at all, fall back to ANY working Java
                                if let Some(any_working) = system_javas.first() {
                                    found_compatible = Some(any_working.clone());
                                }
                            }

                            if let Some(ref comp_java) = found_compatible {
                                let comp_ver = crate::backend::runtime::java::get_java_major_version(&comp_java.version).unwrap_or(0);
                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance_path.clone(),
                                    format!(
                                        "⚡ Auto-selector: Automatically switched launch to compatible system Java:\n  -> Path: {:?}\n  -> Version: Java {}\n",
                                        comp_java.path, comp_ver
                                    ),
                                ));
                                options.java_path = comp_java.path.clone();
                                selected_probed = Some(comp_java.clone());
                            } else {
                                if selected_probed.is_none() {
                                    // Complete showstopper: selected Java doesn't work and we found NO working Java on the system
                                    let mut flatpak_tip = String::new();
                                    if is_flatpak {
                                        flatpak_tip = "\n💡 FLATPAK SANDBOX TIP:\n\
                                                       Your host system's Java installations (in /usr/lib/jvm) are isolated and NOT visible in Flatpak.\n\
                                                       Please click the 'Install Java' button in Settings or Java Selector to download\n\
                                                       and install the required Java runtime directly inside the sandbox in one click!\n".to_string();
                                    }
                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance_path.clone(),
                                        format!(
                                            "❌ ERROR: No working Java installation was found on your system!\n\
                                             Please install Java (e.g. OpenJDK) or select a valid Java path in global settings.{}\n",
                                            flatpak_tip
                                        ),
                                    ));
                                    let _ = sender_clone.send(AppMsg::ProcessFinished(
                                        instance_path.clone(),
                                        0,
                                        start_time_chrono,
                                        Utc::now(),
                                    ));
                                    return;
                                } else {
                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance_path.clone(),
                                        format!(
                                            "⚠️ WARNING: No compatible Java {} was found on your system.\n\
                                             Attempting launch with configured Java ({:?}) anyway, but it may fail or crash!\n",
                                            required_ver, options.java_path
                                        ),
                                    ));
                                }
                            }
                        } else {
                            let _ = sender_clone.send(AppMsg::ConsoleLog(
                                instance_path.clone(),
                                format!("Java check passed: {:?} (Java {})\n", options.java_path, required_ver),
                            ));
                        }

                        match launch_instance(&instance, options) {
                            Ok(child) => {
                                // Store process handle for killing
                                {
                                    let mut _guard = game_process.lock().unwrap();
                                }

                                let mut child = child;
                                let stdout = child.stdout.take().unwrap();
                                let stderr = child.stderr.take().unwrap();

                                {
                                    let mut guard = game_process.lock().unwrap();
                                    *guard = Some(child);
                                }

                                // Spawn a concurrent thread to read stderr in real time
                                let sender_clone_err = sender_clone.clone();
                                let instance_path_err = instance_path.clone();
                                let stderr_thread = std::thread::spawn(move || {
                                    let mut err_reader = BufReader::new(stderr);
                                    let mut err_line = String::new();
                                    while let Ok(n) = err_reader.read_line(&mut err_line) {
                                        if n == 0 {
                                            break;
                                        }
                                        let _ = sender_clone_err.send(AppMsg::ConsoleLog(
                                            instance_path_err.clone(),
                                            format!("ERROR: {}", err_line),
                                        ));
                                        err_line.clear();
                                    }
                                });

                                // Read stdout in the main thread
                                let mut reader = BufReader::new(stdout);
                                let mut line = String::new();
                                while let Ok(n) = reader.read_line(&mut line) {
                                    if n == 0 {
                                        break;
                                    }
                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance_path.clone(),
                                        line.clone(),
                                    ));
                                    line.clear();
                                }

                                // Wait for stderr reader to finish
                                let _ = stderr_thread.join();

                                // Wait for the child to exit and get its exit status
                                // Take the child out of the game_process guard so we can wait on it and own it
                                let exit_status = {
                                    let mut guard = game_process.lock().unwrap();
                                    if let Some(mut c) = guard.take() {
                                        c.wait()
                                    } else {
                                        Err(std::io::Error::new(std::io::ErrorKind::Other, "Process was killed or already finished"))
                                    }
                                };

                                let duration = start_time.elapsed().as_secs();

                                // --- 2. Instant Crash Diagnosis ---
                                let mut exited_with_error = false;
                                let mut exit_status_msg = "\nProcess finished.\n".to_string();

                                match exit_status {
                                    Ok(status) => {
                                        if status.success() {
                                            exit_status_msg = "\nProcess finished successfully.\n".to_string();
                                        } else {
                                            exited_with_error = true;
                                            if let Some(code) = status.code() {
                                                exit_status_msg = format!("\nProcess finished with exit code: {}\n", code);
                                            } else {
                                                exit_status_msg = "\nProcess terminated by signal.\n".to_string();
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        exited_with_error = true;
                                        exit_status_msg = format!("\nProcess finished with error: {}\n", e);
                                    }
                                }

                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance_path.clone(),
                                    exit_status_msg,
                                ));

                                if exited_with_error && duration < 5 {
                                    let actual_ver_str = selected_probed
                                        .as_ref()
                                        .map(|j| format!("Java {} ({})", crate::backend::runtime::java::get_java_major_version(&j.version).unwrap_or(0), j.version))
                                        .unwrap_or_else(|| "Unknown".to_string());

                                    let mut flatpak_tip = String::new();
                                    if is_flatpak {
                                        flatpak_tip = "\n\n📦 FLATPAK SANDBOX DETECTED:\n\
                                                       Your host system's Java installations are isolated and not visible inside Flatpak.\n\
                                                       -> Recommendation: Open Settings -> Java -> Install Java, and download a compatible\n\
                                                          runtime directly inside the sandbox in one click!".to_string();
                                    }

                                    let diagnosis = format!(
                                        "\n\
                                        =================================================================\n\
                                        ⚠️  OBELISK INSTANT CRASH DETECTED\n\
                                        =================================================================\n\
                                        The game terminated immediately (within {} seconds) with a failure status.\n\
                                        This usually indicates incompatible Java, wrong JVM flags, or low memory.\n\n\
                                        🔍 Diagnosis & Environment Checklist:\n\
                                        1. Java Version Compatibility:\n\
                                           - Configured Java: {}\n\
                                           - Required Minecraft Java: Java {}\n\
                                           {}\n\n\
                                        2. Memory Allocation:\n\
                                           - Configured Max Heap (-Xmx): {} MB\n\
                                           - Configured Min Heap (-Xms): {} MB\n\
                                           (Make sure these are within your system's physical RAM capacity)\n\n\
                                        💡 Suggested Solutions:\n\
                                           - Check the error logs above for ClassFormatError or VM Option errors.\n\
                                           - Change the Java path in Instance -> Settings or Global Settings.\n\
                                           - Adjust max/min memory limits in Launcher settings.{}\n\
                                        =================================================================\n",
                                        duration,
                                        actual_ver_str,
                                        required_ver,
                                        if selected_probed.is_some() && crate::backend::runtime::java::get_java_major_version(&selected_probed.as_ref().unwrap().version).unwrap_or(0) == required_ver {
                                            "✅ Java version matches requirements."
                                        } else {
                                            "❌ Java version is incompatible! Please install or select the recommended Java version."
                                        },
                                        max_mem,
                                        min_mem,
                                        flatpak_tip
                                    );

                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance_path.clone(),
                                        diagnosis,
                                    ));
                                }

                                let _ = sender_clone.send(AppMsg::ProcessFinished(
                                    instance_path.clone(),
                                    duration,
                                    start_time_chrono,
                                    Utc::now(),
                                ));
                            }
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance_path.clone(),
                                    format!("Launch failed: {}\n", e),
                                ));
                                let _ = sender_clone.send(AppMsg::ProcessFinished(
                                    instance_path.clone(),
                                    0,
                                    start_time_chrono,
                                    Utc::now(),
                                ));
                            }
                        }
                    });
                }
            }
            AppMsg::ConsoleLog(path, msg) => {
                let is_active = Some(&path) == self.get_active_instance_path().as_ref();

                let level = LogLevel::from_line(&msg);
                let line = LogLine {
                    level,
                    content: msg.clone(),
                };

                self.instance_logs
                    .entry(path.clone())
                    .or_default()
                    .push(line);

                let filter = self.console_filter;
                if filter == LogLevel::All || level == filter {
                    let buf = self.get_console_buffer(&path);
                    let mut iter = buf.end_iter();
                    buf.insert(&mut iter, &msg);

                    if is_active {
                        let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                        let running = self.running_instances.contains(&path);
                        self.instance_summary.emit(SummaryInput::Update(inst, running));
                        self.instance_console.emit(ConsoleInput::Update(buf, running));
                    }
                }
            }
            AppMsg::ProcessFinished(path, duration, start, end) => {
                let is_active = Some(&path) == self.get_active_instance_path().as_ref();
                self.running_instances.remove(&path);

                // Re-scan the instance to update the playtime field from the disk if we just wrote it
                // We sync the total duration to instance.cfg for Prism compatibility,
                // but keep our detailed sessions in playtime.json.
                if duration > 0 {
                    let _ = crate::backend::instance::manager::update_instance_playtime(&path, duration);
                }

                // Important: find and update the instance in self.instances so the UI sees the new playtime
                let mut found = false;
                let mut instance_id = String::new();
                for inst in &mut self.instances {
                    if inst.path == path {
                        inst.total_time_played += duration;
                        instance_id = inst.id.clone();
                        found = true;
                        break;
                    }
                }

                if found && duration > 0 {
                    // Update persistent playtime tracker with a detailed session
                    if !instance_id.is_empty() {
                        self.playtime_manager.add_session(PlaySession {
                            instance_id: instance_id.clone(),
                            start_time: start,
                            end_time: end,
                            duration_seconds: duration,
                        });
                    }

                    self.config.total_playtime += duration;
                    let _ = self.config.save();
                    self.rebuild_overview();
                }

                if is_active {
                    let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                    self.instance_summary.emit(SummaryInput::Update(inst, false));
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        false,
                    ));
                    
                    // Re-scan single instance to make sure we're fully in sync
                    let sender_clone = _sender.input_sender().clone();
                    let path_clone = path.clone();
                    thread::spawn(move || {
                        if let Some(inst) = scan_single_instance(&path_clone, true) {
                            let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(inst));
                        }
                    });
                }
            }
            AppMsg::ClearConsole(path) => {
                if let Some(logs) = self.instance_logs.get_mut(&path) {
                    logs.clear();
                }
                self.rebuild_console_buffer(&path);
                if Some(&path) == self.get_active_instance_path().as_ref() {
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        self.is_active_instance_running(),
                    ));
                }
            }
            AppMsg::ClearActiveConsole => {
                if let Some(path) = self.get_active_instance_path() {
                    if let Some(logs) = self.instance_logs.get_mut(&path) {
                        logs.clear();
                    }
                    self.rebuild_console_buffer(&path);
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        self.is_active_instance_running(),
                    ));
                }
            }
            AppMsg::SetConsoleFilter(filter) => {
                self.console_filter = filter;
                if let Some(path) = self.get_active_instance_path() {
                    self.rebuild_console_buffer(&path);
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        self.is_active_instance_running(),
                    ));
                }
            }
            AppMsg::InstanceCreated(_version, path) => {
                // Apply default icon if set
                if let Some(default_icon) = &self.config.default_instance_icon {
                    let target = path.join("icon.png");
                    let _ = std::fs::copy(default_icon, target);
                }

                // Show success toast
                self.toast_overlay.add_toast(adw::Toast::new("Instance created successfully"));

                // Ensure the dialog is closed
                self.add_instance_dialog.widget().close();

                if let Some(config_path) = &self.config.instances_path {
                    let path_clone = config_path.clone();
                    let sender_clone = _sender.input_sender().clone();
                    thread::spawn(move || {
                        let insts = scan_instances(&path_clone);
                        let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
                    });
                }
                // Automatically start download if vanilla
                self.selected_instance = None; // Reset selection to trigger refresh
                _sender.input(AppMsg::DownloadStart(_version.raw, ModLoader::None, None));
            }
            AppMsg::DownloadStart(raw_version, loader, loader_version) => {
                self.download_dialog.emit(DownloadDialogInput::Start);
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        DownloadState::Starting,
                        true,
                    ));

                let data_path = self.config.minecraft_data_path.clone();
                let sender_clone = _sender.input_sender().clone();

                let job = crate::backend::download::manager::NetworkJob {
                    id: format!("mc-{}-{}", raw_version.id, std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()),
                    title: format!("Minecraft {}", raw_version.id),
                    tasks: vec![crate::backend::download::manager::NetworkTask::MinecraftDownload {
                        version: raw_version.clone(),
                        loader: loader.clone(),
                        loader_version: loader_version.clone(),
                        data_path: data_path.clone(),
                    }],
                    status: crate::backend::download::manager::NetworkJobStatus::Pending,
                    log: Vec::new(),
                };

                let (tx, rx) = std::sync::mpsc::channel::<crate::backend::download::manager::DownloadMsg>();

                // Queue the job in DOWNLOAD_QUEUE
                crate::backend::download::manager::DOWNLOAD_QUEUE.add_job(job, tx);

                // Spawn a thread to forward messages from the channel rx to the AppMsg channel
                thread::spawn(move || {
                    while let Ok(msg) = rx.recv() {
                        let is_finished = matches!(
                            msg,
                            crate::backend::download::manager::DownloadMsg::Finished
                                | crate::backend::download::manager::DownloadMsg::Error(_)
                        );
                        let app_msg = AppMsg::DownloadProgress(msg);
                        if sender_clone.send(app_msg).is_err() {
                            break;
                        }
                        if is_finished {
                            break;
                        }
                    }
                });
            }
            AppMsg::DownloadProgress(msg) => {
                let (state, visible) = match msg {
                    DownloadMsg::Progress(status, progress) => {
                        let state = if progress == 0.0 {
                            DownloadState::Starting
                        } else {
                            DownloadState::Downloading {
                                task: status,
                                current: 0,
                                total: 0,
                                item_name: String::new(),
                                progress,
                            }
                        };
                        (state, true)
                    }
                    DownloadMsg::DetailedProgress {
                        task,
                        current,
                        total,
                        item_name,
                        overall_progress,
                    } => {
                        let state = DownloadState::Downloading {
                            task,
                            current,
                            total,
                            item_name,
                            progress: overall_progress,
                        };
                        (state, true)
                    }
                    DownloadMsg::Error(e) => {
                        _sender.input(AppMsg::DownloadError(e));
                        return;
                    }
                    DownloadMsg::Finished => {
                        _sender.input(AppMsg::DownloadFinished);
                        return;
                    }
                };

                self.download_dialog.emit(DownloadDialogInput::UpdateState(state.clone()));
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(state, visible));
            }
            AppMsg::DownloadFinished => {
                if self.verifying_loading {
                    self.toast_overlay.add_toast(adw::Toast::new("Instance verification completed successfully!"));
                }
                self.verifying_loading = false;
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(false));

                self.download_dialog.emit(DownloadDialogInput::UpdateState(DownloadState::Finished));
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        DownloadState::Finished,
                        false,
                    ));

                if self.launch_after_download {
                    self.launch_after_download = false;
                    _sender.input(AppMsg::LaunchInstance);
                }
            }
            AppMsg::DownloadError(err) => {
                self.launch_after_download = false;
                self.verifying_loading = false;
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(false));

                self.download_dialog.emit(DownloadDialogInput::UpdateState(DownloadState::Failed(err.clone())));
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        DownloadState::Failed(err),
                        true,
                    ));
            }
            AppMsg::DismissDownloadStatus => {
                self.download_status_bar.emit(DownloadStatusBarInput::Dismiss);
            }
            AppMsg::RemoveJob(id) => {
                crate::backend::download::manager::DOWNLOAD_QUEUE.remove_job(&id);
                self.download_dialog.emit(DownloadDialogInput::Refresh);
            }
            AppMsg::ClearFinishedJobs => {
                crate::backend::download::manager::DOWNLOAD_QUEUE.clear_finished_jobs();
                self.download_dialog.emit(DownloadDialogInput::Refresh);
            }
            AppMsg::OpenDownloadDetails => {
                self.download_dialog.emit(DownloadDialogInput::Show);
                self.download_dialog.widget().present(Some(&self.window));
            }
        }
    }
}


// ─── AppModel helper methods ──────────────────────────────────────────────────

impl AppModel {
    /// Re-emit the current instances + groups to the sidebar and overview grid.
    fn show_move_to_group_dialog(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        let dialog_win = adw::Dialog::builder()
            .title("Move to Group")
            .content_width(380)
            .can_close(true)
            .build();
        
        let content = adw::ToolbarView::builder().build();
        let header = adw::HeaderBar::builder().show_end_title_buttons(false).build();
        let window_title = adw::WindowTitle::builder().title("Move to Group").build();
        header.set_title_widget(Some(&window_title));
        content.add_top_bar(&header);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_height(200)
            .max_content_height(350)
            .build();

        let lb = gtk::ListBox::builder()
            .css_classes(vec!["boxed-list".to_string()])
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .valign(gtk::Align::Start)
            .build();

        // Cancel button at the bottom
        let cancel_btn = gtk::Button::builder()
            .label("Cancel")
            .margin_top(6)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .css_classes(vec!["pill".to_string()])
            .halign(gtk::Align::Center)
            .width_request(120)
            .build();
        {
            let dw = dialog_win.clone();
            cancel_btn.connect_clicked(move |_| {
                dw.close();
            });
        }
        content.add_bottom_bar(&cancel_btn);
            
        let group_names = self.groups.sorted_group_names();
        for gname in group_names {
            let row = adw::ActionRow::builder()
                .title(gname)
                .activatable(true)
                .build();
            let icon = gtk::Image::from_icon_name("folder-symbolic");
            row.add_prefix(&icon);
            let dw = dialog_win.clone();
            let s = sender.input_sender().clone();
            let gn = gname.to_string();
            row.connect_activated(move |_| {
                dw.close();
                let _ = s.send(AppMsg::MoveInstanceToGroup(idx, gn.clone()));
            });
            lb.append(&row);
        }
        
        let new_group_row = adw::ActionRow::builder()
            .title("New Group…")
            .activatable(true)
            .build();
        let add_icon = gtk::Image::from_icon_name("list-add-symbolic");
        new_group_row.add_prefix(&add_icon);
        {
            let dw = dialog_win.clone();
            let s = sender.input_sender().clone();
            new_group_row.connect_activated(move |_| {
                dw.close();
                let _ = s.send(AppMsg::CreateGroupWithMove(idx));
            });
        }
        lb.append(&new_group_row);
        
        scrolled.set_child(Some(&lb));
        content.set_content(Some(&scrolled));
        dialog_win.set_child(Some(&content));
        dialog_win.present(Some(&self.window));
    }

    fn rebuild_overview(&self) {
        self.overview_grid.emit(OverviewInput::Rebuild(
            self.instances.clone(),
            self.groups.clone(),
        ));
    }


    /// Show an dialog to get a group name, then optionally move `move_idx` into it.
    fn show_create_group_dialog_then_move(
        &self,
        sender: &ComponentSender<AppModel>,
        move_idx: Option<usize>,
    ) {
        let dialog = adw::AlertDialog::builder()
            .heading("Create Group")
            .body("Enter a name for the new group:")
            .close_response("cancel")
            .default_response("create")
            .build();
        dialog.add_response("cancel", "Cancel");
        dialog.add_response("create", "Create");
        dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);

        let entry = gtk::Entry::builder()
            .placeholder_text("Group name...")
            .activates_default(true)
            .build();
        dialog.set_extra_child(Some(&entry));

        let sender_clone = sender.input_sender().clone();
        dialog.choose(
            &self.window,
            None::<&gtk::gio::Cancellable>,
            move |response| {
                if response == "create" {
                    let name = entry.text().to_string();
                    sender_clone
                        .send(AppMsg::ConfirmCreateGroup(name.clone()))
                        .unwrap();
                    if let Some(idx) = move_idx {
                        sender_clone
                            .send(AppMsg::MoveInstanceToGroup(idx, name))
                            .unwrap();
                    }
                }
            },
        );
    }

    fn show_target_instance_selector(&self, editor_type: EditorType, ids: Vec<String>, is_copy: bool, sender_clone: relm4::Sender<AppMsg>) {
        if let Some(current_inst_index) = self.selected_instance {
            let instances = self.instances.clone();
            let ids_clone = ids.clone();
            let type_clone = editor_type.clone();
            
            let heading = match (is_copy, ids.len()) {
                (true, 1) => "Copy Item".to_string(),
                (true, _) => format!("Copy {} Items", ids.len()),
                (false, 1) => "Move Item".to_string(),
                (false, _) => format!("Move {} Items", ids.len()),
            };
            
            let dialog = adw::AlertDialog::builder()
                .heading(&heading)
                .body("Select the target instance:")
                .build();
            dialog.add_response("cancel", "Cancel");
            
            let list_box = gtk::ListBox::new();
            list_box.set_selection_mode(gtk::SelectionMode::Single);
            list_box.add_css_class("boxed-list");
            
            let mut inst_indices = Vec::new();
            for (idx, other) in instances.iter().enumerate() {
                if idx == current_inst_index { continue; }
                inst_indices.push(idx);
                let row = adw::ActionRow::builder()
                    .title(&other.name)
                    .activatable(true)
                    .build();
                list_box.append(&row);
            }
            
            let scrolled = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .min_content_height(200)
                .max_content_height(400)
                .child(&list_box)
                .build();
                
            dialog.set_extra_child(Some(&scrolled));
            
            dialog.connect_response(None, move |_d, response| {
                if response != "cancel" {
                    if let Some(row) = list_box.selected_row() {
                        let selected_row_idx = row.index() as usize;
                        if let Some(&target_idx) = inst_indices.get(selected_row_idx) {
                             if is_copy {
                                 sender_clone.send(AppMsg::ConfirmCopyItems(type_clone.clone(), ids_clone.clone(), target_idx)).ok();
                             } else {
                                 sender_clone.send(AppMsg::ConfirmMoveItems(type_clone.clone(), ids_clone.clone(), target_idx)).ok();
                             }
                        }
                    }
                }
            });
            dialog.present(Some(&self.window));
        }
    }
}
