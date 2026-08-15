use crate::backend::auth::microsoft::Account;
use crate::backend::download::manager::DownloadMsg;
use crate::backend::instance::manager::{Instance, ModLoader};
use crate::backend::playtime::PlaytimeManager;
use crate::backend::runtime::versions::{MinecraftVersion, RawVersion};
use crate::config::{Config, SortBy};
use crate::frontend::dialogs::instance::components::ComponentEditorOutput;
use crate::frontend::dialogs::instance::editor::{EditorOutput, EditorType, ModUpdateInfo};
use crate::frontend::dialogs::instance::mod_loader::ModLoaderDialogOutput;
use crate::frontend::views::discover::DiscoverOutput;
use crate::frontend::views::instance::{
    ConsoleOutput, EditorTabOutput, SettingsTabOutput, SummaryOutput,
};
use crate::frontend::views::library::{LayoutMode, OverviewOutput};
use crate::frontend::views::sidebar::SidebarOutput;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceStatus {
    NotRunning,
    Loading,
    Running,
}

#[derive(Debug)]
pub enum AppMsg {
    OpenSettings,
    OpenSetup,
    OpenAbout,
    OpenShortcuts,
    OpenAssetManager,
    RefreshAssets,
    AssetsReady(crate::backend::download::assets::AssetScanResult),
    AssetSubpageChanged(Option<String>),
    OpenPlaytime,
    RefreshPlaytime,
    PlaytimeDataReady(PlaytimeManager, Vec<(String, String, u64, bool)>), // manager, (id, name, seconds, is_detected)
    ConfigUpdated(Config),
    SelectInstance(usize),
    AddInstance(Option<String>),
    HeaderAddInstance,
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
    InstallBrowserItems(EditorType, Vec<(String, String)>), // (EditorType, (Project ID, Version ID))
    ModrinthInstallResult(EditorType, Result<usize, String>), // Type and number of items installed or error
    ModUpdatesResult(Result<Vec<ModUpdateInfo>, String>),
    ModUpdateSuccess(String), // filename
    ModUpdateAllSuccess(Vec<String>),

    // Instance management
    RenameInstanceRequest(usize),
    DeleteInstanceRequest(usize),
    InstanceCreated(MinecraftVersion, std::path::PathBuf, Option<String>),
    ConfirmDelete(usize),
    ConfirmRename(usize, String),
    InstancesUpdated(Vec<Instance>),
    /// Open the native file picker for an instance icon (from the icon chooser).
    ChangeInstanceIconFromFile(usize),
    /// Apply the global default icon to an instance.
    ApplyDefaultIcon(usize),
    /// Apply a specific icon file to an instance (e.g. from recents).
    ApplyIconPath(usize, PathBuf),

    ToggleDiscoverSearch,
    DiscoverSearchChanged(String),
    RefreshDiscover,

    // Sidebar / group management
    SidebarEvent(SidebarOutput),
    DiscoverEvent(DiscoverOutput),
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
    SwitchAccount(String),               // UUID
    RemoveAccount(String),               // UUID
    AddOfflineAccount(String),           // Username
    VerifyAccount(String),               // UUID — now handled in AccountView, kept for compat
    VerifyAccountResult(String, String), // UUID, status message
    RefreshAccount(String),              // UUID — now handled in AccountView, kept for compat
    RefreshAccountResult(Result<Account, String>),
    RefreshAccountsAll(Config), // Full config with all refreshed accounts
    RefreshAccountsRequest,
    ShowAddOfflineDialog,
    OpenAccountSettings,

    // Launching
    LaunchInstance,
    LaunchInstanceFromIndex(usize),
    VerifyInstance,
    KillInstance,
    KillInstanceFromIndex(usize),
    ConsoleLog(PathBuf, String),
    ClearConsole(PathBuf),
    SetConsoleSearchQuery(String),
    ProcessFinished(
        PathBuf,
        u64,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    ),
    SwitchTab(String),

    // Performance Tweaks
    SetInstanceFeralGameMode(bool),
    SetInstanceDiscreteGpu(bool),
    SetInstanceZinkVulkan(bool),
    SetInstanceUseWayland(bool),

    // Downloading
    DownloadStart(RawVersion, ModLoader, Option<String>),
    DownloadProgress(DownloadMsg),
    RemoveJob(String),
    ClearFinishedJobs,
    RetryJob(String),
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
    SetOverviewSortBy(SortBy),
    InstanceLaunched(PathBuf, u64),
}
