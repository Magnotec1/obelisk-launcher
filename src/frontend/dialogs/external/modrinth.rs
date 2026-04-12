#![allow(unused_assignments)]

use crate::backend::download::modrinth::{
    clear_caches, get_project_versions, search_mods, ModrinthProject, ModrinthSearchResult,
    ModrinthVersion,
};
use crate::backend::instance::manager::ModLoader;
use adw::prelude::*;
use gtk::gdk;
use gtk::glib;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::collections::HashMap;
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
    pub project: ModrinthProject,
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
    type Init = (ModrinthProject, bool, Option<gdk::Texture>);
    type Input = ProjectRowInput;
    type Output = ProjectRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            #[watch]
            set_title: &escape(&self.project.title),
            #[watch]
            set_subtitle: &{
                let author = self.project.author.as_deref().unwrap_or("Unknown");
                let dls = format_downloads(self.project.downloads);
                escape(&format!("by {} \u{b7} {} downloads", author, dls))
            },
            set_activatable: true,
            set_use_markup: true,

            add_prefix = &gtk::Stack {
                set_hhomogeneous: true,
                set_vhomogeneous: true,
                #[watch]
                set_visible_child_name: if self.icon.is_some() { "icon" } else { "fallback" },

                add_named[Some("icon")] = &gtk::Image {
                    #[watch]
                    set_paintable: self.icon.as_ref().map(|t| t as &gdk::Texture),
                    set_pixel_size: 40,
                },

                add_named[Some("fallback")] = &gtk::Image {
                    set_icon_name: Some("package-x-generic-symbolic"),
                    set_pixel_size: 40,
                },
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
    pub version: ModrinthVersion,
    pub project_id: String,
}

