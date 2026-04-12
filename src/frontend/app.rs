#![allow(unused_assignments)]
use crate::backend::auth::account::{
    add_account, create_offline_account, get_active_account,
    remove_account, switch_account,
};
use crate::backend::auth::microsoft::{self as auth, Account};
use crate::backend::download::manager::{download_minecraft_data, DownloadMsg};
use crate::backend::instance::launcher::{check_instance_assets, launch_instance, LaunchOptions};
use crate::backend::instance::manager::{
    add_instance_item, delete_instance, is_loader_component, remove_component,
    remove_instance_item, remove_mod_loader, rename_instance, scan_instances,
    scan_single_instance, set_component_version, set_instance_java,
    set_mod_loader_with_version, Instance, ModLoader,
};
use crate::backend::playtime::PlaytimeManager;
use crate::backend::runtime::versions::{find_version_by_id, MinecraftVersion, RawVersion};
use crate::config::Config;
use crate::frontend::dialogs::external::download::{
    DownloadDialog, DownloadDialogInput, DownloadStatusBar, DownloadStatusBarInput,
    DownloadStatusBarOutput,
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
use crate::frontend::dialogs::instance::icon_chooser::{
    IconChooserDialog, IconChooserInput, IconChooserOutput,
};
use crate::frontend::dialogs::instance::sharing::{
    ImportDialog, ImportInput, ImportOutput, InstanceSharerDialog, SharerInput, SharerOutput,
    ImportStep,
};
use crate::frontend::dialogs::system::assets::{AssetManagerDialog, AssetManagerInput};
use crate::frontend::dialogs::system::java::{
    JavaSelectorDialog, JavaSelectorInput, JavaSelectorOutput,
};
use crate::frontend::dialogs::system::settings::{SettingsDialog, SettingsInput, SettingsOutput};
use crate::frontend::views::account::{AccountInput, AccountView};
use crate::backend::instance::groups::InstanceGroups;
use crate::backend::instance::sharing::{export_instance, import_shared_instance, SharedInstance, export_instance_to_zip, import_instance_from_zip};
pub use crate::frontend::views::instance::tabs::console::{LogLevel, LogLine};
use crate::frontend::views::instance::{
    ConsoleInput, ConsoleOutput, EditorTabInput, EditorTabOutput,
    InstanceConsole, InstanceEditorTab, InstanceSettingsTab, InstanceSummary, LayoutMode,
    OverviewGrid, OverviewInput, OverviewOutput, SettingsTabInput, SettingsTabOutput,
    SidebarInput, SidebarList, SidebarOutput, SummaryInput, SummaryOutput,
};
use adw::prelude::*;
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
    settings_dialog: Controller<SettingsDialog>,
    add_instance_dialog: Controller<AddInstanceDialog>,
    instance_editor: Controller<InstanceEditorDialog>,
    download_dialog: Controller<DownloadDialog>,
    java_selector: Controller<JavaSelectorDialog>,
    component_editor: Controller<ComponentEditorDialog>,
    mod_loader_dialog: Controller<ModLoaderDialog>,
    asset_manager: Controller<AssetManagerDialog>,
    modrinth_browser: Controller<ModrinthBrowser>,
    icon_chooser: Controller<IconChooserDialog>,
    sharer_dialog: Controller<InstanceSharerDialog>,
    import_dialog: Controller<ImportDialog>,

    // Sidebar + overview
    sidebar: Controller<SidebarList>,
    overview_grid: Controller<OverviewGrid>,
    show_overview: bool,

    // Views
    instance_summary: Controller<InstanceSummary>,
    instance_editor_tab: Controller<InstanceEditorTab>,
    instance_console: Controller<InstanceConsole>,
    instance_settings_tab: Controller<InstanceSettingsTab>,
    account_view: Controller<AccountView>,
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
    playtime_manager: PlaytimeManager,
    sharing_loading: bool,
    import_loading: bool,
    verifying_loading: bool,
}

#[derive(Debug)]
pub enum AppMsg {
    OpenSettings,
    OpenAbout,
    OpenAssetManager,
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
    InstanceCreated(MinecraftVersion),
    ConfirmDelete(usize),
    ConfirmRename(usize, String),
    InstancesScanned(Vec<Instance>),
    InstancesUpdated(Vec<Instance>),
    ChangeInstanceIcon(usize),
    /// Open the native file picker for an instance icon (from the icon chooser).
    ChangeInstanceIconFromFile(usize),
    /// Apply the global default icon to an instance.
    ApplyDefaultIcon(usize),
    /// Apply a specific icon file to an instance (e.g. from recents).
    ApplyIconPath(usize, PathBuf),

