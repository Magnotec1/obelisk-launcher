#![allow(unused_assignments)]
use crate::backend::instance::manager::{create_instance, CreateInstanceOptions, ModLoader};
use crate::backend::runtime::versions::{
    fetch_versions, filter_versions, MinecraftVersion, VersionType,
};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;

// Factory component for version rows in the list
#[derive(Debug)]
pub struct VersionRow {
    id: String,
    version_type: String,
    selected: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (String, String, bool);
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
                set_label: &self.version_type,
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
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        self.selected = msg;
    }
}

pub struct AddInstanceDialog {
    visible: bool,
    instances_path: Option<PathBuf>,
    name: String,
    selected_version: Option<String>,
    error_message: Option<String>,

    // Version data
    all_versions: Vec<MinecraftVersion>,
    filtered_versions: Vec<MinecraftVersion>,
    version_list: FactoryVecDeque<VersionRow>,
    versions_loading: bool,
    versions_error: Option<String>,

    // Models for ComboRows

    // Filter toggles
    show_releases: bool,
    show_snapshots: bool,
    show_betas: bool,
    show_alphas: bool,
    show_experiments: bool,

    // Search filter
    search_text: String,

    // UI Widgets for manual updates
    name_entry: Option<adw::EntryRow>,
    search_entry: Option<gtk::SearchEntry>,
}

#[derive(Debug)]
pub enum AddInstanceInput {
    Open,
    Close,
    SetName(String),
    SelectVersion(usize),
    Create,
    UpdateInstancesPath(Option<PathBuf>),

    // Version loading
    VersionsLoaded(Result<Vec<MinecraftVersion>, String>),

    // Filters
    ToggleReleases(bool),
    ToggleSnapshots(bool),
    ToggleBetas(bool),
    ToggleAlphas(bool),
    ToggleExperiments(bool),
    SearchChanged(String),
}

#[derive(Debug)]
pub enum AddInstanceOutput {
    InstanceCreated(MinecraftVersion, PathBuf),
}

impl AddInstanceDialog {
    fn active_filters(&self) -> Vec<VersionType> {
        let mut types = Vec::new();
        if self.show_releases {
            types.push(VersionType::Release);
        }
        if self.show_snapshots {
            types.push(VersionType::Snapshot);
        }
        if self.show_betas {
            types.push(VersionType::OldBeta);
        }
        if self.show_alphas {
            types.push(VersionType::OldAlpha);
        }
        if self.show_experiments {
            types.push(VersionType::Experiment);
        }
        types
    }

