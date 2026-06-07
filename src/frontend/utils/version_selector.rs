use crate::backend::runtime::versions::{
    fetch_loader_versions_by_uid, fetch_versions, LoaderVersion, MinecraftVersion,
};
use crate::frontend::utils::VersionFilters;
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

// Factory component for version rows in the list
#[derive(Debug)]
pub struct VersionRow {
    pub id: String,
    pub version_type: String,
    pub selected: bool,
    pub is_current: bool,
    pub is_latest: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (String, String, bool, bool, bool);
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
                set_label: &{
                    let mut suffix = self.version_type.clone();
                    if self.is_current {
                        suffix = format!("{} (Current)", suffix);
                    }
                    if self.is_latest {
                        suffix = format!("{} (Latest)", suffix);
                    }
                    suffix
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
            is_current: init.3,
            is_latest: init.4,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        self.selected = msg;
    }
}

pub struct VersionSelector {
    uid: String,
    all_mc_versions: Vec<MinecraftVersion>,
    filtered_mc_versions: Vec<MinecraftVersion>,
    loader_versions: Vec<LoaderVersion>,
    selected_version: Option<String>,
    current_version: Option<String>,
    search_text: String,
    filters: VersionFilters,
    loading: bool,
    error: Option<String>,
    version_list: FactoryVecDeque<VersionRow>,
}

#[derive(Debug)]
pub enum VersionSelectorInput {
    Load {
        uid: String,
        mc_version: Option<String>,
        current_version: Option<String>,
        selected_version: Option<String>,
    },
    McVersionsLoaded(Result<Vec<MinecraftVersion>, String>),
    LoaderVersionsLoaded(Result<Vec<LoaderVersion>, String>),
    SelectVersionIndex(usize),
    SearchChanged(String),
    ToggleReleases(bool),
    ToggleSnapshots(bool),
    ToggleBetas(bool),
    ToggleAlphas(bool),
    ToggleExperiments(bool),
}

#[derive(Debug)]
pub enum VersionSelectorOutput {
    VersionSelected {
        version: String,
        mc_version: Option<MinecraftVersion>,
    },
}

#[relm4::component(pub)]
impl SimpleComponent for VersionSelector {
    type Init = ();
    type Input = VersionSelectorInput;
    type Output = VersionSelectorOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,

            // Search bar with filter button
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,

                gtk::SearchEntry {
                    set_hexpand: true,
                    set_placeholder_text: Some("Search versions..."),
                    connect_search_changed[sender] => move |entry| {
                        sender.input(VersionSelectorInput::SearchChanged(entry.text().to_string()));
                    },
                },