    // Sidebar / group management
    SidebarEvent(SidebarOutput),
    OverviewEvent(OverviewOutput),
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
    ProcessFinished(PathBuf, u64),
    SwitchTab(String),

    // Performance Tweaks
    SetInstanceFeralGameMode(bool),
    SetInstanceDiscreteGpu(bool),
    SetInstanceZinkVulkan(bool),

    // Downloading
    DownloadStart(RawVersion, ModLoader, Option<String>),
    DownloadProgress(DownloadMsg),
    OpenDownloadDetails,
    DownloadFinished,
    DownloadError(String),
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

    fn format_total_playtime(&self) -> String {
        let seconds = self.playtime_manager.get_total_playtime();
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;

        if hours > 0 {
            format!("{}h {}m total playtime", hours, minutes)
        } else {
            format!("{}m total playtime", minutes)
        }
    }

    fn get_account_status_label(&self) -> String {
        if let Some(account) = crate::backend::auth::account::get_active_account(&self.config) {
            let status = crate::backend::auth::account::verify_account_status(account);
            format!("{} ({})", account.username, status)
        } else {
            "Not Logged In".to_string()
        }
    }

    fn get_account_status_class(&self) -> &str {
        if let Some(account) = crate::backend::auth::account::get_active_account(&self.config) {
            match crate::backend::auth::account::verify_account_status(account) {
                crate::backend::auth::account::AccountStatus::Valid |
                crate::backend::auth::account::AccountStatus::ExpiringSoon => "dot-green",
                crate::backend::auth::account::AccountStatus::Expired |
                crate::backend::auth::account::AccountStatus::Unknown(_) => "dot-red",
                crate::backend::auth::account::AccountStatus::Offline => "dot-grey",
            }
        } else {
            "dot-grey"
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
                set_title: Some("Minecraft Manager"),
                set_default_width: 900,
                set_default_height: 600,

                #[wrap(Some)]
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
                            set_sidebar_width_fraction: 0.28,
                            set_min_sidebar_width: 240.0,
                            set_max_sidebar_width: 340.0,

                            // ── Sidebar ──────────────────────────────────────
                            #[wrap(Some)]
                            set_sidebar = &adw::NavigationPage {
                                set_title: "Instances",
                                #[wrap(Some)]
                                set_child = &adw::ToolbarView {
                                    add_top_bar = &adw::HeaderBar {
                                        #[wrap(Some)]
                                        set_title_widget = &adw::WindowTitle {
                                            set_title: "Instances",
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
                                                                set_label: "Asset Manager",
                                                                set_hexpand: true,
                                                                set_halign: gtk::Align::Start,
                                                            },
                                                        },
                                                        connect_clicked[sender, main_popover] => move |_| {
                                                            main_popover.popdown();
                                                            sender.input(AppMsg::OpenAssetManager);
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
                                                                set_label: "Settings",
                                                                set_hexpand: true,
                                                                set_halign: gtk::Align::Start,
                                                            },
                                                        },
                                                        connect_clicked[sender, main_popover] => move |_| {
                                                            main_popover.popdown();
                                                            sender.input(AppMsg::OpenSettings);
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

                                    // Sidebar content: spinner or SidebarList
                                    #[wrap(Some)]
                                    set_content = &gtk::Stack {
                                        set_transition_type: gtk::StackTransitionType::Crossfade,
                                        #[watch]
                                        set_visible_child_name: if model.loading_instances { "loading" } else { "content" },

                                        add_named[Some("content")] = model.sidebar.widget(),

                                        add_named[Some("loading")] = &adw::Spinner {
                                            set_halign: gtk::Align::Center,
                                            set_valign: gtk::Align::Center,
                                            set_width_request: 32,
                                            set_height_request: 32,
                                        },
                                    },

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

                                        #[wrap(Some)]
                                        #[name = "title_widget"]
                                        set_title_widget = &adw::ViewSwitcher {
                                            #[watch]
                                            set_visible: model.selected_instance.is_some() && !model.show_overview,
                                            set_policy: adw::ViewSwitcherPolicy::Narrow,
                                            #[watch]
                                            set_stack: Some(&stack),
                                        },

                                        pack_end = &gtk::Box {
                                            #[watch]
                                            set_visible: model.show_overview,
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

                                            // Refresh toggle
                                            gtk::Button {
                                                set_icon_name: "view-refresh-symbolic",
                                                set_tooltip_text: Some("Refresh Instances"),
                                                set_css_classes: &["flat"],
                                                #[watch]
                                                set_sensitive: !model.loading_instances,
                                                connect_clicked => AppMsg::RefreshInstances,
                                            },

                                            // Layout toggle
                                            gtk::Box {
                                                set_css_classes: &["linked"],
                                                gtk::ToggleButton {
                                                    set_icon_name: "view-grid-symbolic",
                                                    set_tooltip_text: Some("Grid View"),
                                                    #[watch]
                                                    set_active: model.overview_layout == LayoutMode::Grid,
                                                    connect_clicked => AppMsg::SetOverviewLayout(LayoutMode::Grid),
                                                },
                                                gtk::ToggleButton {
                                                    set_icon_name: "view-list-symbolic",
                                                    set_tooltip_text: Some("List View"),
                                                    #[watch]
                                                    set_active: model.overview_layout == LayoutMode::List,
                                                    connect_clicked => AppMsg::SetOverviewLayout(LayoutMode::List),
                                                }
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

                                        // Overview grid
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_vexpand: true,
                                            #[watch]
                                            set_visible: model.show_overview,

                                            model.overview_grid.widget(),
                                        },

                                        // Welcome page (no instance selected, not overview)
                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_valign: gtk::Align::Center,
                                            set_halign: gtk::Align::Center,
                                            set_spacing: 24,
                                            set_vexpand: true,
                                            #[watch]
                                            set_visible: model.selected_instance.is_none() && !model.show_overview,

                                            adw::StatusPage {
                                                set_title: "Welcome",
                                                set_description: Some("Select an instance from the sidebar to get started, or create a new one."),
                                            },

                                            gtk::Button {
                                                set_label: "Add Instance",
                                                set_halign: gtk::Align::Center,
                                                set_css_classes: &["suggested-action", "pill"],
                                                set_width_request: 200,
                                                connect_clicked => AppMsg::AddInstance,
                                            }
                                        },

                                        // Instance detail tabs
                                        #[name = "stack"]
                                        adw::ViewStack {
                                            #[watch]
                                            set_visible: model.selected_instance.is_some() && !model.show_overview,
                                            #[watch]
                                            set_visible_child_name: if model.active_tab.is_empty() { "summary" } else { &model.active_tab },
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
                                        }
                                    },
                                     add_bottom_bar = &gtk::Box {
                                         set_orientation: gtk::Orientation::Horizontal,
                                         set_spacing: 12,
                                         set_css_classes: &["status-bar-container"],
                                         #[watch]
                                         set_visible: model.show_overview,

                                         gtk::Box {
                                             set_orientation: gtk::Orientation::Horizontal,
                                             set_hexpand: true,
                                             set_spacing: 8,
                                             set_valign: gtk::Align::Center,
                                             set_css_classes: &["status-bar"],

                                             gtk::Button {
                                                 set_has_frame: false,
                                                 set_css_classes: &["flat", "account-status-button"],
                                                 set_tooltip_text: Some("Open Account Manager"),
                                                 connect_clicked => AppMsg::AccountAction,
                                                 set_valign: gtk::Align::Center,

                                                 #[wrap(Some)]
                                                 set_child = &gtk::Box {
                                                     set_orientation: gtk::Orientation::Horizontal,
                                                     set_spacing: 8,
                                                     set_valign: gtk::Align::Center,

                                                     gtk::Box {
                                                         #[watch]
                                                         set_css_classes: &["status-dot", model.get_account_status_class()],
                                                         set_width_request: 10,
                                                         set_height_request: 10,
                                                         set_valign: gtk::Align::Center,
                                                     },
                                                     gtk::Label {
                                                         #[watch]
                                                         set_label: &model.get_account_status_label(),
                                                         set_css_classes: &["caption-heading"],
                                                     },
                                                 }
                                             },

                                             gtk::Box { set_hexpand: true },

                                             gtk::Label {
                                                 #[watch]
                                                 set_label: &model.format_total_playtime(),
                                                 set_css_classes: &["caption-heading"],
                                             },
                                         }
                                     },
                                }
                            }
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
                AddInstanceOutput::InstanceCreated(version) => AppMsg::InstanceCreated(version),
            });

        let instance_editor = InstanceEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::EditorOutput);

        let download_dialog = DownloadDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |_| AppMsg::DownloadFinished);

        let java_selector = JavaSelectorDialog::builder()
            .launch(Some(config.minecraft_data_path.join("java")))
            .forward(sender.input_sender(), |out| match out {
                JavaSelectorOutput::Selected(path) => AppMsg::SetInstanceJava(path),
            });

        let component_editor = ComponentEditorDialog::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::ComponentEditorOutput);

        let mod_loader_dialog = ModLoaderDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| AppMsg::ModLoaderOutput(out));

        let asset_manager = AssetManagerDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |_| unreachable!());

        let icon_chooser = IconChooserDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |out| match out {
                IconChooserOutput::ChooseFromFile(idx) => AppMsg::ChangeInstanceIconFromFile(idx),
                IconChooserOutput::UseDefault(idx) => AppMsg::ApplyDefaultIcon(idx),
                IconChooserOutput::UseRecent(idx, path) => AppMsg::ApplyIconPath(idx, path),
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
                let _ = sender_clone.send(AppMsg::InstancesScanned(insts));
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
            settings_dialog,
            add_instance_dialog,
            instance_editor,
            download_dialog,
            java_selector,
            component_editor,
            mod_loader_dialog,
            asset_manager,
            modrinth_browser,
            icon_chooser,
            sharer_dialog,
            import_dialog,
            sidebar,
            overview_grid,
            show_overview: true, // Start on overview

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
            playtime_manager: PlaytimeManager::load(),
            sharing_loading: false,
            import_loading: false,
            verifying_loading: false,
        };

        let widgets = view_output!();

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
            750.0,
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

        // Set initial stack page
        widgets.stack.set_visible_child_name("summary");

        model.toast_overlay = widgets
            .main_content_box
            .parent()
            .unwrap()
            .downcast::<adw::ToastOverlay>()
            .unwrap();


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
                SidebarOutput::ShowOverview => _sender.input(AppMsg::ShowOverview),
                SidebarOutput::SelectInstance(idx) => _sender.input(AppMsg::SelectInstance(idx)),
                SidebarOutput::RenameInstance(idx) => _sender.input(AppMsg::RenameInstanceRequest(idx)),
                SidebarOutput::DeleteInstance(idx) => _sender.input(AppMsg::DeleteInstanceRequest(idx)),
                SidebarOutput::MoveToGroup(idx, group) => _sender.input(AppMsg::MoveInstanceToGroup(idx, group)),
                SidebarOutput::MoveToGroupRequest(idx) => _sender.input(AppMsg::MoveToGroupRequest(idx)),
                SidebarOutput::RemoveFromGroup(idx) => _sender.input(AppMsg::RemoveInstanceFromGroup(idx)),
                SidebarOutput::CreateGroup(raw) => {
                    // "__move__N" sentinel: create group then move instance N
                    if let Some(idx_str) = raw.strip_prefix("__move__") {
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            // Open create-group dialog, then move
                            // For simplicity we open CreateGroupRequest and store pending move
                            self.show_create_group_dialog_then_move(&_sender, Some(idx));
                            return;
                        }
                    }
                    _sender.input(AppMsg::CreateGroupRequest);
                }
                SidebarOutput::RenameGroup(old, _) => _sender.input(AppMsg::RenameGroupRequest(old)),
                SidebarOutput::DeleteGroup(name) => _sender.input(AppMsg::DeleteGroupRequest(name)),
                SidebarOutput::ChangeIcon(idx) => _sender.input(AppMsg::ChangeInstanceIcon(idx)),
                SidebarOutput::ShareInstance(idx) => _sender.input(AppMsg::ShareInstance(idx)),
            },
            AppMsg::OverviewEvent(out) => match out {
                OverviewOutput::SelectInstance(idx) => _sender.input(AppMsg::SelectInstance(idx)),
                OverviewOutput::RenameInstance(idx) => _sender.input(AppMsg::RenameInstanceRequest(idx)),
                OverviewOutput::DeleteInstance(idx) => _sender.input(AppMsg::DeleteInstanceRequest(idx)),
                OverviewOutput::MoveToGroupRequest(idx) => _sender.input(AppMsg::MoveToGroupRequest(idx)),
                OverviewOutput::RemoveFromGroup(idx) => _sender.input(AppMsg::RemoveInstanceFromGroup(idx)),
                OverviewOutput::RenameGroup(name) => _sender.input(AppMsg::RenameGroupRequest(name)),
                OverviewOutput::DeleteGroup(name) => _sender.input(AppMsg::DeleteGroupRequest(name)),
                OverviewOutput::ChangeIcon(idx) => _sender.input(AppMsg::ChangeInstanceIcon(idx)),
                OverviewOutput::ShareInstance(idx) => _sender.input(AppMsg::ShareInstance(idx)),
                OverviewOutput::LayoutModeChanged(mode) => self.overview_layout = mode,
                OverviewOutput::AddInstance => _sender.input(AppMsg::AddInstance),
                OverviewOutput::CreateGroup => _sender.input(AppMsg::CreateGroupRequest),
            },
            AppMsg::ShowOverview => {
                self.show_overview = true;
                self.selected_instance = None;
                self.sidebar.emit(SidebarInput::SetSelected(None));
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
                    self.rebuild_sidebar_and_overview();
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
                    self.rebuild_sidebar_and_overview();
                }
            }
            AppMsg::RemoveInstanceFromGroup(idx) => {
                if let Some(inst) = self.instances.get(idx) {
                    let folder = inst.path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
                    if let Some(path) = &self.config.instances_path {
                        self.groups.remove_instance_from_groups(&folder);
                        let _ = self.groups.save(path);
                    }
                    self.rebuild_sidebar_and_overview();
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
                            sender_clone.send(AppMsg::ConfirmRenameGroup(old.clone(), new_name)).unwrap();
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
                    self.rebuild_sidebar_and_overview();
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
                self.rebuild_sidebar_and_overview();
            }
            AppMsg::OpenSettings => {
                self.settings_dialog.emit(SettingsInput::Open);
            }
            AppMsg::OpenAccountSettings => {
                self.settings_dialog.emit(SettingsInput::Open);
                self.settings_dialog
                    .emit(SettingsInput::SetPage("accounts".to_string()));
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
                    .application_name("Minecraft Manager")
                    .version("0.1.0")
                    .developer_name("Magnotec")
                    .license_type(gtk::License::Gpl30)
                    .website("https://github.com/magnotec/minecraft-manager")
                    .issue_url("https://github.com/magnotec/minecraft-manager/issues")
                    .comments(
                        "A modern Minecraft instance manager built with Rust and GTK4/Libadwaita.",
                    )
                    .build();
                about.present(Some(&self.window));
            }
            AppMsg::RefreshInstances => {
                self.loading_instances = true;
                self.overview_grid.emit(OverviewInput::SetLoading(true));
                self.overview_grid.emit(OverviewInput::GoBack);
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
                let data_path = self.config.minecraft_data_path.clone();
                let shared_path = self.config.shared_data_path.clone();
                let instances_path = self.config.instances_path.clone();
                self.asset_manager.emit(AssetManagerInput::Open(
                    data_path,
                    shared_path,
                    instances_path,
                ));
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
                    self.rebuild_sidebar_and_overview();
                }
            }
            AppMsg::InstancesScanned(insts) => {
                // This message is deprecated, use InstancesUpdated instead
                // For now, just forward to InstancesUpdated
                _sender.input(AppMsg::InstancesUpdated(insts));
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

                self.rebuild_sidebar_and_overview();

                if let Some(idx) = old_selection {
                    if idx < self.instances.len() {
                        _sender.input(AppMsg::SelectInstance(idx));
                    } else {
                        self.selected_instance = None;
                        self.show_overview = true;
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
                            if let Some(updated) = scan_single_instance(&path) {
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
                                    EditorType::Mods => updated_inst.mods.iter().map(|m| EditorItem { id: m.filename.clone(), name: m.name.clone(), version: m.version.clone(), filename: m.filename.clone(), description: m.description.clone(), homepage: m.homepage.clone(), sources: None, icon_path: m.icon_path.clone(), is_checked: false, size: None, seed: None, last_played: None }).collect(),
                                    EditorType::Components => updated_inst.components.iter().map(|c| EditorItem { id: c.uid.clone(), name: c.name.clone(), version: c.version.clone(), filename: c.uid.clone(), description: None, homepage: None, sources: None, icon_path: None, is_checked: false, size: None, seed: None, last_played: None }).collect(),
                                    EditorType::ResourcePacks => updated_inst.resource_packs.iter().map(|rp| EditorItem { id: rp.filename.clone(), name: rp.name.clone(), version: String::new(), filename: rp.filename.clone(), description: None, homepage: None, sources: None, icon_path: None, is_checked: false, size: None, seed: None, last_played: None }).collect(),
                                    EditorType::ShaderPacks => updated_inst.shader_packs.iter().map(|sp| EditorItem { id: sp.filename.clone(), name: sp.name.clone(), version: String::new(), filename: sp.filename.clone(), description: None, homepage: None, sources: None, icon_path: None, is_checked: false, size: None, seed: None, last_played: None }).collect(),
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
                self.rebuild_sidebar_and_overview();
            }
            AppMsg::SelectInstance(index) => {
                self.selected_instance = Some(index);
                self.show_overview = false;
                self.sidebar.emit(SidebarInput::SetSelected(Some(index)));
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
                    // Lists are now handled by InstanceEditorTab which reads from instance record directly
                }
            }
            AppMsg::AddInstance => {
                self.add_instance_dialog.emit(AddInstanceInput::Open);
            }
            AppMsg::ShareInstance(idx) => {
                self.sharer_dialog.emit(SharerInput::Open(idx));
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
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Components,
                        "Edit Components".to_string(),
                        items,
                    ));
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
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Mods,
                        "Edit Mods".to_string(),
                        items,
                    ));
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
                            version: String::new(),
                            filename: rp.filename.clone(),
                            description: None,
                            homepage: None,
                            sources: None,
                            icon_path: None,
                            is_checked: false,
                            size: None,
                            seed: None,
                            last_played: None,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::ResourcePacks,
                        "Edit Resource Packs".to_string(),
                        items,
                    ));
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
                            description: None,
                            homepage: None,
                            sources: None,
                            icon_path: None,
                            is_checked: false,
                            size: None,
                            seed: None,
                            last_played: None,
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::ShaderPacks,
                        "Edit Shader Packs".to_string(),
                        items,
                    ));
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
                            }
                        })
                        .collect();
                    self.instance_editor.emit(EditorInput::Open(
                        EditorType::Worlds,
                        "Edit Worlds".to_string(),
                        items,
                    ));
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
                                    crate::frontend::utils::open_instance_subfolder(&inst.path, subfolder);
                                }
                            }
                            EditorOutput::BrowseModrinth(editor_type) => {
                                _sender.input(AppMsg::BrowseModrinth(editor_type));
                            }
                        }
                        // Refresh only the selected instance
                        _sender.input(AppMsg::RefreshSelectedInstance);
                        // Reselect - inline to avoid ambiguous self.update call
                        self.selected_instance = Some(index);
                        if let Some(_inst) = self.instances.get(index) {
                            // Lists are now handled by InstanceEditorTab
                        }
                    }
                }
            }
            AppMsg::OpenModsFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.path, "mods");
                }
            }
            AppMsg::OpenResourcePacksFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.path, "resourcepacks");
                }
            }
            AppMsg::OpenShaderPacksFolder => {
                if let Some(inst) = self.selected_instance.and_then(|i| self.instances.get(i)) {
                    crate::frontend::utils::open_instance_subfolder(&inst.path, "shaderpacks");
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
                        thread::spawn(move || {
                            let mods_dir = minecraft_dir.join("mods");
                            if !mods_dir.exists() {
                                let _ = std::fs::create_dir_all(&mods_dir);
                            }

                            let mut success_count = 0;
                            let mut last_error = None;
                            let installs_len = installs.len();
                            sender_clone.send(AppMsg::DownloadProgress(crate::backend::download::manager::DownloadMsg::Progress("Starting Mod Downloads...".to_string(), 0.0))).ok();

                            for (i, (project_id, version_id)) in installs.into_iter().enumerate() {
                                let target_version = if version_id.is_empty() {
                                    None
                                } else {
                                    Some(version_id)
                                };
                                sender_clone.send(AppMsg::DownloadProgress(crate::backend::download::manager::DownloadMsg::DetailedProgress {
                                    task: "Downloading Mod".to_string(),
                                    current: i + 1,
                                    total: installs_len,
                                    item_name: project_id.clone(),
                                    overall_progress: (i as f32) / (installs_len as f32),
                                })).ok();

                                match crate::backend::download::modrinth::install_mod_with_dependencies(&project_id, target_version, &gv, loader.clone(), &mods_dir) {
                                    Ok(_) => {
                                        eprintln!("Successfully installed mod {}", project_id);
                                        success_count += 1;
                                        // Refresh after each mod 
                                        sender_clone.send(AppMsg::RefreshSelectedInstance).ok();
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to install mod {}: {}", project_id, e);
                                        last_error = Some(e);
                                    }
                                }
                            }

                            if success_count > 0 {
                                sender_clone.send(AppMsg::DownloadProgress(crate::backend::download::manager::DownloadMsg::Finished)).ok();
                                sender_clone
                                    .send(AppMsg::ModrinthInstallResult(Ok(success_count)))
                                    .ok();
                                sender_clone.send(AppMsg::RefreshSelectedInstance).ok();
                            } else if let Some(e) = last_error {
                                sender_clone.send(AppMsg::DownloadProgress(crate::backend::download::manager::DownloadMsg::Error(e.clone()))).ok();
                                sender_clone
                                    .send(AppMsg::ModrinthInstallResult(Err(e)))
                                    .ok();
                            } else {
                                sender_clone.send(AppMsg::DownloadProgress(crate::backend::download::manager::DownloadMsg::Finished)).ok();
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
                                sender_clone.send(AppMsg::ConfirmRename(index, new_name)).unwrap();
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
                                sender_clone.send(AppMsg::ConfirmDelete(index)).unwrap();
                            }
                        },
                    );
                }
            }
            AppMsg::ChangeInstanceIcon(idx) => {
                // Open the icon chooser dialog instead of a raw file picker
                let default_icon = self.config.default_instance_icon.clone();
                let recents = self.config.recent_instance_icons.clone();
                self.icon_chooser.emit(IconChooserInput::Open(idx, default_icon, recents));
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
                    let mc_dir = inst.minecraft_dir.clone();
                    let inst_path = inst.path.clone();
                    let _ = std::fs::create_dir_all(&mc_dir);
                    let target = mc_dir.join("icon.png");
                    if std::fs::copy(&source_path, &target).is_ok() {
                        // Track in recents
                        self.config.recent_instance_icons.retain(|p| p != &source_path);
                        self.config.recent_instance_icons.insert(0, source_path);
                        self.config.recent_instance_icons.truncate(12);
                        let _ = self.config.save();
                        
                        // Targeted refresh instead of full scan
                        let sender_clone = _sender.input_sender().clone();
                        thread::spawn(move || {
                            if let Some(updated) = scan_single_instance(&inst_path) {
                                let _ = sender_clone.send(AppMsg::SelectedInstanceUpdated(updated));
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
                        self.rebuild_sidebar_and_overview();
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
                self.show_overview = true;
                self.rebuild_sidebar_and_overview();
            }
            AppMsg::AccountAction => {
                self.account_view.emit(AccountInput::Open);
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
            AppMsg::RefreshAccountsAll(new_config) => {
                // Replace accounts with the freshly-refreshed set
                self.config.accounts = new_config.accounts;
                let _ = self.config.save();
                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
            }
            AppMsg::OpenJavaSelector => {
                self.java_selector.emit(JavaSelectorInput::Open);
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
                    self.component_editor.emit(ComponentEditorInput::Open(
                        uid,
                        inst.minecraft_version.clone(),
                    ));
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
                        let start_time = std::time::Instant::now();
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

                                let mut reader = BufReader::new(stdout);
                                let mut err_reader = BufReader::new(stderr);

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

                                let mut err_line = String::new();
                                while let Ok(n) = err_reader.read_line(&mut err_line) {
                                    if n == 0 {
                                        break;
                                    }
                                    let _ = sender_clone.send(AppMsg::ConsoleLog(
                                        instance_path.clone(),
                                        format!("ERROR: {}", err_line),
                                    ));
                                    err_line.clear();
                                }

                                let _ = sender_clone.send(AppMsg::ConsoleLog(
                                    instance_path.clone(),
                                    "\nProcess finished.\n".to_string(),
                                ));
                                {
                                    let mut guard = game_process.lock().unwrap();
                                    *guard = None;
                                }
                                let duration = start_time.elapsed().as_secs();
                                let _ = sender_clone.send(AppMsg::ProcessFinished(
                                    instance_path.clone(),
                                    duration,
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
                        self.instance_summary.emit(SummaryInput::Update(inst, true));
                        self.instance_console.emit(ConsoleInput::Update(buf, true));
                    }
                }
            }
            AppMsg::ProcessFinished(path, duration) => {
                let is_active = Some(&path) == self.get_active_instance_path().as_ref();
                self.running_instances.remove(&path);

                // Re-scan the instance to update the playtime field from the disk if we just wrote it
                // Actually, let's update it in memory first, then call the backend update, AND refresh the sidebar/overview.
                if duration > 0 {
                    let _ = crate::backend::instance::manager::update_instance_playtime(&path, duration);
                }

                // Important: find and update the instance in self.instances so the UI sees the new playtime
                let mut found = false;
                for inst in &mut self.instances {
                    if inst.path == path {
                        inst.total_time_played += duration;
                        found = true;
                        break;
                    }
                }

                if found {
                    // Update persistent playtime tracker
                    let instance_id = self.instances.iter().find(|i| i.path == path).map(|i| i.id.clone()).unwrap_or_default();
                    if !instance_id.is_empty() {
                        self.playtime_manager.add_playtime(&instance_id, duration);
                    }

                    self.config.total_playtime += duration;
                    let _ = self.config.save();
                    self.rebuild_sidebar_and_overview();
                }

                if is_active {
                    let inst = self.instances.get(self.selected_instance.unwrap()).cloned();
                    self.instance_summary.emit(SummaryInput::Update(inst, false));
                    self.instance_console.emit(ConsoleInput::Update(
                        self.get_active_console_buffer(),
                        false,
                    ));
                    
                    // Re-scan single instance to make sure we're fully in sync
                    if let Some(inst) = scan_single_instance(&path) {
                        if let Some(selected_idx) = self.selected_instance {
                             if let Some(current_inst) = self.instances.get_mut(selected_idx) {
                                 if current_inst.path == path {
                                     *current_inst = inst.clone();
                                 }
                             }
                        }
                        self.instance_summary.emit(SummaryInput::Update(Some(inst), false));
                    }
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
            AppMsg::InstanceCreated(_version) => {
                if let Some(path) = &self.config.instances_path {
                    let path_clone = path.clone();
                    let sender_clone = _sender.input_sender().clone();
                    thread::spawn(move || {
                        let insts = scan_instances(&path_clone);
                        let _ = sender_clone.send(AppMsg::InstancesScanned(insts));
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
                        "Starting download...".to_string(),
                        0.0,
                        true,
                    ));

                let data_path = self.config.minecraft_data_path.clone();
                let sender_clone = _sender.input_sender().clone();

                thread::spawn(move || {
                    let (tx, rx) = std::sync::mpsc::channel::<DownloadMsg>();

                    let sender_proxy = sender_clone.clone();
                    let raw_v = raw_version.clone();
                    let d_path = data_path.clone();
                    let dl_loader = loader.clone();
                    let dl_loader_ver = loader_version.clone();

                    // Start downloader in another thread to allow forwarding rx
                    thread::spawn(move || {
                        if let Err(e) = download_minecraft_data(
                            &raw_v,
                            &dl_loader,
                            dl_loader_ver.as_deref(),
                            &d_path,
                            &tx,
                        ) {
                            let _ = tx.send(DownloadMsg::Error(e));
                        }
                    });

                    // Forward messages
                    while let Ok(msg) = rx.recv() {
                        let is_finished =
                            matches!(msg, DownloadMsg::Finished | DownloadMsg::Error(_));
                        let app_msg = AppMsg::DownloadProgress(msg);
                        if sender_proxy.send(app_msg).is_err() {
                            break;
                        }
                        if is_finished {
                            break;
                        }
                    }
                });
            }
            AppMsg::DownloadProgress(msg) => match msg {
                DownloadMsg::Progress(status, progress) => {
                    self.download_dialog
                        .emit(DownloadDialogInput::UpdateStatus(status.clone(), progress));
                    self.download_status_bar
                        .emit(DownloadStatusBarInput::Update(status.clone(), progress, true));
                    self.instance_editor
                        .emit(EditorInput::DownloadProgress(status, progress, true));
                }
                DownloadMsg::DetailedProgress {
                    task,
                    current,
                    total,
                    item_name,
                    overall_progress,
                } => {
                    self.download_dialog
                        .emit(DownloadDialogInput::UpdateDetailed {
                            task: task.clone(),
                            current,
                            total,
                            item_name: item_name.clone(),
                            progress: overall_progress,
                        });
                    let label = format!("{}: {}", task, item_name);
                    self.download_status_bar
                        .emit(DownloadStatusBarInput::Update(
                            label.clone(),
                            overall_progress,
                            true,
                        ));
                    self.instance_editor
                        .emit(EditorInput::DownloadProgress(
                            label,
                            overall_progress,
                            true,
                        ));
                }
                DownloadMsg::Error(e) => {
                    _sender.input(AppMsg::DownloadError(e));
                }
                DownloadMsg::Finished => {
                    _sender.input(AppMsg::DownloadFinished);
                }
            },
            AppMsg::DownloadFinished => {
                self.download_dialog.emit(DownloadDialogInput::Close);
                self.verifying_loading = false;
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(false));

                let fin_msg = "Download finished".to_string();
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        fin_msg.clone(),
                        1.0,
                        false,
                    ));
                self.instance_editor
                    .emit(EditorInput::DownloadProgress(
                        fin_msg,
                        1.0,
                        false,
                    ));

                if self.launch_after_download {
                    self.launch_after_download = false;
                    _sender.input(AppMsg::LaunchInstance);
                }
            }
            AppMsg::DownloadError(err) => {
                self.download_dialog.emit(DownloadDialogInput::Close);
                self.launch_after_download = false;
                self.verifying_loading = false;
                self.instance_summary.emit(SummaryInput::SetVerifyingLoading(false));

                let err_msg = format!("Error: {}", err);
                self.download_status_bar
                    .emit(DownloadStatusBarInput::Update(
                        err_msg.clone(),
                        0.0,
                        true,
                    ));
                self.instance_editor
                    .emit(EditorInput::DownloadProgress(
                        err_msg,
                        0.0,
                        true,
                    ));
            }
            AppMsg::OpenDownloadDetails => {
                self.download_dialog.emit(DownloadDialogInput::Show);
            }
        }
    }
}


// ─── AppModel helper methods ──────────────────────────────────────────────────

impl AppModel {
    /// Re-emit the current instances + groups to the sidebar and overview grid.
    fn show_move_to_group_dialog(&self, sender: &ComponentSender<AppModel>, idx: usize) {
        let dialog_win = adw::Window::builder()
            .title("Move to Group")
            .default_width(380)
            .modal(true)
            .transient_for(&self.window)
            .resizable(false)
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
            cancel_btn.connect_clicked(move |_| dw.close());
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
        dialog_win.set_content(Some(&content));
        dialog_win.present();
    }

    fn rebuild_sidebar_and_overview(&self) {
        self.sidebar.emit(SidebarInput::Rebuild(
            self.instances.clone(),
            self.groups.clone(),
        ));
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
}