    fn rebuild_version_list(&mut self) {
        let types = self.active_filters();
        let mut filtered = filter_versions(&self.all_versions, &types);

        // Apply text search
        if !self.search_text.is_empty() {
            let query = self.search_text.to_lowercase();
            filtered.retain(|v| v.id.to_lowercase().contains(&query));
        }

        self.filtered_versions = filtered;

        let mut guard = self.version_list.guard();
        guard.clear();

        // Limit to 100 items to prevent UI freeze
        for v in self.filtered_versions.iter().take(100) {
            let is_selected = self.selected_version.as_ref() == Some(&v.id);
            guard.push_back((v.id.clone(), v.version_type.as_str().to_string(), is_selected));
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for AddInstanceDialog {
    type Init = Option<PathBuf>;
    type Input = AddInstanceInput;
    type Output = AddInstanceOutput;

    view! {
        adw::Dialog {
            set_title: "Add Instance",
            set_content_width: 500,
            set_content_height: 580,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "New Instance",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 0,

                    adw::PreferencesPage {
                        // Instance name group
                        adw::PreferencesGroup {
                            set_title: "Instance Details",
                            #[name = "name_entry"]
                            adw::EntryRow {
                                set_title: "Instance Name",
                                connect_changed[sender] => move |entry| {
                                    sender.input(AddInstanceInput::SetName(entry.text().to_string()));
                                },
                            },
                        },

                        // Version selection group
                        adw::PreferencesGroup {
                            set_title: "Minecraft Version",
                            #[watch]
                            set_description: Some(&if let Some(ref v) = model.selected_version {
                                format!("Selected: {}", v)
                            } else {
                                "Select a version below".to_string()
                            }),

                            // Search bar with filter button
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 6,
                                set_margin_bottom: 8,

                                #[name = "search_entry"]
                                gtk::SearchEntry {
                                    set_hexpand: true,
                                    set_placeholder_text: Some("Search versions..."),
                                    connect_search_changed[sender] => move |entry| {
                                        sender.input(AddInstanceInput::SearchChanged(entry.text().to_string()));
                                    },
                                },

                                gtk::MenuButton {
                                    set_icon_name: "funnel-symbolic",
                                    set_tooltip_text: Some("Filter version types"),
                                    set_valign: gtk::Align::Center,
                                    #[wrap(Some)]
                                    set_popover = &gtk::Popover {
                                        set_autohide: true,
                                        #[wrap(Some)]
                                        set_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            set_spacing: 4,
                                            set_margin_all: 8,

                                            gtk::CheckButton {
                                                set_label: Some("Releases"),
                                                set_active: true,
                                                connect_toggled[sender] => move |btn| {
                                                    sender.input(AddInstanceInput::ToggleReleases(btn.is_active()));
                                                },
                                            },
                                            gtk::CheckButton {
                                                set_label: Some("Snapshots"),
                                                set_active: false,
                                                connect_toggled[sender] => move |btn| {
                                                    sender.input(AddInstanceInput::ToggleSnapshots(btn.is_active()));
                                                },
                                            },
                                            gtk::CheckButton {
                                                set_label: Some("Beta"),
                                                set_active: false,
                                                connect_toggled[sender] => move |btn| {
                                                    sender.input(AddInstanceInput::ToggleBetas(btn.is_active()));
                                                },
                                            },
                                            gtk::CheckButton {
                                                set_label: Some("Alpha"),
                                                set_active: false,
                                                connect_toggled[sender] => move |btn| {
                                                    sender.input(AddInstanceInput::ToggleAlphas(btn.is_active()));
                                                },
                                            },
                                            gtk::CheckButton {
                                                set_label: Some("Experiments"),
                                                set_active: false,
                                                connect_toggled[sender] => move |btn| {
                                                    sender.input(AddInstanceInput::ToggleExperiments(btn.is_active()));
                                                },
                                            },
                                        },
                                    },
                                },
                            },

                            // Loading indicator
                            gtk::Spinner {
                                #[watch]
                                set_visible: model.versions_loading,
                                set_spinning: true,
                                set_halign: gtk::Align::Center,
                                set_margin_all: 20,
                            },

                            // Error label
                            gtk::Label {
                                #[watch]
                                set_visible: model.versions_error.is_some() && !model.versions_loading,
                                #[watch]
                                set_label: model.versions_error.as_deref().unwrap_or(""),
                                set_css_classes: &["error"],
                                set_wrap: true,
                            },

                            // Version list
                            gtk::ScrolledWindow {
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_min_content_height: 200,
                                set_max_content_height: 300,
                                #[watch]
                                set_visible: !model.versions_loading && model.versions_error.is_none(),

                                #[local_ref]
                                version_list_box -> gtk::ListBox {
                                    set_css_classes: &["boxed-list"],
                                    set_selection_mode: gtk::SelectionMode::None,
                                },
                            },
                        },

                        // Error + create button
                        adw::PreferencesGroup {
                            gtk::Label {
                                #[watch]
                                set_visible: model.error_message.is_some(),
                                #[watch]
                                set_label: model.error_message.as_deref().unwrap_or(""),
                                set_css_classes: &["error", "caption"],
                                set_wrap: true,
                                set_halign: gtk::Align::Start,
                                set_margin_bottom: 8,
                            },

                            gtk::Button {
                                set_label: "Create Instance",
                                set_css_classes: &["suggested-action", "pill"],
                                set_halign: gtk::Align::Center,
                                set_margin_top: 8,

                                #[watch]
                                set_sensitive: !model.name.is_empty()
                                    && model.selected_version.is_some()
                                    && model.instances_path.is_some(),

                                connect_clicked => AddInstanceInput::Create,
                            },
                        },
                    },
                },
            }
        }
    }

    fn init(
        instances_path: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let version_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), AddInstanceInput::SelectVersion);

        let model = AddInstanceDialog {
            visible: false,
            instances_path,
            name: String::new(),
            selected_version: None,
            error_message: None,
            all_versions: Vec::new(),
            filtered_versions: Vec::new(),
            version_list,
            versions_loading: false,
            versions_error: None,
            show_releases: true,
            show_snapshots: false,
            show_betas: false,
            show_alphas: false,
            show_experiments: false,
            search_text: String::new(),
            name_entry: None,
            search_entry: None,
        };

        let version_list_box = model.version_list.widget();
        let widgets = view_output!();

        let mut model = model;
        model.name_entry = Some(widgets.name_entry.clone());
        model.search_entry = Some(widgets.search_entry.clone());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AddInstanceInput::Open => {
                self.visible = true;
                self.name.clear();
                if let Some(entry) = &self.name_entry {
                    entry.set_text("");
                }
                self.selected_version = None;
                if let Some(entry) = &self.search_entry {
                    entry.set_text("");
                }

                // Fetch versions if not already loaded
                if self.all_versions.is_empty() && !self.versions_loading {
                    self.versions_loading = true;
                    self.versions_error = None;

                    let sender_clone = sender.input_sender().clone();
                    std::thread::spawn(move || {
                        let result = fetch_versions();
                        sender_clone
                            .send(AddInstanceInput::VersionsLoaded(result))
                            .ok();
                    });
                }
            }
            AddInstanceInput::Close => {
                self.visible = false;
            }
            AddInstanceInput::SetName(name) => {
                self.name = name;
                self.error_message = None;
            }
            AddInstanceInput::SelectVersion(index) => {
                if let Some(v) = self.filtered_versions.get(index) {
                    self.selected_version = Some(v.id.clone());
                    self.error_message = None;
                    for i in 0..self.version_list.len() {
                        if let Some(row) = self.version_list.get(i) {
                            let is_sel = row.id == v.id;
                            self.version_list.send(i, is_sel);
                        }
                    }
                }
            }

            AddInstanceInput::UpdateInstancesPath(path) => {
                self.instances_path = path;
            }
            AddInstanceInput::VersionsLoaded(result) => {
                self.versions_loading = false;
                match result {
                    Ok(versions) => {
                        self.all_versions = versions;
                        self.versions_error = None;
                        self.rebuild_version_list();
                    }
                    Err(e) => {
                        self.versions_error = Some(e);
                    }
                }
            }
            AddInstanceInput::ToggleReleases(active) => {
                self.show_releases = active;
                self.rebuild_version_list();
            }
            AddInstanceInput::ToggleSnapshots(active) => {
                self.show_snapshots = active;
                self.rebuild_version_list();
            }
            AddInstanceInput::ToggleBetas(active) => {
                self.show_betas = active;
                self.rebuild_version_list();
            }
            AddInstanceInput::ToggleAlphas(active) => {
                self.show_alphas = active;
                self.rebuild_version_list();
            }
            AddInstanceInput::ToggleExperiments(active) => {
                self.show_experiments = active;
                self.rebuild_version_list();
            }
            AddInstanceInput::SearchChanged(text) => {
                self.search_text = text;
                self.rebuild_version_list();
            }
            AddInstanceInput::Create => {
                let Some(instances_path) = &self.instances_path else {
                    self.error_message = Some("No instances directory configured.".to_string());
                    return;
                };

                if self.name.trim().is_empty() {
                    self.error_message = Some("Instance name cannot be empty.".to_string());
                    return;
                }

                let Some(ref version) = self.selected_version else {
                    self.error_message = Some("Please select a Minecraft version.".to_string());
                    return;
                };

                let options = CreateInstanceOptions {
                    name: self.name.trim().to_string(),
                    minecraft_version: version.clone(),
                    mod_loader: ModLoader::None,
                    loader_version: None,
                };

                let selected_v_data = self
                    .filtered_versions
                    .iter()
                    .find(|v| v.id == *version)
                    .cloned();

                match create_instance(instances_path, options) {
                    Ok(path) => {
                        self.visible = false;
                        self.error_message = None;
                        if let Some(v) = selected_v_data {
                            sender
                                .output(AddInstanceOutput::InstanceCreated(v, path))
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        self.error_message = Some(e);
                    }
                }
            }
        }
    }
}