#[derive(Debug)]
pub enum VersionRowOutput {
    Install(String, String),
    AddToQueue(String, String, String, String), // pid, vid, vname, filename
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (ModrinthVersion, String);
    type Input = ();
    type Output = VersionRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &escape(&self.version.name),
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

// ---------------------------------------------------------------------------
// Queue Row factory (for the bottom sheet)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Queue Row
// ---------------------------------------------------------------------------

pub struct QueueRow {
    pub project_id: String,
    pub item: QueueItem,
    pub loading: bool,
    pub fetched: bool,
    pub versions: Vec<ModrinthVersion>,
    // Widget refs for imperative updates
    version_label: gtk::Label,
    version_list: gtk::ListBox,
    popover: gtk::Popover,
}

#[derive(Debug)]
pub enum QueueRowInput {
    SetVersions(Vec<ModrinthVersion>),
    Select(u32),
    OpenPopover,
    FilterVersions(String),
    Reset,
}

#[derive(Debug)]
pub enum QueueRowOutput {
    Remove(String),
    SelectVersion(String, String, String, String), // id, vid, vname, fname
    FetchVersions(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for QueueRow {
    type Init = (String, QueueItem, Vec<ModrinthVersion>);
    type Input = QueueRowInput;
    type Output = QueueRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::ListBoxRow {
            set_activatable: true,
            set_selectable: false,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_margin_all: 12,
                set_spacing: 12,

                #[name = "info_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_valign: gtk::Align::Center,
                    set_spacing: 1,

                    gtk::Label {
                        set_label: &escape(&self.item.title),
                        set_halign: gtk::Align::Start,
                        set_css_classes: &["heading"],
                        set_use_markup: true,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 4,
                        set_halign: gtk::Align::Start,
                        #[name = "version_anchor"]
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 4,

                            #[name = "version_lbl"]
                            gtk::Label {
                                set_markup: "<span size='small'>Latest Compatible</span>",
                                set_css_classes: &["dim-label"],
                                set_halign: gtk::Align::Start,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_use_markup: true,
                            },

                            gtk::Image {
                                set_icon_name: Some("pan-down-symbolic"),
                                set_css_classes: &["dim-label"],
                                set_pixel_size: 10,
                                set_valign: gtk::Align::Center,
                            },
                        },
                    },
                },

                gtk::Button {
                    set_icon_name: "edit-undo-symbolic",
                    set_css_classes: &["flat", "circular"],
                    set_valign: gtk::Align::Center,
                    set_tooltip_text: Some("Reset to latest compatible"),
                    #[watch]
                    set_visible: self.item.version_id.is_some(),
                    connect_clicked[sender] => move |_| {
                        sender.input(QueueRowInput::Reset);
                    }
                },

                gtk::Button {
                    set_icon_name: "list-remove-symbolic",
                    set_css_classes: &["flat", "circular"],
                    set_valign: gtk::Align::Center,
                    set_tooltip_text: Some("Remove from queue"),
                    connect_clicked[sender, pid = self.project_id.clone()] => move |_| {
                        sender.output(QueueRowOutput::Remove(pid.clone())).ok();
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let version_list = gtk::ListBox::new();
        version_list.set_selection_mode(gtk::SelectionMode::None);
        version_list.set_css_classes(&["boxed-list"]);

        // Function to create a version row
        let create_row = |title: &str| {
            adw::ActionRow::builder()
                .title(title)
                .activatable(true)
                .build()
        };

        // Create version rows
        for v in &init.2 {
            version_list.append(&create_row(&format!("{} · {}", v.name, v.version_number)));
        }

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_placeholder_text(Some("Search versions..."));
        search_entry.set_margin_all(6);

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .width_request(320)
            .max_content_height(400)
            .propagate_natural_height(true)
            .child(&version_list)
            .build();

        let popover_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        popover_content.append(&search_entry);
        popover_content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        popover_content.append(&scroll);

        let popover = gtk::Popover::new();
        popover.set_child(Some(&popover_content));

        // Connect search entry
        let sender_clone = sender.clone();
        search_entry.connect_search_changed(move |entry| {
            sender_clone.input(QueueRowInput::FilterVersions(entry.text().to_string()));
        });

        // Connect row activation
        let sender_clone = sender.clone();
        version_list.connect_row_activated(move |_, row| {
            let idx = row.index() as u32;
            sender_clone.input(QueueRowInput::Select(idx));
        });

        let version_label = gtk::Label::new(None);
        version_label.set_use_markup(true);
        version_label.set_markup("<span size='small'>Latest Compatible</span>");

        // Set initial label if we have a selected version
        if let Some(vid) = &init.1.version_id {
            if let Some(v) = init.2.iter().find(|v| v.id == *vid) {
                version_label.set_markup(&format!("<span size='small'>{} · {}</span>", glib::markup_escape_text(&v.name), glib::markup_escape_text(&v.version_number)));
            }
        }

        Self {
            project_id: init.0,
            item: init.1,
            loading: false,
            fetched: !init.2.is_empty(),
            versions: init.2,
            version_label,
            version_list,
            popover,
        }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as relm4::factory::FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        self.version_label = widgets.version_lbl.clone();
        if let Some(vid) = &self.item.version_id {
            if let Some(v) = self.versions.iter().find(|v| v.id == *vid) {
                self.version_label.set_markup(&format!("<span size='small'>{} · {}</span>", glib::markup_escape_text(&v.name), glib::markup_escape_text(&v.version_number)));
            }
        }

        self.popover.set_parent(&widgets.version_anchor);
        self.popover.set_has_arrow(true);
        self.popover.set_autohide(true);
        self.popover.set_position(gtk::PositionType::Bottom);

        // Open on click only on the info part
        let gesture = gtk::GestureClick::new();
        let sender_clone = sender.clone();
        gesture.connect_pressed(move |_, _, _, _| {
            sender_clone.input(QueueRowInput::OpenPopover);
        });
        widgets.info_box.add_controller(gesture);

        // Also handle keyboard activation
        let sender_clone2 = sender.clone();
        root.connect_activate(move |_| {
            sender_clone2.input(QueueRowInput::OpenPopover);
        });

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            QueueRowInput::SetVersions(versions) => {
                self.loading = false;
                self.fetched = true;
                self.versions = versions;
                
                // Clear existing rows
                while let Some(child) = self.version_list.first_child() {
                    self.version_list.remove(&child);
                }

                for v in &self.versions {
                    let row = adw::ActionRow::builder()
                        .title(&format!("{} · {}", v.name, v.version_number))
                        .activatable(true)
                        .build();
                    self.version_list.append(&row);
                }
            }
            QueueRowInput::FilterVersions(query) => {
                let query = query.to_lowercase();
                
                for (i, v) in self.versions.iter().enumerate() {
                    if let Some(row) = self.version_list.row_at_index(i as i32) {
                        let text = format!("{} {}", v.name, v.version_number).to_lowercase();
                        row.set_visible(query.is_empty() || text.contains(&query));
                    }
                }
            }
            QueueRowInput::Reset => {
                self.version_label.set_markup("<span size='small'>Latest Compatible</span>");
                self.item.version_id = None;
                self.item.version_name = None;
                self.item.filename = None;
                sender.output(QueueRowOutput::SelectVersion(self.project_id.clone(), "".to_string(), "".to_string(), "".to_string())).ok();
            }
            QueueRowInput::OpenPopover => {
                self.popover.popup();
            }
            QueueRowInput::Select(idx) => {
                self.popover.popdown();

                if let Some(v) = self.versions.get(idx as usize) {
                    self.version_label.set_markup(&format!("<span size='small'>{} · {}</span>", glib::markup_escape_text(&v.name), glib::markup_escape_text(&v.version_number)));
                    let filename = v.files.iter().find(|f| f.primary).or_else(|| v.files.first()).map(|f| f.filename.clone()).unwrap_or_default();
                    self.item.version_id = Some(v.id.clone());
                    self.item.version_name = Some(v.name.clone());
                    self.item.filename = Some(filename.clone());
                    sender.output(QueueRowOutput::SelectVersion(self.project_id.clone(), v.id.clone(), v.name.clone(), filename)).ok();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum BrowserInput {
    Open(String, ModLoader),
    Close,

    GoBack,
    StartSearch(String),
    Search(String),
    SearchDone(Result<ModrinthSearchResult, String>),
    LoadMore,
    LoadMoreDone(Result<ModrinthSearchResult, String>),
    Refresh,

    ClearQueue,
    PromptInstallQueue,
    ConfirmInstallQueue,
    OpenQueueDialog,

    ToggleQueueFromDetails,
    ShowDetails(String),
    ShowList,
    DetailsLoaded(Result<(ModrinthProject, Vec<ModrinthVersion>), String>),

    ToggleQueueItem(String, String),

    OpenProjectDetails(String),
    FetchVersions(String),
    VersionsFetched(String, Result<Vec<ModrinthVersion>, String>),
    ApplyVersionToQueue(String, String, String, String),
    InstallProject(String, String),
    AddVersionToQueue(String, String, String, String),
    IconLoaded(String, gdk::Texture),
    RemoveQueueItem(String),
    ShowToast(String),
}

#[derive(Debug)]
pub enum BrowserOutput {
    InstallMods(Vec<(String, String)>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserView {
    Overview,
    Results,
    Details,
}

pub struct ModrinthBrowser {
    visible: bool,
    game_version: String,
    loader: ModLoader,

    search_query: String,
    loading: bool,
    loading_more: bool,
    offset: u32,
    total_hits: u32,
    error: Option<String>,

    download_queue: HashMap<String, QueueItem>,
    icon_cache: HashMap<String, gdk::Texture>,
    author_cache: HashMap<String, String>,

    selected_project: Option<ModrinthProject>,
    loading_details: bool,

    view_state: BrowserView,
    queue_dialog: Controller<QueueDialog>,
    toast_overlay: adw::ToastOverlay,

    projects: FactoryVecDeque<ProjectRow>,
    versions: FactoryVecDeque<VersionRow>,
}

impl ModrinthBrowser {
}

#[relm4::component(pub)]
impl SimpleComponent for ModrinthBrowser {
    type Init = ();
    type Input = BrowserInput;
    type Output = BrowserOutput;

    view! {
        adw::Window {
            set_title: Some("Browse Mods"),
            set_default_width: 950,
            set_default_height: 700,
            set_modal: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(BrowserInput::Close);
                gtk::glib::Propagation::Stop
            },

            #[name = "toast_overlay"]
            #[wrap(Some)]
            set_content = &adw::ToastOverlay {
                adw::ToolbarView {
                    #[wrap(Some)]
                    set_content = &adw::NavigationSplitView {
                        #[watch]
                        set_show_content: model.view_state == BrowserView::Details || model.view_state == BrowserView::Overview,
                        set_vexpand: true,
                        set_min_sidebar_width: 350.0,
                        set_max_sidebar_width: 500.0,

                    #[wrap(Some)]
                    set_sidebar = &adw::NavigationPage {
                        set_title: "Modrinth Browser",
                        #[wrap(Some)]
                        set_child = &adw::ToolbarView {
                            add_top_bar = &adw::HeaderBar {
                                set_css_classes: &["flat"],
                                pack_end = &gtk::Button {
                                    set_icon_name: "view-refresh-symbolic",
                                    set_tooltip_text: Some("Refresh (clear cache)"),
                                    connect_clicked => BrowserInput::Refresh,
                                },
                            },

                            #[wrap(Some)]
                            set_content = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,

                                gtk::SearchEntry {
                                    set_margin_all: 8,
                                    set_placeholder_text: Some("Search Modrinth..."),
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
                                            set_child = &gtk::Spinner {
                                                set_spinning: true,
                                                set_halign: gtk::Align::Center,
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
                                            set_title: "Error fetching data",
                                            #[watch]
                                            set_description: model.error.as_deref(),
                                            set_icon_name: Some("dialog-error-symbolic"),
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
                            _ => "Modrinth Browser",
                        },
                        #[wrap(Some)]
                        set_child = &gtk::Stack {
                            set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                            #[watch]
                            set_visible_child_name: match model.view_state {
                                BrowserView::Details => "details",
                                _ => "overview",
                            },

                            // --- 1. Overview Page (Icon + Title) ---
                            add_named[Some("overview")] = &adw::NavigationPage {
                                set_title: "Overview",
                                #[wrap(Some)]
                                set_child = &adw::ToolbarView {
                                    add_top_bar = &adw::HeaderBar {
                                        set_css_classes: &["flat"],
                                        set_show_title: false,
                                    },

                                    #[wrap(Some)]
                                    set_content = &adw::StatusPage {
                                        set_vexpand: true,
                                        set_title: "Modrinth Mod Browser",
                                        set_description: Some("Search and discover mods for your instance from Modrinth."),
                                        set_icon_name: Some("web-browser-symbolic"),
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

                                    adw::HeaderBar {
                                        pack_start = &gtk::Button {
                                            set_icon_name: "go-previous-symbolic",
                                            connect_clicked => BrowserInput::GoBack,
                                        },
                                    },

                                    gtk::ScrolledWindow {
                                        set_vexpand: true,
                                        set_hscrollbar_policy: gtk::PolicyType::Never,

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 24,
                                            set_margin_all: 32,

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
                                                        #[watch]
                                                        set_visible_child_name: if model.selected_project.as_ref().and_then(|p| p.icon_url.as_ref()).and_then(|url| model.icon_cache.get(url)).is_some() { "icon" } else { "fallback" },

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
                                                        }
                                                    },
                                                    gtk::Box {
                                                        set_orientation: gtk::Orientation::Vertical,
                                                        set_valign: gtk::Align::Center,
                                                        set_spacing: 4,
                                                        gtk::Label {
                                                            #[watch]
                                                            set_label: &escape(model.selected_project.as_ref().map(|p| p.title.as_str()).unwrap_or("")),
                                                            set_css_classes: &["title-1"],
                                                            set_halign: gtk::Align::Start,
                                                            set_use_markup: true,
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
                                                        }
                                                    },
                                                    gtk::Box { set_hexpand: true },

                                                    gtk::Button {
                                                        #[watch]
                                                        set_label: if model.selected_project.as_ref().map(|p| model.download_queue.contains_key(&p.project_id)).unwrap_or(false) { "Remove from Queue" } else { "Add to Queue" },
                                                        #[watch]
                                                        set_icon_name: if model.selected_project.as_ref().map(|p| model.download_queue.contains_key(&p.project_id)).unwrap_or(false) { "list-remove-symbolic" } else { "list-add-symbolic" },
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
                                                        set_label: &format!("⬇ {}", format_downloads(model.selected_project.as_ref().map(|p| p.downloads).unwrap_or(0))),
                                                        set_css_classes: &["pill-badge"],
                                                    },
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &format!("♥ {}", format_downloads(model.selected_project.as_ref().map(|p| p.follows).unwrap_or(0))),
                                                        set_css_classes: &["pill-badge"],
                                                    },
                                                },

                                                // Description
                                                adw::PreferencesGroup {
                                                    set_title: "Description",
                                                    gtk::Label {
                                                        #[watch]
                                                        set_label: &escape(model.selected_project.as_ref().map(|p| p.description.as_str()).unwrap_or("")),
                                                        set_wrap: true,
                                                        set_halign: gtk::Align::Start,
                                                        set_margin_all: 12,
                                                        set_use_markup: true,
                                                    }
                                                },

                                                // Links section
                                                adw::PreferencesGroup {
                                                    set_title: "Links",
                                                    #[watch]
                                                    set_visible: model.selected_project.as_ref().map(|p| {
                                                        p.source_url.is_some() || p.wiki_url.is_some() || p.discord_url.is_some()
                                                    }).unwrap_or(false),

                                                    adw::ActionRow {
                                                        set_title: "Source Code",
                                                        add_prefix = &gtk::Image { set_icon_name: Some("code-symbolic"), set_pixel_size: 16 },
                                                        #[watch]
                                                        set_visible: model.selected_project.as_ref().and_then(|p| p.source_url.as_ref()).is_some(),
                                                        #[watch]
                                                        set_subtitle: &escape(model.selected_project.as_ref().and_then(|p| p.source_url.as_deref()).unwrap_or("")),
                                                        set_activatable: true,
                                                    },
                                                    adw::ActionRow {
                                                        set_title: "Wiki / Docs",
                                                        add_prefix = &gtk::Image { set_icon_name: Some("accessories-dictionary-symbolic"), set_pixel_size: 16 },
                                                        #[watch]
                                                        set_visible: model.selected_project.as_ref().and_then(|p| p.wiki_url.as_ref()).is_some(),
                                                        #[watch]
                                                        set_subtitle: &escape(model.selected_project.as_ref().and_then(|p| p.wiki_url.as_deref()).unwrap_or("")),
                                                        set_activatable: true,
                                                    },
                                                    adw::ActionRow {
                                                        set_title: "Discord",
                                                        add_prefix = &gtk::Image { set_icon_name: Some("user-available-symbolic"), set_pixel_size: 16 },
                                                        #[watch]
                                                        set_visible: model.selected_project.as_ref().and_then(|p| p.discord_url.as_ref()).is_some(),
                                                        #[watch]
                                                        set_subtitle: &escape(model.selected_project.as_ref().and_then(|p| p.discord_url.as_deref()).unwrap_or("")),
                                                        set_activatable: true,
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
                                                            .and_then(|p| p.license_name())
                                                            .map(|l| format!("Licensed under {}", l))
                                                            .unwrap_or_default()
                                                    },
                                                    #[watch]
                                                    set_visible: model.selected_project.as_ref().and_then(|p| p.license_name()).is_some(),
                                                    set_halign: gtk::Align::Center,
                                                    set_css_classes: &["dim-label", "caption"],
                                                    set_margin_top: 8,
                                                    set_margin_bottom: 16,
                                                },
                                            },
                                        },
                                    }
                                }
                            },
                        }
                    }
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
                            set_label: "Manage",
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
                VersionRowOutput::AddToQueue(pid, vid, vname, fname) => BrowserInput::AddVersionToQueue(pid, vid, vname, fname),
            });

        let queue_dialog = QueueDialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                QueueDialogOutput::Remove(id) => BrowserInput::RemoveQueueItem(id),
                QueueDialogOutput::Clear => BrowserInput::ClearQueue,
                QueueDialogOutput::Install => BrowserInput::PromptInstallQueue,
                QueueDialogOutput::FetchVersions(id) => BrowserInput::FetchVersions(id),
                QueueDialogOutput::SelectVersion(id, vid, vname, fname) => {
                    BrowserInput::ApplyVersionToQueue(id, vid, vname, fname)
                }
            });

        let mut model = ModrinthBrowser {
            visible: false,
            game_version: String::new(),
            loader: ModLoader::None,
            search_query: String::new(),
            loading: false,
            loading_more: false,
            offset: 0,
            total_hits: 0,
            error: None,
            download_queue: HashMap::new(),
            icon_cache: HashMap::new(),
            author_cache: HashMap::new(),
            selected_project: None,
            loading_details: false,
            view_state: BrowserView::Overview,
            projects,
            versions,
            queue_dialog,
            toast_overlay: adw::ToastOverlay::new(),
        };

        let projects_list = model.projects.widget();
        let versions_list = model.versions.widget();
        let widgets = view_output!();
        
        model.toast_overlay = widgets.toast_overlay.clone();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            BrowserInput::Open(game_version, loader) => {
                self.visible = true;
                self.game_version = game_version;
                self.loader = loader;
                self.view_state = BrowserView::Overview;
                self.error = None;
                self.download_queue.clear();
                self.projects.guard().clear();
                self.versions.guard().clear();
                self.search_query.clear();
                self.queue_dialog.emit(QueueDialogInput::ClearCache);

                // Trigger default search
                sender.input(BrowserInput::Search("".to_string()));
            }
            BrowserInput::Close => {
                self.visible = false;
            }
            BrowserInput::GoBack => match self.view_state {
                BrowserView::Details | BrowserView::Results => {
                    self.view_state = BrowserView::Overview
                }
                BrowserView::Overview => {}
            },
            BrowserInput::Refresh => {
                clear_caches();
                if !self.search_query.is_empty() || self.view_state == BrowserView::Results {
                    sender.input(BrowserInput::Search(self.search_query.clone()));
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
                let s_clone = sender.input_sender().clone();

                std::thread::spawn(move || {
                    let result = search_mods(&query, 20, 0, Some(&gv), Some(l), Some("mod"));
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
                if self.loading || self.loading_more || (self.projects.len() as u32) >= self.total_hits {
                    return;
                }
                self.loading_more = true;
                let query = self.search_query.clone();
                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let s_clone = sender.input_sender().clone();
                let next_offset = self.offset + 20;

                std::thread::spawn(move || {
                    let result = search_mods(&query, 20, next_offset, Some(&gv), Some(l), Some("mod"));
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
                        sender.input(BrowserInput::ShowToast(format!("Failed to load more: {}", e)));
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
                let title = self.selected_project.as_ref().map(|p| p.title.clone()).unwrap_or_else(|| "Unknown".to_string());
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
                self.queue_dialog.emit(QueueDialogInput::Open(self.download_queue.clone()));
            }
            BrowserInput::OpenProjectDetails(id) => {
                sender.input(BrowserInput::ShowDetails(id));
            }
            BrowserInput::ShowDetails(id) => {
                self.view_state = BrowserView::Details;
                self.loading_details = true;
                self.selected_project = None;
                self.versions.guard().clear();

                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let id_clone = id.clone();

                std::thread::spawn(move || {
                    let result = (|| {
                        let project =
                            crate::backend::download::modrinth::get_project(&id_clone)?;
                        let versions =
                            get_project_versions(&id_clone, Some(&gv), Some(l))?;
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
                    installs.push((
                        id.clone(),
                        item.version_id.clone().unwrap_or_default(),
                    ));
                }
                sender.output(BrowserOutput::InstallMods(installs)).ok();
                self.visible = false;
                self.queue_dialog.emit(QueueDialogInput::Close);
                sender.input(BrowserInput::ClearQueue);
            }
            BrowserInput::InstallProject(pid, vid) => {
                sender
                    .output(BrowserOutput::InstallMods(vec![(pid, vid)]))
                    .ok();
                self.visible = false;
            }
            BrowserInput::FetchVersions(id) => {
                let gv = self.game_version.clone();
                let l = self.loader.clone();
                let s_clone = sender.input_sender().clone();
                let id_clone = id.clone();

                std::thread::spawn(move || {
                    let result = get_project_versions(&id_clone, Some(&gv), Some(l));
                    s_clone.send(BrowserInput::VersionsFetched(id_clone, result)).ok();
                });
            }
            BrowserInput::VersionsFetched(id, result) => {
                if let Ok(versions) = result {
                    self.queue_dialog.emit(QueueDialogInput::SetVersions(id, versions));
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
                    // We don't call Open here anymore to avoid redundant list rebuilding and churn.
                    // The dialog will update the row in-place if it's already open.
                }
            }
            BrowserInput::ShowToast(msg) => {
                self.toast_overlay.add_toast(adw::Toast::new(&msg));
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
    SetVersions(String, Vec<ModrinthVersion>),
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
    version_cache: HashMap<String, Vec<ModrinthVersion>>,
    queue_rows: FactoryVecDeque<QueueRow>,
}

#[relm4::component(pub)]
impl SimpleComponent for QueueDialog {
    type Init = ();
    type Input = QueueDialogInput;
    type Output = QueueDialogOutput;

    view! {
        #[name = "window"]
        adw::Window {
            set_title: Some("Download Queue"),
            set_default_width: 500,
            set_default_height: 400,
            set_modal: true,
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(QueueDialogInput::Close);
                gtk::glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::HeaderBar {
                    pack_start = &gtk::ToggleButton {
                        set_icon_name: "system-search-symbolic",
                        #[watch]
                        set_active: search_bar.is_search_mode(),
                        connect_toggled[search_bar] => move |btn| {
                            search_bar.set_search_mode(btn.is_active());
                        }
                    },
                    pack_end = &gtk::Button {
                        set_label: "Install",
                        set_css_classes: &["suggested-action"],
                        connect_clicked => QueueDialogInput::Install,
                        #[watch]
                        set_sensitive: !model.all_items.is_empty(),
                    }
                },

                #[name = "search_bar"]
                gtk::SearchBar {
                    set_key_capture_widget: Some(&window),
                    #[wrap(Some)]
                    set_child = &gtk::SearchEntry {
                        set_placeholder_text: Some("Search queue..."),
                        connect_search_changed[sender] => move |entry| {
                            sender.input(QueueDialogInput::Search(entry.text().to_string()));
                        }
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    set_hscrollbar_policy: gtk::PolicyType::Never,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 12,
                        set_spacing: 6,

                        #[local_ref]
                        queue_list -> gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_css_classes: &["boxed-list"],
                        }
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
                    QueueRowOutput::SelectVersion(id, vid, vname, fname) => QueueDialogInput::SelectVersion(id, vid, vname, fname),
                }),
        };

        let queue_list = model.queue_rows.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            QueueDialogInput::Open(items) => {
                self.all_items = items;
                self.visible = true;
                self.rebuild_list();

                // Proactively fetch versions only for missing items
                for id in self.all_items.keys() {
                    if !self.version_cache.contains_key(id) {
                        sender.output(QueueDialogOutput::FetchVersions(id.clone())).ok();
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
            }
            QueueDialogInput::Close => {
                self.visible = false;
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
                sender.output(QueueDialogOutput::SelectVersion(id, vid, vname, fname)).ok();
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
            .user_agent("minecraft-launcher-rs (github.com/magnotec/minecraft-manager)")
            .build()
            .unwrap();
        match client.get(&url).send() {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("[modrinth-icon] HTTP {} for {}", response.status(), url);
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
                                eprintln!("[modrinth-icon] Texture decode error for {}: {}", url, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[modrinth-icon] Body read error for {}: {}", url, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[modrinth-icon] Request error for {}: {}", url, e);
            }
        }
    });
}
