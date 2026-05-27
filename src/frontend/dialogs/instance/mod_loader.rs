#![allow(unused_assignments)]
use crate::backend::instance::manager::ModLoader;
use crate::backend::runtime::versions::{fetch_loader_versions, LoaderVersion};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// ── Version Row Factory ──────────────────────────────────────────────────────

#[derive(Debug)]
pub struct VersionRow {
    id: String,
    version_type: String,
    selected: bool,
    is_newest: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (String, String, bool, bool);
    type Input = bool;
    type Output = usize;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.id,
            add_prefix = &gtk::Image {
                set_icon_name: Some("object-select-symbolic"),
                #[watch]
                set_visible: self.selected,
            },
            add_suffix = &gtk::Label {
                #[watch]
                set_label: &if self.is_newest {
                    format!("{} (Latest)", self.version_type)
                } else {
                    self.version_type.clone()
                },
                set_css_classes: &["dim-label"],
            },
            set_activatable: true,
            connect_activated[sender, index] => move |_| {
                let _ = sender.output(index.current_index());
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            id: init.0,
            version_type: init.1,
            selected: init.2,
            is_newest: init.3,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        self.selected = msg;
    }
}

// ── Dialog Model ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Page {
    SelectLoader,
    SelectVersion,
}

pub struct ModLoaderDialog {
    visible: bool,
    page: Page,
    selected_loader: Option<ModLoader>,
    mc_version: Option<String>,

    // Cached versions for each loader
    fabric_versions: Option<Vec<LoaderVersion>>,
    quilt_versions: Option<Vec<LoaderVersion>>,
    forge_versions: Option<Vec<LoaderVersion>>,
    neoforge_versions: Option<Vec<LoaderVersion>>,

    // Loading state per loader
    fabric_loading: bool,
    quilt_loading: bool,
    forge_loading: bool,
    neoforge_loading: bool,

    // Version list
    version_list: FactoryVecDeque<VersionRow>,
    search_text: String,

    // Current loader display
    loader_title: String,
    selected_version: Option<String>,
}

#[derive(Debug)]
pub enum ModLoaderDialogInput {
    Open(Option<String>), // mc_version
    Close,
    SelectLoader(ModLoader),
    GoBack,
    FabricLoaded(Result<Vec<LoaderVersion>, String>),
    QuiltLoaded(Result<Vec<LoaderVersion>, String>),
    ForgeLoaded(Result<Vec<LoaderVersion>, String>),
    NeoForgeLoaded(Result<Vec<LoaderVersion>, String>),
    SelectVersion(usize),
    ConfirmInstall,
    SearchChanged(String),
}

#[derive(Debug)]
pub enum ModLoaderDialogOutput {
    InstallModLoader(ModLoader, String),
}

#[relm4::component(pub)]
impl SimpleComponent for ModLoaderDialog {
    type Init = ();
    type Input = ModLoaderDialogInput;
    type Output = ModLoaderDialogOutput;

    view! {
        adw::Dialog {
            set_title: "Install Mod Loader",
            set_content_width: 450,
            set_content_height: 500,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    pack_start = &gtk::Button {
                        set_icon_name: "go-previous-symbolic",
                        set_tooltip_text: Some("Back"),
                        #[watch]
                        set_visible: model.page == Page::SelectVersion,
                        connect_clicked[sender] => move |_| {
                            sender.input(ModLoaderDialogInput::GoBack);
                        },
                    },

                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        #[watch]
                        set_title: if model.page == Page::SelectLoader {
                            "Install Mod Loader"
                        } else {
                            &model.loader_title
                        },
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::SlideLeftRight,

                    add_named[Some("loader_page")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,
                        set_margin_all: 12,

                        gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_css_classes: &["boxed-list"],

                            adw::ActionRow {
                                set_title: "Fabric",
                                set_subtitle: "The lightweight, modular loader",
                                set_activatable: true,
                                add_suffix = &gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                },
                                connect_activated[sender] => move |_| {
                                    sender.input(ModLoaderDialogInput::SelectLoader(ModLoader::Fabric));
                                },
                            },

                            adw::ActionRow {
                                set_title: "Quilt",
                                set_subtitle: "The open-source community fork",
                                set_activatable: true,
                                add_suffix = &gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                },
                                connect_activated[sender] => move |_| {
                                    sender.input(ModLoaderDialogInput::SelectLoader(ModLoader::Quilt));
                                },
                            },

                            adw::ActionRow {
                                set_title: "Forge",
                                set_subtitle: "The classic, heavy-duty loader",
                                set_activatable: true,
                                add_suffix = &gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                },
                                connect_activated[sender] => move |_| {
                                    sender.input(ModLoaderDialogInput::SelectLoader(ModLoader::Forge));
                                },
                            },

