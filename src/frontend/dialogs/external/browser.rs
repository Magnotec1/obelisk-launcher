#![allow(unused_assignments)]

use crate::backend::download::manager::{
    clear_modrinth_caches as clear_caches, fetch_modrinth_project as get_project,
    fetch_modrinth_versions as get_project_versions, search_modrinth_mods as search_mods,
    ModProject as ModrinthProject, ModVersion as ModrinthVersion,
};
use crate::backend::instance::manager::ModLoader;
use crate::frontend::dialogs::instance::editor::EditorType;
use adw::prelude::*;
use gtk::gdk;
use gtk::glib;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

fn escape(text: &str) -> String {
    glib::markup_escape_text(text).to_string()
}

fn format_downloads(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

// ---------------------------------------------------------------------------
// Generic Browser Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct BrowserProject {
    pub project_id: String,
    pub title: String,
    pub author: Option<String>,
    pub description: String,
    pub body: Option<String>,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub source_url: Option<String>,
    pub wiki_url: Option<String>,
    pub discord_url: Option<String>,
    pub license_name: Option<String>,
    pub screenshots: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct BrowserVersion {
    pub id: String,
    pub name: String,
    pub version_number: String,
    pub version_type: String,
    pub game_versions: Vec<String>,
    pub downloads: u64,
    pub files: Vec<BrowserFile>,
}

#[derive(Debug, Clone)]
pub struct BrowserFile {
    pub filename: String,
    pub url: String,
    pub primary: bool,
}

#[derive(Debug, Clone)]
pub struct BrowserSearchResult {
    pub hits: Vec<BrowserProject>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

impl From<ModrinthProject> for BrowserProject {
    fn from(p: ModrinthProject) -> Self {
        Self {
            project_id: p.project_id,
            title: p.title,
            author: p.author,
            description: p.description,
            body: p.body,
            icon_url: p.icon_url,
            downloads: p.downloads,
            follows: p.follows,
            source_url: p.source_url,
            wiki_url: p.wiki_url,
            discord_url: p.discord_url,
            license_name: p.license_name,
            screenshots: p.gallery.unwrap_or_default().into_iter().map(|img| img.url).collect(),
            categories: p.categories,
        }
    }
}

impl From<ModrinthVersion> for BrowserVersion {
    fn from(v: ModrinthVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            version_number: v.version_number,
            version_type: v.version_type,
            game_versions: v.game_versions,
            downloads: v.downloads,
            files: v.files.into_iter().map(|f| BrowserFile {
                filename: f.filename,
                url: f.url,
                primary: f.primary,
            }).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// Sourcing Abstraction Trait
// ---------------------------------------------------------------------------

pub trait BrowserSource: Send + Sync + 'static {
    fn name(&self) -> &str;

    fn search(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        game_version: &str,
        loader: ModLoader,
        editor_type: &EditorType,
    ) -> Result<BrowserSearchResult, String>;

    fn get_project(&self, id: &str) -> Result<BrowserProject, String>;

    fn get_project_versions(
        &self,
        id: &str,
        game_version: &str,
        loader: ModLoader,
    ) -> Result<Vec<BrowserVersion>, String>;
}

pub struct ModrinthSource;

impl BrowserSource for ModrinthSource {
    fn name(&self) -> &str {
        "Modrinth"
    }

    fn search(
        &self,
        query: &str,
        limit: u32,
        offset: u32,
        game_version: &str,
        loader: ModLoader,
        editor_type: &EditorType,
    ) -> Result<BrowserSearchResult, String> {
        let project_type = match editor_type {
            EditorType::Mods => "mod",
            EditorType::ResourcePacks => "resourcepack",
            EditorType::ShaderPacks => "shader",
            EditorType::Worlds => "world",
            _ => "mod",
        };

        let effective_loader = if matches!(editor_type, EditorType::Mods) {
            loader
        } else {
            ModLoader::None
        };

        let res = search_mods(query, limit, offset, Some(game_version), Some(effective_loader), Some(project_type))?;
        Ok(BrowserSearchResult {
            hits: res.hits.into_iter().map(BrowserProject::from).collect(),
            offset: res.offset,
            limit: res.limit,
            total_hits: res.total_hits,
        })
    }

    fn get_project(&self, id: &str) -> Result<BrowserProject, String> {
        let p = get_project(id)?;
        Ok(BrowserProject::from(p))
    }

    fn get_project_versions(
        &self,
        id: &str,
        game_version: &str,
        loader: ModLoader,
    ) -> Result<Vec<BrowserVersion>, String> {
        let versions = get_project_versions(id, Some(game_version), Some(loader))?;
        Ok(versions.into_iter().map(BrowserVersion::from).collect())
    }
}

// ---------------------------------------------------------------------------
// Queue item
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QueueItem {
    pub title: String,
    pub version_id: Option<String>,
    pub version_name: Option<String>,
    pub filename: Option<String>,
}

// ---------------------------------------------------------------------------
// Factories
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct ProjectRow {
    pub project: BrowserProject,
    pub in_queue: bool,
    pub icon: Option<gdk::Texture>,
}

#[derive(Debug)]
pub enum ProjectRowInput {
    AddToQueue,
    OpenDetails,
}

#[derive(Debug)]
pub enum ProjectRowOutput {
    AddToQueue(String, String),
    OpenDetails(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for ProjectRow {
    type Init = (BrowserProject, bool, Option<gdk::Texture>);
    type Input = ProjectRowInput;
    type Output = ProjectRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            #[watch]
            set_title: &escape(&self.project.title),
            set_title_lines: 1,
            #[watch]
            set_subtitle: &{
                let author = self.project.author.as_deref().unwrap_or("Unknown");
                let dls = format_downloads(self.project.downloads);
                escape(&format!("by {} \u{b7} {} downloads", author, dls))
            },
            set_subtitle_lines: 1,
            set_activatable: true,
            set_use_markup: true,

            add_prefix = &gtk::Stack {
                set_hhomogeneous: true,
                set_vhomogeneous: true,

                add_named[Some("icon")] = &gtk::Image {
                    #[watch]
                    set_paintable: self.icon.as_ref().map(|t| t as &gdk::Texture),
                    set_pixel_size: 40,
                },

                add_named[Some("fallback")] = &gtk::Image {
                    set_icon_name: Some("package-x-generic-symbolic"),
                    set_pixel_size: 40,
                },

                // Must come after add_named so children exist on first render
                #[watch]
                set_visible_child_name: if self.icon.is_some() { "icon" } else { "fallback" },
            },

            add_suffix = &gtk::Button {
                #[watch]
                set_icon_name: if self.in_queue { "list-remove-symbolic" } else { "list-add-symbolic" },
                #[watch]
                set_tooltip_text: if self.in_queue { Some("Remove from Queue") } else { Some("Add to Queue") },
                set_css_classes: &["flat", "circular"],
                set_valign: gtk::Align::Center,
                connect_clicked => ProjectRowInput::AddToQueue,
            },

            connect_activated => ProjectRowInput::OpenDetails,
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            project: init.0,
            in_queue: init.1,
            icon: init.2,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            ProjectRowInput::AddToQueue => {
                sender
                    .output(ProjectRowOutput::AddToQueue(
                        self.project.project_id.clone(),
                        self.project.title.clone(),
                    ))
                    .ok();
            }
            ProjectRowInput::OpenDetails => {
                sender
                    .output(ProjectRowOutput::OpenDetails(
                        self.project.project_id.clone(),
                    ))
                    .ok();
            }
        }
    }
}

#[derive(Debug)]
pub struct VersionRow {
    pub version: BrowserVersion,
    pub project_id: String,
}

#[derive(Debug)]
pub enum VersionRowOutput {
    Install(String, String),
    AddToQueue(String, String, String, String), // pid, vid, vname, filename
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (BrowserVersion, String);
    type Input = ();
    type Output = VersionRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &escape(&self.version.name),
            set_title_lines: 1,
            set_subtitle_lines: 1,
            set_subtitle: &escape(&format!(
                "{} · {} · {}",
                self.version.version_number,
                self.version.version_type,
                &self.version.game_versions.join(", ")
            )),

            add_suffix = &gtk::Box {
                set_spacing: 6,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    set_label: &format_downloads(self.version.downloads),
                    set_css_classes: &["dim-label", "caption"],
                    set_tooltip_text: Some("Downloads"),
                },

                gtk::Button {
                    set_label: "Install Now",
                    set_css_classes: &["flat", "accent"],
                    set_valign: gtk::Align::Center,
                    connect_clicked[sender, pid = self.project_id.clone(), vid = self.version.id.clone()] => move |_| {
                        sender.output(VersionRowOutput::Install(pid.clone(), vid.clone())).ok();
                    }
                },
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            version: init.0,
            project_id: init.1,
        }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

pub struct QueueRow {
    pub project_id: String,
    pub item: QueueItem,
    pub versions: Vec<BrowserVersion>,
    pub string_list: gtk::StringList,
}

#[derive(Debug)]
pub enum QueueRowInput {
    SetVersions(Vec<BrowserVersion>),
    Select(u32),
}

#[derive(Debug)]
pub enum QueueRowOutput {
    Remove(String),
    SelectVersion(String, String, String, String), // id, vid, vname, fname
    FetchVersions(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for QueueRow {
    type Init = (String, QueueItem, Vec<BrowserVersion>);
    type Input = QueueRowInput;
    type Output = QueueRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        adw::ComboRow {
            #[watch]
            set_title: &escape(&self.item.title),
            set_enable_search: true,
            set_model: Some(&self.string_list),
            #[watch]
            set_selected: self.get_selected_index(),

            connect_selected_notify[sender] => move |row| {
                let idx = row.selected();
                sender.input(QueueRowInput::Select(idx));
            },

            add_suffix = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_valign: gtk::Align::Center,
                set_spacing: 6,

                gtk::Button {
                    set_icon_name: "list-remove-symbolic",
                    set_css_classes: &["flat", "circular"],
                    set_tooltip_text: Some("Remove from queue"),
                    connect_clicked[sender, pid = self.project_id.clone()] => move |_| {
                        sender.output(QueueRowOutput::Remove(pid.clone())).ok();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let strings: Vec<String> = init
            .2
            .iter()
            .map(|v| format!("{} · {}", v.name, v.version_number))
            .collect();
        let refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        let string_list = gtk::StringList::new(&refs);

        Self {
            project_id: init.0,
            item: init.1,
            versions: init.2,
            string_list,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            QueueRowInput::SetVersions(versions) => {
                self.versions = versions;
                let strings: Vec<String> = self
                    .versions
                    .iter()
                    .map(|v| format!("{} · {}", v.name, v.version_number))
                    .collect();
                let refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
                self.string_list
                    .splice(0, self.string_list.n_items(), &refs);
            }
            QueueRowInput::Select(idx) => {
                if let Some(v) = self.versions.get(idx as usize) {
                    if self.item.version_id.as_ref() != Some(&v.id) {
                        let filename = v
                            .files
                            .iter()
                            .find(|f| f.primary)
                            .or_else(|| v.files.first())
                            .map(|f| f.filename.clone())
                            .unwrap_or_default();
                        self.item.version_id = Some(v.id.clone());
                        self.item.version_name = Some(v.name.clone());
                        self.item.filename = Some(filename.clone());
                        sender
                            .output(QueueRowOutput::SelectVersion(
                                self.project_id.clone(),
                                v.id.clone(),
                                v.name.clone(),
                                filename,
                            ))
                            .ok();
                    }
                }
            }
        }
    }
}

impl QueueRow {
    fn get_selected_index(&self) -> u32 {
        self.item
            .version_id
            .as_ref()
            .and_then(|vid| self.versions.iter().position(|v| v.id == *vid))
            .unwrap_or(0) as u32
    }
}

pub struct ScreenshotCard {
    pub texture: gdk::Texture,
}

#[relm4::factory(pub)]
impl FactoryComponent for ScreenshotCard {
    type Init = gdk::Texture;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = adw::Carousel;

    view! {
        #[root]
        gtk::Picture {
            set_paintable: Some(&self.texture),
            set_can_shrink: true,
            set_valign: gtk::Align::Center,
            set_halign: gtk::Align::Center,
            set_hexpand: true,
            set_vexpand: true,
            add_css_class: "screenshot-img",
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { texture: init }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BrowserInput {
    Open {
        game_version: String,
        loader: ModLoader,
        editor_type: EditorType,
    },
    Close,

    GoBack,
    StartSearch(String),
    Search(String),
    SearchDone(Result<BrowserSearchResult, String>),
    LoadMore,
    LoadMoreDone(Result<BrowserSearchResult, String>),
    Refresh,

    ClearQueue,
    PromptInstallQueue,
    ConfirmInstallQueue,
    OpenQueueDialog,

    ToggleQueueFromDetails,
    ShowDetails(String),
    ShowList,
    DetailsLoaded(Result<(BrowserProject, Vec<BrowserVersion>), String>),

    ToggleQueueItem(String, String),

    OpenProjectDetails(String),
    FetchVersions(String),
    VersionsFetched(String, Result<Vec<BrowserVersion>, String>),
    ApplyVersionToQueue(String, String, String, String),
    InstallProject(String, String),
    AddVersionToQueue(String, String, String, String),
    IconLoaded(String, gdk::Texture),
    ScreenshotLoaded(String, gdk::Texture),
    RemoveQueueItem(String),
    ShowToast(String),

    SetCollapsed(bool),
    ToggleSidebar,
    OpenUrl(String),
    ScreenshotScroll(f64),
    ShowFullDescription,
}

#[derive(Debug)]
pub enum BrowserOutput {
    InstallItems {
        editor_type: EditorType,
        installs: Vec<(String, String)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserView {
    Overview,
    Results,
    Details,
}

pub struct UnifiedBrowser {
    visible: bool,
    game_version: String,
    loader: ModLoader,
    editor_type: EditorType,
    source: Arc<dyn BrowserSource + Send + Sync>,

    search_query: String,
    loading: bool,
    loading_more: bool,
    offset: u32,
    total_hits: u32,
    error: Option<String>,

    download_queue: HashMap<String, QueueItem>,
    icon_cache: HashMap<String, gdk::Texture>,
    screenshot_cache: HashMap<String, gdk::Texture>,
    author_cache: HashMap<String, String>,

    selected_project: Option<BrowserProject>,
    loading_details: bool,

    view_state: BrowserView,
    queue_dialog: Controller<QueueDialog>,
    description_dialog: Controller<DescriptionDialog>,
    toast_overlay: adw::ToastOverlay,

    projects: FactoryVecDeque<ProjectRow>,
    versions: FactoryVecDeque<VersionRow>,
    screenshot_cards: FactoryVecDeque<ScreenshotCard>,

    collapsed: bool,
    show_sidebar: bool,
}

#[relm4::component(pub)]
impl Component for UnifiedBrowser {
    type Init = ();
    type Input = BrowserInput;
    type Output = BrowserOutput;
    type CommandOutput = ();

    view! {
        adw::Dialog {
            #[watch]
            set_title: match model.editor_type {
                EditorType::Mods => "Browse Mods",
                EditorType::ResourcePacks => "Browse Resource Packs",
                EditorType::ShaderPacks => "Browse Shader Packs",
                EditorType::Worlds => "Browse Worlds",
                _ => "Browse Resources",
            },
            set_content_width: 950,
            set_content_height: 700,
            set_width_request: 360,
            set_height_request: 320,
            set_can_close: true,

            #[name = "toast_overlay"]
            #[wrap(Some)]
            set_child = &adw::ToastOverlay {
                adw::ToolbarView {
                    #[wrap(Some)]
                    #[name = "split_view"]
                    set_content = &adw::NavigationSplitView {
                        #[watch]
                        set_show_content: !model.show_sidebar,
                        set_vexpand: true,
                        set_min_sidebar_width: 260.0,
                        set_max_sidebar_width: 500.0,
                        connect_collapsed_notify[sender] => move |split| {
                            sender.input(BrowserInput::SetCollapsed(split.is_collapsed()));
                        },

                    #[wrap(Some)]
                    set_sidebar = &adw::NavigationPage {
                        set_title: "Browser",
                        #[wrap(Some)]
                        set_child = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                set_css_classes: &["flat"],
                                #[wrap(Some)]
                                set_title_widget = &adw::WindowTitle {
                                    set_title: "Browser",
                                    #[watch]
                                    set_subtitle: model.source.name(),
                                },
                                pack_end = &gtk::Button {
                                    set_icon_name: "view-refresh-symbolic",
                                    set_tooltip_text: Some("Refresh (clear cache)"),
                                    connect_clicked => BrowserInput::Refresh,
                                    #[watch]
                                    set_visible: model.collapsed,
                                },
                            },

                            #[wrap(Some)]
                            set_content = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,

                                gtk::SearchEntry {
                                    set_margin_all: 8,
                                    #[watch]
                                    set_placeholder_text: Some(&format!("Search {}...", model.source.name())),
                                    #[watch]
                                    set_text: &model.search_query,
                                    connect_activate[sender] => move |entry| {
                                        sender.input(BrowserInput::Search(entry.text().to_string()));
                                    },
                                },

                                gtk::ScrolledWindow {
                                    set_vexpand: true,
                                    set_hscrollbar_policy: gtk::PolicyType::Never,
                                    connect_edge_reached[sender] => move |_, pos| {
                                        if pos == gtk::PositionType::Bottom {
                                            sender.input(BrowserInput::LoadMore);
                                        }
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 12,
                                        set_margin_all: 12,

                                        adw::StatusPage {
                                            #[watch]
                                            set_visible: model.loading,
                                            set_title: "Searching...",
                                            #[wrap(Some)]
                                            set_child = &adw::Spinner {
                                                set_halign: gtk::Align::Center,
                                                set_width_request: 32,
                                                set_height_request: 32,
                                            }
                                        },

                                        adw::StatusPage {
                                            #[watch]
                                            set_visible: !model.loading && model.projects.is_empty() && model.error.is_none() && !model.search_query.is_empty(),
                                            set_title: "No Results Found",
                                            set_description: Some("Try a different search term."),
                                            set_icon_name: Some("system-search-symbolic"),
                                        },

                                        adw::StatusPage {
                                            #[watch]
                                            set_visible: model.error.is_some(),
                                            #[watch]
                                            set_title: if model.error.as_deref().unwrap_or("").contains("No internet") { "No Internet Connection" } else { "Error fetching data" },
                                            #[watch]
                                            set_description: model.error.as_deref(),
                                            #[watch]
                                            set_icon_name: Some(if model.error.as_deref().unwrap_or("").contains("No internet") { "network-offline-symbolic" } else { "dialog-error-symbolic" }),
                                        },

                                        #[local_ref]
                                        projects_list -> gtk::ListBox {
                                            #[watch]
                                            set_visible: !model.loading && !model.projects.is_empty(),
                                            set_selection_mode: gtk::SelectionMode::None,
                                            set_css_classes: &["boxed-list"],
                                        },

                                        gtk::Button {
                                            #[watch]
                                            set_visible: !model.loading && !model.loading_more && !model.projects.is_empty() && (model.projects.len() as u32) < model.total_hits,
                                            set_label: "Load More",
                                            set_css_classes: &["pill", "suggested-action"],
                                            set_halign: gtk::Align::Center,
                                            set_margin_top: 12,
                                            set_margin_bottom: 12,
                                            connect_clicked => BrowserInput::LoadMore,
                                        },

                                        gtk::Spinner {
                                            #[watch]
                                            set_visible: model.loading_more,
                                            set_spinning: true,
                                            set_halign: gtk::Align::Center,
                                            set_margin_top: 12,
                                            set_margin_bottom: 12,
                                        }
                                    }
                                },
                            },
                        },
                    },

                    #[wrap(Some)]
                    set_content = &adw::NavigationPage {
                        #[watch]
                        set_title: match model.view_state {
                            BrowserView::Details => "Details",
                            _ => "Browser Overview",
                        },
                        #[wrap(Some)]
                        set_child = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                set_css_classes: &["flat"],
                                set_show_title: false,
                                set_show_back_button: false,
                                pack_start = &gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 6,
                                    gtk::Button {
                                        set_icon_name: "go-previous-symbolic",
                                        set_tooltip_text: Some("Back"),
                                        connect_clicked => BrowserInput::GoBack,
                                        #[watch]
                                        set_visible: model.view_state == BrowserView::Details,
                                    },
                                },
                                pack_end = &gtk::Button {
                                    set_icon_name: "view-refresh-symbolic",
                                    set_tooltip_text: Some("Refresh (clear cache)"),
                                    connect_clicked => BrowserInput::Refresh,
                                },
                            },
                            #[wrap(Some)]
                            set_content = &gtk::Stack {
                                set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                                // --- 1. Overview Page ---
                                add_named[Some("overview")] = &adw::NavigationPage {
                                    set_title: "Overview",
                                    #[wrap(Some)]
                                    set_child = &adw::ToolbarView {

                                        #[wrap(Some)]
                                        set_content = &gtk::ScrolledWindow {
                                            set_hscrollbar_policy: gtk::PolicyType::Never,
                                            set_vscrollbar_policy: gtk::PolicyType::Automatic,

                                            adw::StatusPage {
                                                set_vexpand: true,
                                                #[watch]
                                                set_title: match model.editor_type {
                                                    EditorType::Mods => "Mod Browser",
                                                    EditorType::ResourcePacks => "Resource Pack Browser",
                                                    EditorType::ShaderPacks => "Shader Pack Browser",
                                                    EditorType::Worlds => "World Browser",
                                                    _ => "Resource Browser",
                                                },
                                                #[watch]
                                                set_description: Some(&format!("Search and discover content for your instance from {}.", model.source.name())),
                                                set_icon_name: Some("web-browser-symbolic"),
                                            }
                                        }
                                    }
                                },

                                // --- 2. Details Page ---
                                add_named[Some("details")] = &adw::NavigationPage {
                                    #[watch]
                                    set_title: model.selected_project.as_ref().map(|p| p.title.as_str()).unwrap_or("Details"),
                                    #[wrap(Some)]
                                    set_child = &gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,

                                        gtk::ScrolledWindow {
                                            set_vexpand: true,
                                            set_hscrollbar_policy: gtk::PolicyType::Never,

                                            gtk::Box {
                                                set_orientation: gtk::Orientation::Vertical,
                                                set_spacing: 24,
                                                set_margin_all: 20,

                                                adw::StatusPage {
                                                    #[watch]
                                                    set_visible: model.loading_details,
                                                    set_title: "Loading Details...",
                                                    #[wrap(Some)]
                                                    set_child = &gtk::Spinner {
                                                        set_spinning: true,
                                                        set_halign: gtk::Align::Center,
                                                    }
                                                },

                                                // --- Detail content ---
                                                gtk::Box {
                                                    #[watch]
                                                    set_visible: !model.loading_details && model.selected_project.is_some(),
                                                    set_orientation: gtk::Orientation::Vertical,
                                                    set_spacing: 24,

                                                    // Header: icon + title + author + add-to-queue
                                                    gtk::Box {
                                                        set_spacing: 20,
                                                        gtk::Stack {
                                                            set_hhomogeneous: true,
                                                            set_vhomogeneous: true,

                                                            add_named[Some("icon")] = &gtk::Image {
                                                                #[watch]
                                                                set_paintable: model.selected_project.as_ref().and_then(|p| p.icon_url.as_ref()).and_then(|url| model.icon_cache.get(url)).map(|t| t as &gdk::Texture),
                                                                set_pixel_size: 96,
                                                                set_css_classes: &["icon-dropshadow"],
                                                            },

                                                            add_named[Some("fallback")] = &gtk::Image {
                                                                set_icon_name: Some("package-x-generic-symbolic"),
                                                                set_pixel_size: 96,
                                                                set_css_classes: &["icon-dropshadow"],
                                                            },

                                                            // Must come after add_named so children exist on first render
                                                            #[watch]
                                                            set_visible_child_name: if model.selected_project.as_ref().and_then(|p| p.icon_url.as_ref()).and_then(|url| model.icon_cache.get(url)).is_some() { "icon" } else { "fallback" },
                                                        },
                                                        gtk::Box {
                                                            set_orientation: gtk::Orientation::Vertical,
                                                            set_valign: gtk::Align::Center,
                                                            set_spacing: 4,
                                                            set_hexpand: true,
                                                            gtk::Label {
                                                                #[watch]
                                                                set_label: &escape(model.selected_project.as_ref().map(|p| p.title.as_str()).unwrap_or("")),
                                                                set_css_classes: &["title-1"],
                                                                set_halign: gtk::Align::Start,
                                                                set_use_markup: true,
                                                                set_wrap: true,
                                                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                                            },
                                                            gtk::Label {
                                                                #[watch]
                                                                set_label: &{
                                                                    let pid = model.selected_project.as_ref().map(|p| p.project_id.as_str()).unwrap_or("");
                                                                    let author = model.selected_project.as_ref()
                                                                        .and_then(|p| p.author.as_deref())
                                                                        .or_else(|| model.author_cache.get(pid).map(|s| s.as_str()))
                                                                        .unwrap_or("Unknown author");
                                                                    format!("by {}", escape(author))
                                                                },
                                                                set_css_classes: &["dim-label"],
                                                                set_halign: gtk::Align::Start,
                                                                set_use_markup: true,
                                                                set_wrap: true,
                                                                set_wrap_mode: gtk::pango::WrapMode::WordChar,
                                                            }
                                                        },

                                                        gtk::Button {
                                                            #[watch]
                                                            set_icon_name: if model.selected_project.as_ref().map(|p| model.download_queue.contains_key(&p.project_id)).unwrap_or(false) { "list-remove-symbolic" } else { "list-add-symbolic" },
                                                            #[watch]
                                                            set_tooltip_text: Some(if model.selected_project.as_ref().map(|p| model.download_queue.contains_key(&p.project_id)).unwrap_or(false) { "Remove from Queue" } else { "Add to Queue" }),
                                                            set_valign: gtk::Align::Center,
                                                            set_css_classes: &["pill"],
                                                            connect_clicked => BrowserInput::ToggleQueueFromDetails,
                                                        }
                                                    },

                                                    // Stats pills row
                                                    gtk::Box {
                                                        set_spacing: 8,
                                                        set_halign: gtk::Align::Start,

                                                        gtk::Label {
                                                            #[watch]
                                                            set_label: &format!("{} downloads", format_downloads(model.selected_project.as_ref().map(|p| p.downloads).unwrap_or(0))),
                                                            set_css_classes: &["pill-badge"],
                                                        },
                                                        gtk::Label {
                                                            #[watch]
                                                            set_label: &format!("{} likes", format_downloads(model.selected_project.as_ref().map(|p| p.follows).unwrap_or(0))),
                                                            set_css_classes: &["pill-badge"],
                                                        },
                                                    },

                                                    // Screenshot Gallery Carousel
                                                    gtk::Box {
                                                        set_orientation: gtk::Orientation::Vertical,
                                                        set_spacing: 6,
                                                        #[watch]
                                                        set_visible: !model.screenshot_cards.is_empty(),

                                                        gtk::Overlay {
                                                            #[local_ref]
                                                            screenshot_carousel -> adw::Carousel {
                                                                set_height_request: 240,
                                                                set_hexpand: true,
                                                                set_allow_scroll_wheel: false,
                                                            },

                                                            add_overlay = &gtk::Button {
                                                                set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                                                set_halign: gtk::Align::Start,
                                                                set_valign: gtk::Align::Center,
                                                                set_margin_start: 8,

                                                                gtk::Image {
                                                                    set_icon_name: Some("go-previous-symbolic"),
                                                                    set_pixel_size: 16,
                                                                },
                                                                connect_clicked[screenshot_carousel] => move |_| {
                                                                    let page = screenshot_carousel.position().round() as u32;
                                                                    if page > 0 {
                                                                        let widget = screenshot_carousel.nth_page(page - 1);
                                                                        screenshot_carousel.scroll_to(&widget, true);
                                                                    }
                                                                }
                                                            },

                                                            add_overlay = &gtk::Button {
                                                                set_css_classes: &["circular", "flat", "carousel-nav-btn"],
                                                                set_halign: gtk::Align::End,
                                                                set_valign: gtk::Align::Center,
                                                                set_margin_end: 8,

                                                                gtk::Image {
                                                                    set_icon_name: Some("go-next-symbolic"),
                                                                    set_pixel_size: 16,
                                                                },
                                                                connect_clicked[screenshot_carousel] => move |_| {
                                                                    let page = screenshot_carousel.position().round() as u32;
                                                                    let n_pages = screenshot_carousel.n_pages();
                                                                    if page + 1 < n_pages {
                                                                        let widget = screenshot_carousel.nth_page(page + 1);
                                                                        screenshot_carousel.scroll_to(&widget, true);
                                                                    }
                                                                }
                                                            }
                                                        },

                                                        adw::CarouselIndicatorDots {
                                                            set_carousel: Some(&screenshot_carousel),
                                                            set_halign: gtk::Align::Center,
                                                        }
                                                    },

                                                    // Description
                                                    adw::PreferencesGroup {
                                                        set_title: "Description",

                                                        gtk::Box {
                                                            set_orientation: gtk::Orientation::Vertical,
                                                            set_spacing: 6,
                                                            set_css_classes: &["card"],

                                                            gtk::Label {
                                                                #[watch]
                                                                set_label: &escape(model.selected_project.as_ref().map(|p| p.description.as_str()).unwrap_or("")),
                                                                set_wrap: true,
                                                                set_halign: gtk::Align::Fill,
                                                                set_xalign: 0.0,
                                                                set_use_markup: true,
                                                                set_margin_all: 12,
                                                            },

                                                            gtk::Button {
                                                                set_label: "More Description...",
                                                                set_halign: gtk::Align::Center,
                                                                set_margin_bottom: 16,
                                                                set_css_classes: &["circular", "button-more-description"],
                                                                #[watch]
                                                                set_visible: model.selected_project.as_ref().map(|p| p.body.is_some()).unwrap_or(false),
                                                                connect_clicked => BrowserInput::ShowFullDescription,
                                                            }
                                                        }
                                                    },

                                                    // Links section
                                                    adw::PreferencesGroup {
                                                        set_title: "Links",
                                                        #[watch]
                                                        set_visible: model.selected_project.is_some(),

                                                        adw::ActionRow {
                                                            #[watch]
                                                            set_title: &format!("{} Page", model.source.name()),
                                                            add_prefix = &gtk::Image { set_icon_name: Some("web-browser-symbolic"), set_pixel_size: 16 },
                                                            #[watch]
                                                            set_subtitle: &model.selected_project.as_ref().map(|p| format!("https://modrinth.com/project/{}", p.project_id)).unwrap_or_default(),
                                                            set_subtitle_lines: 1,
                                                            set_activatable: true,
                                                            connect_activated[sender] => move |row| {
                                                                if let Some(subtitle) = row.subtitle() {
                                                                    let url = subtitle.to_string();
                                                                    if !url.is_empty() {
                                                                        sender.input(BrowserInput::OpenUrl(url));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        adw::ActionRow {
                                                            set_title: "Source Code",
                                                            add_prefix = &gtk::Image { set_icon_name: Some("text-editor-symbolic"), set_pixel_size: 16 },
                                                            #[watch]
                                                            set_visible: model.selected_project.as_ref().and_then(|p| p.source_url.as_ref()).is_some(),
                                                            #[watch]
                                                            set_subtitle: &model.selected_project.as_ref().and_then(|p| p.source_url.clone()).unwrap_or_default(),
                                                            set_subtitle_lines: 1,
                                                            set_activatable: true,
                                                            connect_activated[sender] => move |row| {
                                                                if let Some(subtitle) = row.subtitle() {
                                                                    let url = subtitle.to_string();
                                                                    if !url.is_empty() {
                                                                        sender.input(BrowserInput::OpenUrl(url));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        adw::ActionRow {
                                                            set_title: "Wiki / Docs",
                                                            add_prefix = &gtk::Image { set_icon_name: Some("open-book-symbolic"), set_pixel_size: 16 },
                                                            #[watch]
                                                            set_visible: model.selected_project.as_ref().and_then(|p| p.wiki_url.as_ref()).is_some(),
                                                            #[watch]
                                                            set_subtitle: &model.selected_project.as_ref().and_then(|p| p.wiki_url.clone()).unwrap_or_default(),
                                                            set_subtitle_lines: 1,
                                                            set_activatable: true,
                                                            connect_activated[sender] => move |row| {
                                                                if let Some(subtitle) = row.subtitle() {
                                                                    let url = subtitle.to_string();
                                                                    if !url.is_empty() {
                                                                        sender.input(BrowserInput::OpenUrl(url));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        adw::ActionRow {
                                                            set_title: "Discord",
                                                            add_prefix = &gtk::Image { set_icon_name: Some("chat-message-new-symbolic"), set_pixel_size: 16 },
                                                            #[watch]
                                                            set_visible: model.selected_project.as_ref().and_then(|p| p.discord_url.as_ref()).is_some(),
                                                            #[watch]
                                                            set_subtitle: &model.selected_project.as_ref().and_then(|p| p.discord_url.clone()).unwrap_or_default(),
                                                            set_subtitle_lines: 1,
                                                            set_activatable: true,
                                                            connect_activated[sender] => move |row| {
                                                                if let Some(subtitle) = row.subtitle() {
                                                                    let url = subtitle.to_string();
                                                                    if !url.is_empty() {
                                                                        sender.input(BrowserInput::OpenUrl(url));
                                                                    }
                                                                }
                                                            }
                                                        },
                                                    },

                                                    // Versions
                                                    adw::PreferencesGroup {
                                                        set_title: "Available Versions",
                                                        #[local_ref]
                                                        versions_list -> gtk::ListBox {
                                                            set_selection_mode: gtk::SelectionMode::None,
                                                            set_css_classes: &["boxed-list"],
                                                        }
                                                    },

                                                    // License footer (centered)
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &{
                                                            model.selected_project.as_ref()
                                                                .and_then(|p| p.license_name.clone())
                                                                .map(|l| format!("Licensed under {}", l))
                                                                .unwrap_or_default()
                                                        },
                                                        #[watch]
                                                        set_visible: model.selected_project.as_ref().and_then(|p| p.license_name.as_ref()).is_some(),
                                                        set_halign: gtk::Align::Center,
                                                        set_css_classes: &["dim-label", "caption"],
                                                        set_margin_top: 8,
                                                        set_margin_bottom: 16,
                                                    },
                                                },
                                            },
                                        }
                                    },
                                },

                                #[watch]
                                set_visible_child_name: match model.view_state {
                                    BrowserView::Details => "details",
                                    _ => "overview",
                                },
                            }
                        }
                    },
                },

                add_bottom_bar = &gtk::Revealer {
                    #[watch]
                    set_reveal_child: !model.download_queue.is_empty(),
                    set_transition_type: gtk::RevealerTransitionType::SlideUp,

                    gtk::ActionBar {
                        pack_start = &gtk::Button {
                            set_label: "Clear",
                            connect_clicked => BrowserInput::ClearQueue,
                        },

                        #[wrap(Some)]
                        set_center_widget = &gtk::Label {
                            #[watch]
                            set_label: &format!("{} items in queue", model.download_queue.len()),
                            set_css_classes: &["dim-label"],
                        },

                        pack_end = &gtk::Button {
                            set_label: "Install",
                            set_css_classes: &["suggested-action"],
                            connect_clicked => BrowserInput::OpenQueueDialog,
                        },
                    },
                },
            }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let projects = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| match output {
                ProjectRowOutput::AddToQueue(id, title) => BrowserInput::ToggleQueueItem(id, title),
                ProjectRowOutput::OpenDetails(id) => BrowserInput::OpenProjectDetails(id),
            });

        let versions = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| match output {
                VersionRowOutput::Install(pid, vid) => BrowserInput::InstallProject(pid, vid),
                VersionRowOutput::AddToQueue(pid, vid, vname, fname) => {
                    BrowserInput::AddVersionToQueue(pid, vid, vname, fname)
                }
            });

        let screenshot_cards = FactoryVecDeque::builder()
            .launch(adw::Carousel::new())
            .forward(sender.input_sender(), |_| BrowserInput::GoBack); // dummy

        let queue_dialog = QueueDialog::builder().launch(()).forward(
            sender.input_sender(),
            |output| match output {
                QueueDialogOutput::Remove(id) => BrowserInput::RemoveQueueItem(id),
                QueueDialogOutput::Clear => BrowserInput::ClearQueue,
                QueueDialogOutput::Install => BrowserInput::PromptInstallQueue,
                QueueDialogOutput::FetchVersions(id) => BrowserInput::FetchVersions(id),
                QueueDialogOutput::SelectVersion(id, vid, vname, fname) => {
                    BrowserInput::ApplyVersionToQueue(id, vid, vname, fname)
                }
            },
        );

        let description_dialog = DescriptionDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |_| unreachable!());

        let mut model = UnifiedBrowser {
            visible: false,
            game_version: String::new(),
            loader: ModLoader::None,
            editor_type: EditorType::Mods,
            source: Arc::new(ModrinthSource),
            search_query: String::new(),
            loading: false,
            loading_more: false,
            offset: 0,
            total_hits: 0,
            error: None,
            download_queue: HashMap::new(),
            icon_cache: HashMap::new(),
            screenshot_cache: HashMap::new(),
            author_cache: HashMap::new(),
            selected_project: None,
            loading_details: false,
            view_state: BrowserView::Overview,
            projects,
            versions,
            screenshot_cards,
            queue_dialog,
            description_dialog,
            toast_overlay: adw::ToastOverlay::new(),
            collapsed: false,
            show_sidebar: true,
        };

        let projects_list = model.projects.widget();
        let versions_list = model.versions.widget();
        let screenshot_carousel = model.screenshot_cards.widget();
        let widgets = view_output!();

        model.toast_overlay = widgets.toast_overlay.clone();

        let bp_condition = adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            600.0,
            adw::LengthUnit::Sp,
        );
        let bp = adw::Breakpoint::new(bp_condition);
        {
            let split = widgets.split_view.clone();
            bp.connect_apply(move |_| {
                split.set_collapsed(true);
            });
        }
        {
            let split = widgets.split_view.clone();
            bp.connect_unapply(move |_| {
                split.set_collapsed(false);
            });
        }
        root.add_breakpoint(bp);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            BrowserInput::Open { game_version, loader, editor_type } => {
                self.visible = true;
                self.game_version = game_version;
                self.loader = loader;
                self.editor_type = editor_type;
                self.view_state = BrowserView::Overview;
                self.show_sidebar = true;
                self.error = None;
                self.download_queue.clear();
                self.projects.guard().clear();
                self.versions.guard().clear();
                self.screenshot_cards.guard().clear();
                self.search_query.clear();
                self.queue_dialog.emit(QueueDialogInput::ClearCache);

                // Trigger default search
                sender.input(BrowserInput::Search("".to_string()));
            }
            BrowserInput::Close => {
                self.visible = false;
                root.close();
            }
            BrowserInput::GoBack => match self.view_state {
                BrowserView::Details | BrowserView::Results => {
                    self.view_state = BrowserView::Overview;
                    if self.collapsed {
                        self.show_sidebar = true;
                    }
                }
                BrowserView::Overview => {}
            },
            BrowserInput::Refresh => {
                clear_caches();
                if !self.search_query.is_empty() || self.view_state == BrowserView::Results || self.view_state == BrowserView::Details {
                    sender.input(BrowserInput::Search(self.search_query.clone()));
                }
                if self.view_state == BrowserView::Details {
                    if let Some(project) = &self.selected_project {
                        let id = project.project_id.clone();
                        sender.input(BrowserInput::ShowDetails(id));
                    }
                }
            }
            BrowserInput::StartSearch(query) => {
                self.view_state = BrowserView::Results;
                sender.input(BrowserInput::Search(query));
            }
            BrowserInput::Search(query) => {
                self.search_query = query.clone();
                self.loading = true;
                self.loading_more = false;
                self.offset = 0;
                self.total_hits = 0;
                self.error = None;

                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let et = self.editor_type.clone();
                let s_clone = sender.input_sender().clone();
                let source = self.source.clone();

                std::thread::spawn(move || {
                    let result = source.search(&query, 20, 0, &gv, l, &et);
                    s_clone.send(BrowserInput::SearchDone(result)).ok();
                });
            }
            BrowserInput::SearchDone(result) => {
                self.loading = false;
                self.loading_more = false;
                match result {
                    Ok(search_result) => {
                        self.offset = search_result.offset;
                        self.total_hits = search_result.total_hits;
                        let mut guard = self.projects.guard();
                        guard.clear();
                        for hit in search_result.hits {
                            let in_queue = self.download_queue.contains_key(&hit.project_id);
                            let icon = hit
                                .icon_url
                                .as_ref()
                                .and_then(|url| self.icon_cache.get(url).cloned());

                            if icon.is_none() {
                                if let Some(url) = &hit.icon_url {
                                    fetch_icon(url.clone(), sender.input_sender().clone());
                                }
                            }

                            if let Some(ref author) = hit.author {
                                self.author_cache
                                    .insert(hit.project_id.clone(), author.clone());
                            }

                            guard.push_back((hit, in_queue, icon));
                        }
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
            }
            BrowserInput::LoadMore => {
                if self.loading
                    || self.loading_more
                    || (self.projects.len() as u32) >= self.total_hits
                {
                    return;
                }
                self.loading_more = true;
                let query = self.search_query.clone();
                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let et = self.editor_type.clone();
                let s_clone = sender.input_sender().clone();
                let next_offset = self.offset + 20;
                let source = self.source.clone();

                std::thread::spawn(move || {
                    let result = source.search(&query, 20, next_offset, &gv, l, &et);
                    s_clone.send(BrowserInput::LoadMoreDone(result)).ok();
                });
            }
            BrowserInput::LoadMoreDone(result) => {
                self.loading_more = false;
                match result {
                    Ok(search_result) => {
                        self.offset = search_result.offset;
                        self.total_hits = search_result.total_hits;
                        let mut guard = self.projects.guard();
                        for hit in search_result.hits {
                            let in_queue = self.download_queue.contains_key(&hit.project_id);
                            let icon = hit
                                .icon_url
                                .as_ref()
                                .and_then(|url| self.icon_cache.get(url).cloned());

                            if icon.is_none() {
                                if let Some(url) = &hit.icon_url {
                                    fetch_icon(url.clone(), sender.input_sender().clone());
                                }
                            }

                            if let Some(ref author) = hit.author {
                                self.author_cache
                                    .insert(hit.project_id.clone(), author.clone());
                            }

                            guard.push_back((hit, in_queue, icon));
                        }
                    }
                    Err(e) => {
                        sender.input(BrowserInput::ShowToast(format!(
                            "Failed to load more: {}",
                            e
                        )));
                    }
                }
            }
            BrowserInput::ToggleQueueItem(id, title) => {
                if self.download_queue.contains_key(&id) {
                    self.download_queue.remove(&id);
                } else {
                    self.download_queue.insert(
                        id.clone(),
                        QueueItem {
                            title,
                            version_id: None,
                            version_name: None,
                            filename: None,
                        },
                    );
                }

                let mut guard = self.projects.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        if row.project.project_id == id {
                            row.in_queue = self.download_queue.contains_key(&id);
                            break;
                        }
                    }
                }
                drop(guard);
            }
            BrowserInput::RemoveQueueItem(id) => {
                self.download_queue.remove(&id);

                let mut guard = self.projects.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        if row.project.project_id == id {
                            row.in_queue = false;
                            break;
                        }
                    }
                }
                drop(guard);
            }
            BrowserInput::ToggleQueueFromDetails => {
                if let Some(p) = &self.selected_project {
                    let id = p.project_id.clone();
                    let title = p.title.clone();
                    sender.input(BrowserInput::ToggleQueueItem(id, title));
                }
            }
            BrowserInput::AddVersionToQueue(pid, vid, vname, fname) => {
                let title = self
                    .selected_project
                    .as_ref()
                    .map(|p| p.title.clone())
                    .unwrap_or_else(|| "Unknown".to_string());
                self.download_queue.insert(
                    pid.clone(),
                    QueueItem {
                        title,
                        version_id: Some(vid),
                        version_name: Some(vname),
                        filename: Some(fname),
                    },
                );

                // Update ProjectRow if visible
                let mut guard = self.projects.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        if row.project.project_id == pid {
                            row.in_queue = true;
                            break;
                        }
                    }
                }
            }
            BrowserInput::OpenQueueDialog => {
                self.queue_dialog
                    .emit(QueueDialogInput::Open(self.download_queue.clone()));
                self.queue_dialog.widget().present(Some(root));
            }
            BrowserInput::OpenProjectDetails(id) => {
                sender.input(BrowserInput::ShowDetails(id));
            }
            BrowserInput::ShowDetails(id) => {
                self.view_state = BrowserView::Details;
                if self.collapsed {
                    self.show_sidebar = false;
                }
                self.loading_details = true;
                self.selected_project = None;
                self.versions.guard().clear();
                self.screenshot_cards.guard().clear();

                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let id_clone = id.clone();
                let source = self.source.clone();

                std::thread::spawn(move || {
                    let result = (|| {
                        let project = source.get_project(&id_clone)?;
                        let versions = source.get_project_versions(&id_clone, &gv, l)?;
                        Ok((project, versions))
                    })();
                    sender.input(BrowserInput::DetailsLoaded(result));
                });
            }
            BrowserInput::IconLoaded(url, texture) => {
                self.icon_cache.insert(url.clone(), texture.clone());

                let mut guard = self.projects.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        if row.project.icon_url.as_ref() == Some(&url) {
                            row.icon = Some(texture.clone());
                        }
                    }
                }
            }
            BrowserInput::ScreenshotLoaded(url, texture) => {
                self.screenshot_cache.insert(url.clone(), texture.clone());

                if let Some(ref project) = self.selected_project {
                    if project.screenshots.contains(&url) {
                        self.screenshot_cards.guard().push_back(texture);
                    }
                }
            }
            BrowserInput::ShowList => {
                self.view_state = BrowserView::Results;
            }
            BrowserInput::DetailsLoaded(result) => {
                self.loading_details = false;
                match result {
                    Ok((mut project, versions)) => {
                        if project.author.is_none() {
                            if let Some(author) =
                                self.author_cache.get(&project.project_id).cloned()
                            {
                                project.author = Some(author);
                            }
                        }

                        if let Some(url) = &project.icon_url {
                            if !self.icon_cache.contains_key(url) {
                                fetch_icon(url.clone(), sender.input_sender().clone());
                            }
                        }

                        // Load screenshots
                        let mut scr_guard = self.screenshot_cards.guard();
                        scr_guard.clear();
                        for url in &project.screenshots {
                            if let Some(tex) = self.screenshot_cache.get(url) {
                                scr_guard.push_back(tex.clone());
                            } else {
                                let url_clone = url.clone();
                                let s_input = sender.input_sender().clone();
                                thread::spawn(move || {
                                    use crate::backend::instance::modpack::HTTP_CLIENT;
                                    if let Ok(res) = HTTP_CLIENT.get(&url_clone).send() {
                                        if res.status().is_success() {
                                            if let Ok(bytes) = res.bytes() {
                                                if let Ok(img) = image::load_from_memory(&bytes) {
                                                    let width = img.width() as i32;
                                                    let height = img.height() as i32;
                                                    let gbytes = glib::Bytes::from(&img.to_rgba8().into_raw());
                                                    let texture = gdk::MemoryTexture::new(
                                                        width,
                                                        height,
                                                        gdk::MemoryFormat::R8g8b8a8,
                                                        &gbytes,
                                                        (width * 4) as usize,
                                                    );
                                                    let texture: gdk::Texture = texture.upcast();
                                                    s_input.send(BrowserInput::ScreenshotLoaded(url_clone, texture)).ok();
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                        }

                        let pid = project.project_id.clone();
                        self.selected_project = Some(project);
                        let mut guard = self.versions.guard();
                        guard.clear();
                        for v in versions {
                            guard.push_back((v, pid.clone()));
                        }
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
            }
            BrowserInput::ClearQueue => {
                self.download_queue.clear();
                let mut guard = self.projects.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        row.in_queue = false;
                    }
                }
                drop(guard);
            }
            BrowserInput::PromptInstallQueue => {
                let mut titles: Vec<_> = self
                    .download_queue
                    .values()
                    .map(|q| {
                        if let Some(ref vname) = q.version_name {
                            format!("{} ({})", q.title, vname)
                        } else {
                            q.title.clone()
                        }
                    })
                    .collect();
                titles.sort();
                let body = format!(
                    "You are about to install {} item{}:\n\n{}",
                    titles.len(),
                    if titles.len() == 1 { "" } else { "s" },
                    titles.join("\n")
                );

                let dialog = adw::AlertDialog::builder()
                    .heading("Confirm Installation")
                    .body(&body)
                    .build();
                dialog.add_response("cancel", "Cancel");
                dialog.add_response("install", "Install");
                dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);

                dialog.present(Some(self.queue_dialog.widget()));

                let s_clone = sender.input_sender().clone();
                dialog.connect_response(None, move |_d, response| {
                    if response == "install" {
                        s_clone.send(BrowserInput::ConfirmInstallQueue).ok();
                    }
                });
            }
            BrowserInput::ConfirmInstallQueue => {
                let mut installs = Vec::new();
                for (id, item) in &self.download_queue {
                    installs.push((id.clone(), item.version_id.clone().unwrap_or_default()));
                }
                sender.output(BrowserOutput::InstallItems {
                    editor_type: self.editor_type.clone(),
                    installs,
                }).ok();
                self.visible = false;
                self.queue_dialog.emit(QueueDialogInput::Close);
                sender.input(BrowserInput::ClearQueue);
                root.close();
            }
            BrowserInput::InstallProject(pid, vid) => {
                sender
                    .output(BrowserOutput::InstallItems {
                        editor_type: self.editor_type.clone(),
                        installs: vec![(pid, vid)],
                    })
                    .ok();
                self.visible = false;
                root.close();
            }
            BrowserInput::FetchVersions(id) => {
                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let s_clone = sender.input_sender().clone();
                let id_clone = id.clone();
                let source = self.source.clone();

                std::thread::spawn(move || {
                    let result = source.get_project_versions(&id_clone, &gv, l);
                    s_clone
                        .send(BrowserInput::VersionsFetched(id_clone, result))
                        .ok();
                });
            }
            BrowserInput::VersionsFetched(id, result) => {
                if let Ok(versions) = result {
                    self.queue_dialog
                        .emit(QueueDialogInput::SetVersions(id, versions));
                }
            }
            BrowserInput::ApplyVersionToQueue(id, vid, vname, fname) => {
                if let Some(item) = self.download_queue.get_mut(&id) {
                    if vid.is_empty() {
                        item.version_id = None;
                        item.version_name = None;
                        item.filename = None;
                    } else {
                        item.version_id = Some(vid);
                        item.version_name = Some(vname);
                        item.filename = Some(fname);
                    }
                }
            }
            BrowserInput::ShowToast(msg) => {
                self.toast_overlay.add_toast(adw::Toast::new(&msg));
            }
            BrowserInput::SetCollapsed(collapsed) => {
                self.collapsed = collapsed;
                if collapsed {
                    if self.view_state == BrowserView::Details {
                        self.show_sidebar = false;
                    } else {
                        self.show_sidebar = true;
                    }
                } else {
                    self.show_sidebar = true;
                }
            }
            BrowserInput::ToggleSidebar => {
                if self.collapsed {
                    self.show_sidebar = true;
                } else {
                    self.show_sidebar = !self.show_sidebar;
                }
            }
            BrowserInput::OpenUrl(url) => {
                crate::frontend::utils::open_url(&url);
            }
            BrowserInput::ScreenshotScroll(_pos) => {}
            BrowserInput::ShowFullDescription => {
                if let Some(ref project) = self.selected_project {
                    let body = project.body.clone().unwrap_or_else(|| project.description.clone());
                    self.description_dialog.emit(DescriptionDialogInput::Show {
                        title: project.title.clone(),
                        body,
                    });
                    let parent = relm4::main_application().active_window();
                    self.description_dialog.widget().present(parent.as_ref());
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Queue Dialog component
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum QueueDialogInput {
    Open(HashMap<String, QueueItem>),
    Search(String),
    Remove(String),
    Clear,
    ClearCache,
    Install,
    Close,
    SetVersions(String, Vec<BrowserVersion>),
    SelectVersion(String, String, String, String),
    FetchVersions(String),
}

#[derive(Debug)]
pub enum QueueDialogOutput {
    Remove(String),
    Clear,
    Install,
    FetchVersions(String),
    SelectVersion(String, String, String, String),
}

pub struct QueueDialog {
    visible: bool,
    search_query: String,
    all_items: HashMap<String, QueueItem>,
    version_cache: HashMap<String, Vec<BrowserVersion>>,
    queue_rows: FactoryVecDeque<QueueRow>,
}

#[relm4::component(pub)]
impl Component for QueueDialog {
    type Init = ();
    type Input = QueueDialogInput;
    type Output = QueueDialogOutput;
    type CommandOutput = ();

    view! {
        adw::Dialog {
            set_title: "Download Queue",
            set_content_width: 500,
            set_content_height: 480,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Download Queue",
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 8,

                    gtk::SearchEntry {
                        set_margin_start: 16,
                        set_margin_end: 16,
                        set_margin_top: 12,
                        set_margin_bottom: 4,
                        set_placeholder_text: Some("Search queue..."),
                        connect_search_changed[sender] => move |entry| {
                            sender.input(QueueDialogInput::Search(entry.text().to_string()));
                        }
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_margin_all: 16,
                            set_spacing: 6,

                            #[local_ref]
                            queue_list -> gtk::ListBox {
                                set_selection_mode: gtk::SelectionMode::None,
                                set_css_classes: &["boxed-list"],
                            }
                        }
                    }
                },

                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "Install",
                        set_css_classes: &["pill", "suggested-action"],
                        connect_clicked => QueueDialogInput::Install,
                        #[watch]
                        set_sensitive: !model.all_items.is_empty(),
                    },

                    gtk::Button {
                        set_label: "Cancel",
                        set_css_classes: &["pill"],
                        connect_clicked => QueueDialogInput::Close,
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            visible: false,
            search_query: String::new(),
            all_items: HashMap::new(),
            version_cache: HashMap::new(),
            queue_rows: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .forward(sender.input_sender(), |output| match output {
                    QueueRowOutput::Remove(id) => QueueDialogInput::Remove(id),
                    QueueRowOutput::FetchVersions(id) => QueueDialogInput::FetchVersions(id),
                    QueueRowOutput::SelectVersion(id, vid, vname, fname) => {
                        QueueDialogInput::SelectVersion(id, vid, vname, fname)
                    }
                }),
        };

        let queue_list = model.queue_rows.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            QueueDialogInput::Open(items) => {
                self.all_items = items;
                self.visible = true;
                self.rebuild_list();

                // Proactively fetch versions only for missing items
                for id in self.all_items.keys() {
                    if !self.version_cache.contains_key(id) {
                        sender
                            .output(QueueDialogOutput::FetchVersions(id.clone()))
                            .ok();
                    }
                }
            }
            QueueDialogInput::Search(query) => {
                self.search_query = query;
                self.rebuild_list();
            }
            QueueDialogInput::Remove(id) => {
                self.all_items.remove(&id);
                sender.output(QueueDialogOutput::Remove(id)).ok();
                self.rebuild_list();
            }
            QueueDialogInput::Clear => {
                self.all_items.clear();
                sender.output(QueueDialogOutput::Clear).ok();
                self.rebuild_list();
            }
            QueueDialogInput::ClearCache => {
                self.version_cache.clear();
            }
            QueueDialogInput::Install => {
                sender.output(QueueDialogOutput::Install).ok();
                root.close();
            }
            QueueDialogInput::Close => {
                self.visible = false;
                root.close();
            }
            QueueDialogInput::FetchVersions(id) => {
                sender.output(QueueDialogOutput::FetchVersions(id)).ok();
            }
            QueueDialogInput::SelectVersion(id, vid, vname, fname) => {
                if let Some(item) = self.all_items.get_mut(&id) {
                    if vid.is_empty() {
                        item.version_id = None;
                        item.version_name = None;
                        item.filename = None;
                    } else {
                        item.version_id = Some(vid.clone());
                        item.version_name = Some(vname.clone());
                        item.filename = Some(fname.clone());
                    }

                    // Update the row in-place in the factory
                    let mut guard = self.queue_rows.guard();
                    for i in 0..guard.len() {
                        if guard.get(i).map(|r| r.project_id.as_str()) == Some(&id) {
                            if let Some(row) = guard.get_mut(i) {
                                row.item = item.clone();
                            }
                            break;
                        }
                    }
                }
                sender
                    .output(QueueDialogOutput::SelectVersion(id, vid, vname, fname))
                    .ok();
            }
            QueueDialogInput::SetVersions(id, versions) => {
                self.version_cache.insert(id.clone(), versions.clone());
                let mut guard = self.queue_rows.guard();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get_mut(i) {
                        if row.project_id == id {
                            guard.send(i, QueueRowInput::SetVersions(versions.clone()));
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl QueueDialog {
    fn rebuild_list(&mut self) {
        let mut guard = self.queue_rows.guard();
        guard.clear();
        let mut items: Vec<_> = self.all_items.iter().collect();
        items.sort_by(|a, b| a.1.title.cmp(&b.1.title));

        let query = self.search_query.to_lowercase();
        for (id, item) in items {
            if query.is_empty() || item.title.to_lowercase().contains(&query) {
                let cached = self.version_cache.get(id).cloned().unwrap_or_default();
                guard.push_back((id.clone(), item.clone(), cached));
            }
        }
    }
}

fn fetch_icon(url: String, sender: relm4::Sender<BrowserInput>) {
    thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .user_agent("obelisk-launcher-rs (github.com/magnotec/obelisk-launcher)")
            .build()
            .unwrap();
        match client.get(&url).send() {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("[browser-icon] HTTP {} for {}", response.status(), url);
                    return;
                }
                match response.bytes() {
                    Ok(bytes) => {
                        let gbytes = gtk::glib::Bytes::from(&bytes);
                        match gdk::Texture::from_bytes(&gbytes) {
                            Ok(tex) => {
                                sender.send(BrowserInput::IconLoaded(url, tex)).ok();
                            }
                            Err(e) => {
                                eprintln!(
                                    "[browser-icon] Texture decode error for {}: {}",
                                    url, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[browser-icon] Body read error for {}: {}", url, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[browser-icon] Request error for {}: {}", url, e);
            }
        }
    });
}

struct TagBalancer {
    stack: Vec<String>,
}

impl TagBalancer {
    fn new() -> Self {
        TagBalancer { stack: Vec::new() }
    }

    fn open(&mut self, tag: &str) -> String {
        self.stack.push(tag.to_string());
        format!("<{}>", tag)
    }

    fn close(&mut self, tag_name: &str) -> String {
        let tag_name_lower = tag_name.to_lowercase();
        if let Some(pos) = self.stack.iter().rposition(|t| {
            let name = t.split_whitespace().next().unwrap_or(t);
            name.to_lowercase() == tag_name_lower
        }) {
            let mut result = String::new();
            let mut to_reopen = Vec::new();
            while self.stack.len() > pos {
                let tag = self.stack.pop().unwrap();
                let name = tag.split_whitespace().next().unwrap_or(&tag);
                result.push_str(&format!("</{}>", name));
                if name.to_lowercase() != tag_name_lower {
                    to_reopen.push(tag);
                }
            }
            for tag in to_reopen.into_iter().rev() {
                result.push_str(&self.open(&tag));
            }
            result
        } else {
            String::new()
        }
    }

    fn close_all(&mut self) -> String {
        let mut result = String::new();
        while let Some(tag) = self.stack.pop() {
            let name = tag.split_whitespace().next().unwrap_or(&tag);
            result.push_str(&format!("</{}>", name));
        }
        result
    }
}

enum TagAction {
    Open(String),
    Close(String),
    Text(String),
    None,
}

fn process_html_tag_action(tag: &str) -> TagAction {
    let lower = tag.to_lowercase();
    let trimmed = lower.trim_start_matches('<').trim_end_matches('>').trim();

    if trimmed.starts_with("br") {
        return TagAction::Text("\n".to_string());
    }
    if trimmed.starts_with("p") && !trimmed.starts_with("/p") {
        return TagAction::Text("\n".to_string());
    }
    if trimmed.starts_with("/p") {
        return TagAction::Text("\n".to_string());
    }
    if trimmed.starts_with("strong") || trimmed.starts_with("b") {
        if trimmed.starts_with("/") {
            return TagAction::Close("b".to_string());
        } else {
            return TagAction::Open("b".to_string());
        }
    }
    if trimmed.starts_with("em") || trimmed.starts_with("i") {
        if trimmed.starts_with("/") {
            return TagAction::Close("i".to_string());
        } else {
            return TagAction::Open("i".to_string());
        }
    }
    if trimmed.starts_with("code") {
        if trimmed.starts_with("/") {
            return TagAction::Close("span".to_string());
        } else {
            return TagAction::Open("span font_family=\"monospace\" background=\"#282c34\" color=\"#e06c75\"".to_string());
        }
    }
    if trimmed.starts_with("li") {
        return TagAction::Text("  • ".to_string());
    }
    if trimmed.starts_with("/li") {
        return TagAction::Text("\n".to_string());
    }

    if trimmed.starts_with("a ") {
        if let Some(href_idx) = lower.find("href=") {
            let rest = &tag[href_idx + 5..];
            let quote_char = rest.chars().next().unwrap_or('"');
            if quote_char == '"' || quote_char == '\'' {
                let rest_after_quote = &rest[1..];
                if let Some(end_quote_idx) = rest_after_quote.find(quote_char) {
                    let url = &rest_after_quote[..end_quote_idx];
                    let escaped_url = glib::markup_escape_text(url).to_string();
                    return TagAction::Open(format!("a href=\"{}\"", escaped_url));
                }
            }
        }
        return TagAction::Open("a".to_string());
    }
    if trimmed.starts_with("/a") {
        return TagAction::Close("a".to_string());
    }

    TagAction::None
}

fn markdown_to_pango(md: &str) -> String {
    let mut balancer = TagBalancer::new();
    let mut block_parsed = String::new();
    let mut in_code_block = false;
    let mut code_block_content = String::new();

    for line in md.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                let escaped_code = glib::markup_escape_text(&code_block_content).to_string();
                block_parsed.push_str(&format!(
                    "<span font_family=\"monospace\" background=\"#282c34\" color=\"#abb2bf\">\n{}\n</span>\n",
                    escaped_code
                ));
                code_block_content.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_content.push_str(line);
            code_block_content.push('\n');
            continue;
        }

        // Headers
        if trimmed.starts_with("# ") {
            let content = trimmed[2..].trim();
            block_parsed.push_str(&format!(
                "\n<span size=\"xx-large\" weight=\"bold\">{}</span>\n\n",
                content
            ));
        } else if trimmed.starts_with("## ") {
            let content = trimmed[3..].trim();
            block_parsed.push_str(&format!(
                "\n<span size=\"x-large\" weight=\"bold\">{}</span>\n\n",
                content
            ));
        } else if trimmed.starts_with("### ") {
            let content = trimmed[4..].trim();
            block_parsed.push_str(&format!(
                "\n<span size=\"large\" weight=\"bold\">{}</span>\n\n",
                content
            ));
        } else if trimmed.starts_with("#### ") {
            let content = trimmed[5..].trim();
            block_parsed.push_str(&format!(
                "\n<span weight=\"bold\">{}</span>\n\n",
                content
            ));
        } else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            block_parsed.push_str("\n<span color=\"#555555\">────────────────────────────────────────────────</span>\n\n");
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
            let content = trimmed[2..].trim();
            block_parsed.push_str(&format!("  • {}\n", content));
        } else if trimmed.is_empty() {
            block_parsed.push('\n');
        } else {
            block_parsed.push_str(line);
            block_parsed.push('\n');
        }
    }

    if in_code_block && !code_block_content.is_empty() {
        let escaped_code = glib::markup_escape_text(&code_block_content).to_string();
        block_parsed.push_str(&format!(
            "<span font_family=\"monospace\" background=\"#282c34\" color=\"#abb2bf\">\n{}\n</span>\n",
            escaped_code
        ));
    }

    let mut parsed = parse_html_and_inline(&block_parsed, &mut balancer);
    parsed.push_str(&balancer.close_all());
    parsed
}

fn parse_html_and_inline(text: &str, balancer: &mut TagBalancer) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '<' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '>' {
                j += 1;
            }
            if j < chars.len() {
                let tag: String = chars[i..=j].iter().collect();
                match process_html_tag_action(&tag) {
                    TagAction::Open(t) => result.push_str(&balancer.open(&t)),
                    TagAction::Close(t) => result.push_str(&balancer.close(&t)),
                    TagAction::Text(txt) => result.push_str(&txt),
                    TagAction::None => {}
                }
                i = j + 1;
                continue;
            }
        }

        let mut j = i + 1;
        while j < chars.len() && chars[j] != '<' {
            j += 1;
        }
        let segment: String = chars[i..j].iter().collect();
        result.push_str(&parse_inline(&segment, balancer));
        i = j;
    }

    result
}

fn parse_inline(text: &str, balancer: &mut TagBalancer) -> String {
    let escaped = glib::markup_escape_text(text).to_string();
    let chars: Vec<char> = escaped.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    let mut in_bold = false;
    let mut in_italic = false;
    let mut in_code = false;

    while i < chars.len() {
        if chars[i] == '`' {
            if in_code {
                result.push_str(&balancer.close("span"));
            } else {
                result.push_str(&balancer.open("span font_family=\"monospace\" background=\"#282c34\" color=\"#e06c75\""));
            }
            in_code = !in_code;
            i += 1;
            continue;
        }

        if in_code {
            result.push(chars[i]);
            i += 1;
            continue;
        }

        if chars[i] == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if in_bold {
                result.push_str(&balancer.close("b"));
            } else {
                result.push_str(&balancer.open("b"));
            }
            in_bold = !in_bold;
            i += 2;
            continue;
        }

        if chars[i] == '*' {
            if in_italic {
                result.push_str(&balancer.close("i"));
            } else {
                result.push_str(&balancer.open("i"));
            }
            in_italic = !in_italic;
            i += 1;
            continue;
        }

        if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' {
            let mut j = i + 2;
            let mut bracket_count = 1;
            while j < chars.len() {
                if chars[j] == '[' {
                    bracket_count += 1;
                } else if chars[j] == ']' {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if j < chars.len() && j + 1 < chars.len() && chars[j + 1] == '(' {
                let mut k = j + 2;
                while k < chars.len() && chars[k] != ')' {
                    k += 1;
                }
                if k < chars.len() {
                    let alt: String = chars[i + 2..j].iter().collect();
                    let url: String = chars[j + 2..k].iter().collect();
                    let alt_text = if alt.is_empty() { "Image" } else { &alt };

                    result.push_str(&balancer.open(&format!("a href=\"{}\"", url)));
                    result.push_str(&format!("🖼️ {}", alt_text));
                    result.push_str(&balancer.close("a"));

                    i = k + 1;
                    continue;
                }
            }
        }

        if chars[i] == '[' {
            let mut j = i + 1;
            let mut bracket_count = 1;
            while j < chars.len() {
                if chars[j] == '[' {
                    bracket_count += 1;
                } else if chars[j] == ']' {
                    bracket_count -= 1;
                    if bracket_count == 0 {
                        break;
                    }
                }
                j += 1;
            }
            if j < chars.len() && j + 1 < chars.len() && chars[j + 1] == '(' {
                let mut k = j + 2;
                while k < chars.len() && chars[k] != ')' {
                    k += 1;
                }
                if k < chars.len() {
                    let text: String = chars[i + 1..j].iter().collect();
                    let url: String = chars[j + 2..k].iter().collect();

                    result.push_str(&balancer.open(&format!("a href=\"{}\"", url)));
                    result.push_str(&parse_inline(&text, balancer));
                    result.push_str(&balancer.close("a"));

                    i = k + 1;
                    continue;
                }
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    if in_bold {
        result.push_str(&balancer.close("b"));
    }
    if in_italic {
        result.push_str(&balancer.close("i"));
    }
    if in_code {
        result.push_str(&balancer.close("span"));
    }

    result
}

// ---------------------------------------------------------------------------
// Description Dialog component
// ---------------------------------------------------------------------------

pub struct DescriptionDialog {
    title: String,
    body: String,
}

#[derive(Debug)]
pub enum DescriptionDialogInput {
    Show { title: String, body: String },
    Close,
}

#[relm4::component(pub)]
impl Component for DescriptionDialog {
    type Init = ();
    type Input = DescriptionDialogInput;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Dialog {
            #[watch]
            set_title: &model.title,
            set_content_width: 700,
            set_content_height: 550,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        #[watch]
                        set_title: &model.title,
                        set_subtitle: "Detailed Description",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vscrollbar_policy: gtk::PolicyType::Automatic,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 20,

                        gtk::Label {
                            #[watch]
                            set_label: &markdown_to_pango(&model.body),
                            set_wrap: true,
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Start,
                            set_xalign: 0.0,
                            set_yalign: 0.0,
                            set_selectable: true,
                            set_use_markup: true,
                            set_can_focus: false,
                        }
                    }
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, _sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DescriptionDialog {
            title: String::new(),
            body: String::new(),
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DescriptionDialogInput::Show { title, body } => {
                self.title = title;
                self.body = body;
            }
            DescriptionDialogInput::Close => {
                root.close();
            }
        }
    }
}