                gtk::MenuButton {
                    set_icon_name: "funnel-outline-symbolic",
                    set_tooltip_text: Some("Filter version types"),
                    set_valign: gtk::Align::Center,
                    #[watch]
                    set_visible: model.uid == "net.minecraft",
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
                                #[watch]
                                set_active: model.filters.show_releases,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(VersionSelectorInput::ToggleReleases(btn.is_active()));
                                },
                            },
                            gtk::CheckButton {
                                set_label: Some("Snapshots"),
                                #[watch]
                                set_active: model.filters.show_snapshots,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(VersionSelectorInput::ToggleSnapshots(btn.is_active()));
                                },
                            },
                            gtk::CheckButton {
                                set_label: Some("Beta"),
                                #[watch]
                                set_active: model.filters.show_betas,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(VersionSelectorInput::ToggleBetas(btn.is_active()));
                                },
                            },
                            gtk::CheckButton {
                                set_label: Some("Alpha"),
                                #[watch]
                                set_active: model.filters.show_alphas,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(VersionSelectorInput::ToggleAlphas(btn.is_active()));
                                },
                            },
                            gtk::CheckButton {
                                set_label: Some("Experiments"),
                                #[watch]
                                set_active: model.filters.show_experiments,
                                connect_toggled[sender] => move |btn| {
                                    sender.input(VersionSelectorInput::ToggleExperiments(btn.is_active()));
                                },
                            },
                        },
                    },
                },
            },

            // Loading indicator
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,
                set_vexpand: true,
                #[watch]
                set_visible: model.loading,

                adw::Spinner {
                    set_width_request: 32,
                    set_height_request: 32,
                },

                gtk::Label {
                    set_label: "Loading versions...",
                    set_css_classes: &["dim-label"],
                }
            },

            // Error label
            gtk::Label {
                #[watch]
                set_visible: model.error.is_some() && !model.loading,
                #[watch]
                set_label: model.error.as_deref().unwrap_or(""),
                set_css_classes: &["error"],
                set_wrap: true,
                set_halign: gtk::Align::Center,
            },

            // Version list
            gtk::ScrolledWindow {
                set_vexpand: true,
                set_hscrollbar_policy: gtk::PolicyType::Never,
                set_min_content_height: 200,
                #[watch]
                set_visible: !model.loading && model.error.is_none(),

                #[local_ref]
                version_list_box -> gtk::ListBox {
                    set_css_classes: &["boxed-list"],
                    set_selection_mode: gtk::SelectionMode::None,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let version_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), VersionSelectorInput::SelectVersionIndex);

        let model = VersionSelector {
            uid: String::new(),
            all_mc_versions: Vec::new(),
            filtered_mc_versions: Vec::new(),
            loader_versions: Vec::new(),
            selected_version: None,
            current_version: None,
            search_text: String::new(),
            filters: VersionFilters::new(),
            loading: false,
            error: None,
            version_list,
        };

        let version_list_box = model.version_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VersionSelectorInput::Load {
                uid,
                mc_version,
                current_version,
                selected_version,
            } => {
                self.uid = uid.clone();
                self.current_version = current_version;
                self.selected_version = selected_version;
                self.loading = true;
                self.error = None;
                self.search_text.clear();

                let sender_clone = sender.input_sender().clone();
                match uid.as_str() {
                    "net.minecraft" => {
                        std::thread::spawn(move || {
                            let result = fetch_versions();
                            let _ = sender_clone.send(VersionSelectorInput::McVersionsLoaded(result));
                        });
                    }
                    _ => {
                        let uid_clone = uid.clone();
                        let mc_version_val = mc_version.clone();
                        std::thread::spawn(move || {
                            let result = if let Some(mv) = mc_version_val {
                                fetch_loader_versions_by_uid(&uid_clone, &mv)
                            } else {
                                Ok(vec![])
                            };
                            let _ = sender_clone.send(VersionSelectorInput::LoaderVersionsLoaded(result));
                        });
                    }
                }
            }
            VersionSelectorInput::McVersionsLoaded(res) => {
                self.loading = false;
                match res {
                    Ok(versions) => {
                        self.all_mc_versions = versions;
                        self.rebuild_list();
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
            }
            VersionSelectorInput::LoaderVersionsLoaded(res) => {
                self.loading = false;
                match res {
                    Ok(versions) => {
                        self.loader_versions = versions;
                        self.rebuild_list();
                    }
                    Err(e) => {
                        self.error = Some(e);
                    }
                }
            }
            VersionSelectorInput::SelectVersionIndex(index) => {
                let selected = if self.uid == "net.minecraft" {
                    self.filtered_mc_versions.get(index).map(|v| v.id.clone())
                } else {
                    self.loader_versions.get(index).map(|v| v.version.clone())
                };

                if let Some(v) = selected {
                    self.selected_version = Some(v.clone());
                    if self.uid == "net.minecraft" {
                        let mc_v = self.filtered_mc_versions.iter().find(|mv| mv.id == v).cloned();
                        sender
                            .output(VersionSelectorOutput::VersionSelected {
                                version: v.clone(),
                                mc_version: mc_v,
                            })
                            .ok();
                    } else {
                        sender
                            .output(VersionSelectorOutput::VersionSelected {
                                version: v.clone(),
                                mc_version: None,
                            })
                            .ok();
                    }
                    for i in 0..self.version_list.len() {
                        if let Some(row) = self.version_list.get(i) {
                            let is_sel = row.id == v;
                            self.version_list.send(i, is_sel);
                        }
                    }
                }
            }
            VersionSelectorInput::SearchChanged(text) => {
                self.search_text = text;
                self.rebuild_list();
            }
            VersionSelectorInput::ToggleReleases(active) => {
                self.filters.show_releases = active;
                self.rebuild_list();
            }
            VersionSelectorInput::ToggleSnapshots(active) => {
                self.filters.show_snapshots = active;
                self.rebuild_list();
            }
            VersionSelectorInput::ToggleBetas(active) => {
                self.filters.show_betas = active;
                self.rebuild_list();
            }
            VersionSelectorInput::ToggleAlphas(active) => {
                self.filters.show_alphas = active;
                self.rebuild_list();
            }
            VersionSelectorInput::ToggleExperiments(active) => {
                self.filters.show_experiments = active;
                self.rebuild_list();
            }
        }
    }
}

impl VersionSelector {
    fn rebuild_list(&mut self) {
        let mut guard = self.version_list.guard();
        guard.clear();

        if self.uid == "net.minecraft" {
            self.filtered_mc_versions = self.filters.filter_and_limit(
                &self.all_mc_versions,
                &self.search_text,
                100,
            );

            for (i, v) in self.filtered_mc_versions.iter().enumerate() {
                let is_selected = self.selected_version.as_ref() == Some(&v.id);
                let is_current = self.current_version.as_ref() == Some(&v.id);
                let is_latest = i == 0 && self.search_text.is_empty();
                guard.push_back((
                    v.id.clone(),
                    v.version_type.as_str().to_string(),
                    is_selected,
                    is_current,
                    is_latest,
                ));
            }
        } else {
            let mut filtered = self.loader_versions.clone();
            if !self.search_text.is_empty() {
                let query = self.search_text.to_lowercase();
                filtered.retain(|v| v.version.to_lowercase().contains(&query));
            }
            filtered.truncate(100);

            for (i, v) in filtered.iter().enumerate() {
                let suffix = if v.stable { "Stable" } else { "Experimental" };
                let is_selected = self.selected_version.as_ref() == Some(&v.version);
                let is_current = self.current_version.as_ref() == Some(&v.version);
                let is_latest = i == 0 && self.search_text.is_empty();
                guard.push_back((
                    v.version.clone(),
                    suffix.to_string(),
                    is_selected,
                    is_current,
                    is_latest,
                ));
            }
        }
    }
}