                            adw::ActionRow {
                                set_title: "NeoForge",
                                set_subtitle: "The modern Forge successor",
                                set_activatable: true,
                                add_suffix = &gtk::Image {
                                    set_icon_name: Some("go-next-symbolic"),
                                },
                                connect_activated[sender] => move |_| {
                                    sender.input(ModLoaderDialogInput::SelectLoader(ModLoader::NeoForge));
                                },
                            },
                        }
                    },

                    add_named[Some("version_page")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 12,

                        gtk::SearchEntry {
                            set_placeholder_text: Some("Search versions..."),
                            connect_search_changed[sender] => move |entry| {
                                sender.input(ModLoaderDialogInput::SearchChanged(entry.text().to_string()));
                            },
                        },

                        // Loading spinner (adw::Spinner)
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_vexpand: true,
                            set_valign: gtk::Align::Center,
                            set_halign: gtk::Align::Center,
                            #[watch]
                            set_visible: model.is_current_loader_loading(),

                            adw::Spinner {
                                set_width_request: 32,
                                set_height_request: 32,
                            },
                        },

                        // Empty state
                        adw::StatusPage {
                            set_title: "No Versions Available",
                            set_description: Some("This mod loader has no versions available\nfor the current Minecraft version."),
                            set_icon_name: Some("dialog-information-symbolic"),
                            #[watch]
                            set_visible: model.is_current_loader_empty(),
                            set_vexpand: true,
                        },

                        // Version list
                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            #[watch]
                            set_visible: !model.is_current_loader_loading() && !model.is_current_loader_empty(),
                            #[local_ref]
                            version_list_box -> gtk::ListBox {
                                set_css_classes: &["boxed-list"],
                            }
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            set_margin_top: 12,
                            #[watch]
                            set_visible: model.page == Page::SelectVersion && !model.is_current_loader_loading() && !model.is_current_loader_empty(),

                            gtk::Button {
                                set_label: "Cancel",
                                set_css_classes: &["pill"],
                                set_hexpand: true,
                                connect_clicked[root] => move |_| {
                                    root.close();
                                }
                            },

                            gtk::Button {
                                set_label: "Install",
                                set_css_classes: &["pill", "suggested-action"],
                                set_hexpand: true,
                                #[watch]
                                set_sensitive: model.selected_version.is_some(),
                                connect_clicked[root, sender] => move |_| {
                                    sender.input(ModLoaderDialogInput::ConfirmInstall);
                                    root.close();
                                }
                            }
                        }
                    },

                    // Must come after add_named so children exist on first render
                    #[watch]
                    set_visible_child_name: if model.page == Page::SelectLoader {
                        "loader_page"
                    } else {
                        "version_page"
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
        let version_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), ModLoaderDialogInput::SelectVersion);

        let model = ModLoaderDialog {
            visible: false,
            page: Page::SelectLoader,
            selected_loader: None,
            mc_version: None,
            fabric_versions: None,
            quilt_versions: None,
            forge_versions: None,
            neoforge_versions: None,
            fabric_loading: false,
            quilt_loading: false,
            forge_loading: false,
            neoforge_loading: false,
            version_list,
            search_text: String::new(),
            loader_title: String::new(),
            selected_version: None,
        };

        let version_list_box = model.version_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ModLoaderDialogInput::Open(mc_version) => {
                self.visible = true;
                self.page = Page::SelectLoader;
                self.mc_version = mc_version.clone();
                self.search_text.clear();
                self.selected_loader = None;
                self.selected_version = None;
                self.version_list.guard().clear();

                // Reset cached versions
                self.fabric_versions = None;
                self.quilt_versions = None;
                self.forge_versions = None;
                self.neoforge_versions = None;

                // Pre-load all versions in parallel
                let mc_ver = mc_version.unwrap_or_default();

                self.fabric_loading = true;
                let sender_clone = sender.input_sender().clone();
                let mc_ver_clone = mc_ver.clone();
                std::thread::spawn(move || {
                    let res = fetch_loader_versions(&ModLoader::Fabric, &mc_ver_clone);
                    let _ = sender_clone.send(ModLoaderDialogInput::FabricLoaded(res));
                });

                self.quilt_loading = true;
                let sender_clone = sender.input_sender().clone();
                let mc_ver_clone = mc_ver.clone();
                std::thread::spawn(move || {
                    let res = fetch_loader_versions(&ModLoader::Quilt, &mc_ver_clone);
                    let _ = sender_clone.send(ModLoaderDialogInput::QuiltLoaded(res));
                });

                self.forge_loading = true;
                let sender_clone = sender.input_sender().clone();
                let mc_ver_clone = mc_ver.clone();
                std::thread::spawn(move || {
                    let res = fetch_loader_versions(&ModLoader::Forge, &mc_ver_clone);
                    let _ = sender_clone.send(ModLoaderDialogInput::ForgeLoaded(res));
                });

                self.neoforge_loading = true;
                let sender_clone = sender.input_sender().clone();
                let mc_ver_clone = mc_ver.clone();
                std::thread::spawn(move || {
                    let res = fetch_loader_versions(&ModLoader::NeoForge, &mc_ver_clone);
                    let _ = sender_clone.send(ModLoaderDialogInput::NeoForgeLoaded(res));
                });
            }
            ModLoaderDialogInput::Close => {
                self.visible = false;
            }
            ModLoaderDialogInput::SelectLoader(loader) => {
                self.selected_loader = Some(loader.clone());
                self.loader_title = format!("Select {} Version", loader.as_str());
                self.search_text.clear();
                self.selected_version = None;
                self.page = Page::SelectVersion;
                self.rebuild_list();
            }
            ModLoaderDialogInput::GoBack => {
                self.page = Page::SelectLoader;
                self.selected_loader = None;
                self.search_text.clear();
                self.selected_version = None;
                self.version_list.guard().clear();
            }
            ModLoaderDialogInput::FabricLoaded(res) => {
                self.fabric_loading = false;
                self.fabric_versions = Some(res.unwrap_or_default());
                if self.selected_loader == Some(ModLoader::Fabric) {
                    self.rebuild_list();
                }
            }
            ModLoaderDialogInput::QuiltLoaded(res) => {
                self.quilt_loading = false;
                self.quilt_versions = Some(res.unwrap_or_default());
                if self.selected_loader == Some(ModLoader::Quilt) {
                    self.rebuild_list();
                }
            }
            ModLoaderDialogInput::ForgeLoaded(res) => {
                self.forge_loading = false;
                self.forge_versions = Some(res.unwrap_or_default());
                if self.selected_loader == Some(ModLoader::Forge) {
                    self.rebuild_list();
                }
            }
            ModLoaderDialogInput::NeoForgeLoaded(res) => {
                self.neoforge_loading = false;
                self.neoforge_versions = Some(res.unwrap_or_default());
                if self.selected_loader == Some(ModLoader::NeoForge) {
                    self.rebuild_list();
                }
            }
            ModLoaderDialogInput::SearchChanged(text) => {
                self.search_text = text;
                self.rebuild_list();
            }
            ModLoaderDialogInput::SelectVersion(idx) => {
                let versions = self.get_current_versions();
                let filtered = self.filter_versions(&versions);
                if let Some(v) = filtered.get(idx) {
                    self.selected_version = Some(v.version.clone());
                    for i in 0..self.version_list.len() {
                        if let Some(row) = self.version_list.get(i) {
                            let is_sel = row.id == v.version;
                            self.version_list.send(i, is_sel);
                        }
                    }
                }
            }
            ModLoaderDialogInput::ConfirmInstall => {
                if let (Some(loader), Some(version)) = (&self.selected_loader, &self.selected_version) {
                    sender
                        .output(ModLoaderDialogOutput::InstallModLoader(
                            loader.clone(),
                            version.clone(),
                        ))
                        .ok();
                    self.visible = false;
                }
            }
        }
    }
}

impl ModLoaderDialog {
    fn get_current_versions(&self) -> Vec<LoaderVersion> {
        match &self.selected_loader {
            Some(ModLoader::Fabric) => self.fabric_versions.clone().unwrap_or_default(),
            Some(ModLoader::Quilt) => self.quilt_versions.clone().unwrap_or_default(),
            Some(ModLoader::Forge) => self.forge_versions.clone().unwrap_or_default(),
            Some(ModLoader::NeoForge) => self.neoforge_versions.clone().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    fn filter_versions<'a>(&self, versions: &'a [LoaderVersion]) -> Vec<&'a LoaderVersion> {
        let mut filtered: Vec<&LoaderVersion> = versions.iter().collect();
        if !self.search_text.is_empty() {
            let query = self.search_text.to_lowercase();
            filtered.retain(|v| v.version.to_lowercase().contains(&query));
        }
        filtered
    }

    fn rebuild_list(&mut self) {
        // Capture version data before taking the mutable guard
        let versions = self.get_current_versions();
        let search = self.search_text.to_lowercase();

        let mut guard = self.version_list.guard();
        guard.clear();

        let filtered: Vec<&LoaderVersion> = if search.is_empty() {
            versions.iter().collect()
        } else {
            versions.iter().filter(|v| v.version.to_lowercase().contains(&search)).collect()
        };

        for (i, v) in filtered.iter().take(100).enumerate() {
            let suffix = if v.stable { "Stable" } else { "Beta" };
            let is_selected = self.selected_version.as_ref() == Some(&v.version);
            let is_newest = i == 0;
            guard.push_back((v.version.clone(), suffix.to_string(), is_selected, is_newest));
        }
    }

    fn is_current_loader_loading(&self) -> bool {
        match &self.selected_loader {
            Some(ModLoader::Fabric) => self.fabric_loading,
            Some(ModLoader::Quilt) => self.quilt_loading,
            Some(ModLoader::Forge) => self.forge_loading,
            Some(ModLoader::NeoForge) => self.neoforge_loading,
            _ => false,
        }
    }

    fn is_current_loader_empty(&self) -> bool {
        if self.is_current_loader_loading() {
            return false;
        }
        match &self.selected_loader {
            Some(ModLoader::Fabric) => self.fabric_versions.as_ref().map_or(false, |v| v.is_empty()),
            Some(ModLoader::Quilt) => self.quilt_versions.as_ref().map_or(false, |v| v.is_empty()),
            Some(ModLoader::Forge) => self.forge_versions.as_ref().map_or(false, |v| v.is_empty()),
            Some(ModLoader::NeoForge) => self.neoforge_versions.as_ref().map_or(false, |v| v.is_empty()),
            _ => false,
        }
    }
}
