#![allow(unused_assignments)]
use crate::backend::runtime::versions::{
    fetch_loader_versions_by_uid, fetch_versions, filter_versions, LoaderVersion,
    MinecraftVersion, VersionType,
};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug)]
pub struct VersionRow {
    id: String,
    version_type: String,
    selected: bool,
    is_current: bool,
    is_newest: bool,
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
                    if self.is_newest {
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
            is_newest: init.4,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        self.selected = msg;
    }
}

pub struct ComponentEditorDialog {
    visible: bool,
    uid: String,
    title: String,

    // Minecraft versions
    all_mc_versions: Vec<MinecraftVersion>,
    filtered_mc_versions: Vec<MinecraftVersion>,

    // Loader versions
    loader_versions: Vec<LoaderVersion>,

    version_list: FactoryVecDeque<VersionRow>,
    loading: bool,
    error: Option<String>,

    search_text: String,
    show_releases: bool,
    show_snapshots: bool,

    selected_version: Option<String>,
    current_version: Option<String>,
}

#[derive(Debug)]
pub enum ComponentEditorInput {
    Open(String, Option<String>, Option<String>), // UID, MC version, Current Version
    Close,
    VersionsLoaded(Result<Vec<MinecraftVersion>, String>),
    LoadersLoaded(Result<Vec<LoaderVersion>, String>),
    SelectVersion(usize),
    ConfirmInstall,
    SearchChanged(String),
    ToggleSnapshots(bool),
    ToggleReleases(bool),
}

#[derive(Debug)]
pub enum ComponentEditorOutput {
    SetVersion(String, String), // UID, Version
}

#[relm4::component(pub)]
impl SimpleComponent for ComponentEditorDialog {
    type Init = ();
    type Input = ComponentEditorInput;
    type Output = ComponentEditorOutput;

    view! {
        adw::Dialog {
            #[watch]
            set_title: &model.title,
            set_content_width: 450,
            set_content_height: 500,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {

                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        #[watch]
                        set_title: &model.title,
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 12,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,

                        gtk::SearchEntry {
                            set_hexpand: true,
                            set_placeholder_text: Some("Search versions..."),
                            connect_search_changed[sender] => move |entry| {
                                sender.input(ComponentEditorInput::SearchChanged(entry.text().to_string()));
                            },
                        },

                        gtk::CheckButton {
                            set_label: Some("Releases"),
                            #[watch]
                            set_active: model.show_releases,
                            #[watch]
                            set_visible: model.uid == "net.minecraft",
                            connect_toggled[sender] => move |btn| {
                                sender.input(ComponentEditorInput::ToggleReleases(btn.is_active()));
                            },
                        },

                        gtk::CheckButton {
                            set_label: Some("Snapshots"),
                            #[watch]
                            set_active: model.show_snapshots,
                            #[watch]
                            set_visible: model.uid == "net.minecraft",
                            connect_toggled[sender] => move |btn| {
                                sender.input(ComponentEditorInput::ToggleSnapshots(btn.is_active()));
                            },
                        }
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

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[watch]
                        set_visible: !model.loading && model.error.is_none(),
                        #[local_ref]
                        version_list_box -> gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_margin_top: 12,

                        gtk::Button {
                            set_label: "Cancel",
                            set_css_classes: &["pill"],
                            set_hexpand: true,
                            connect_clicked[root] => move |_| {
                                root.close();
                            }
                        },

                        gtk::Button {
                            set_label: "Confirm",
                            set_css_classes: &["pill", "suggested-action"],
                            set_hexpand: true,
                            #[watch]
                            set_sensitive: !model.loading && model.selected_version.is_some() && model.selected_version != model.current_version,
                            connect_clicked[root, sender] => move |_| {
                                sender.input(ComponentEditorInput::ConfirmInstall);
                                root.close();
                            }
                        }
                    }
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
            .forward(sender.input_sender(), ComponentEditorInput::SelectVersion);

        let model = ComponentEditorDialog {
            visible: false,
            uid: String::new(),
            title: "Select Version".to_string(),
            all_mc_versions: Vec::new(),
            filtered_mc_versions: Vec::new(),
            loader_versions: Vec::new(),
            version_list,
            loading: false,
            error: None,
            search_text: String::new(),
            show_releases: true,
            show_snapshots: false,
            selected_version: None,
            current_version: None,
        };

        let version_list_box = model.version_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ComponentEditorInput::Open(uid, mc_version, current_ver) => {
                self.visible = true;
                self.uid = uid.clone();
                self.loading = true;
                self.error = None;
                self.search_text.clear();
                self.current_version = current_ver.clone();
                self.selected_version = current_ver;

                let sender_clone = sender.input_sender().clone();
                match uid.as_str() {
                    "net.minecraft" => {
                        self.title = "Select Minecraft Version".to_string();
                        std::thread::spawn(move || {
                            let res = fetch_versions();
                            let _ = sender_clone.send(ComponentEditorInput::VersionsLoaded(res));
                        });
                    }
                    "net.fabricmc.fabric-loader" | "org.quiltmc.quilt-loader" | "net.minecraftforge" | "net.neoforged" => {
                        let name = match uid.as_str() {
                            "net.fabricmc.fabric-loader" => "Fabric Loader",
                            "org.quiltmc.quilt-loader" => "Quilt Loader",
                            "net.minecraftforge" => "Forge",
                            "net.neoforged" => "NeoForge",
                            _ => "",
                        };
                        self.title = format!("Select {} Version", name);
                        std::thread::spawn(move || {
                            let res = if let Some(mv) = mc_version {
                                fetch_loader_versions_by_uid(&uid, &mv)
                            } else {
                                Ok(vec![])
                            };
                            let _ = sender_clone.send(ComponentEditorInput::LoadersLoaded(res));
                        });
                    }
                    _ => {
                        self.loading = false;
                        self.error = Some("Unsupported component for hotswapping".to_string());
                    }
                }
            }
            ComponentEditorInput::Close => {
                self.visible = false;
            }
            ComponentEditorInput::VersionsLoaded(res) => {
                self.loading = false;
                match res {
                    Ok(versions) => {
                        self.all_mc_versions = versions;
                        self.rebuild_list();
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            ComponentEditorInput::LoadersLoaded(res) => {
                self.loading = false;
                match res {
                    Ok(versions) => {
                        self.loader_versions = versions;
                        self.rebuild_list();
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            ComponentEditorInput::SearchChanged(text) => {
                self.search_text = text;
                self.rebuild_list();
            }
            ComponentEditorInput::ToggleSnapshots(active) => {
                self.show_snapshots = active;
                self.rebuild_list();
            }
            ComponentEditorInput::ToggleReleases(active) => {
                self.show_releases = active;
                self.rebuild_list();
            }
            ComponentEditorInput::SelectVersion(idx) => {
                let version = if self.uid == "net.minecraft" {
                    self.filtered_mc_versions.get(idx).map(|v| v.id.clone())
                } else {
                    self.loader_versions.get(idx).map(|v| v.version.clone())
                };

                if let Some(v) = version {
                    self.selected_version = Some(v.clone());
                    for i in 0..self.version_list.len() {
                        if let Some(row) = self.version_list.get(i) {
                            let is_sel = row.id == v;
                            self.version_list.send(i, is_sel);
                        }
                    }
                }
            }
            ComponentEditorInput::ConfirmInstall => {
                if let Some(v) = &self.selected_version {
                    sender
                        .output(ComponentEditorOutput::SetVersion(self.uid.clone(), v.clone()))
                        .ok();
                    self.visible = false;
                }
            }
        }
    }
}

impl ComponentEditorDialog {
    fn rebuild_list(&mut self) {
        let mut guard = self.version_list.guard();
        guard.clear();

        if self.uid == "net.minecraft" {
            let mut types = Vec::new();
            if self.show_releases {
                types.push(VersionType::Release);
            }
            if self.show_snapshots {
                types.push(VersionType::Snapshot);
            }

            let mut filtered = filter_versions(&self.all_mc_versions, &types);
            if !self.search_text.is_empty() {
                let query = self.search_text.to_lowercase();
                filtered.retain(|v| v.id.to_lowercase().contains(&query));
            }

            self.filtered_mc_versions = filtered;
            for (i, v) in self.filtered_mc_versions.iter().take(50).enumerate() {
                let is_selected = self.selected_version.as_ref() == Some(&v.id);
                let is_current = self.current_version.as_ref() == Some(&v.id);
                let is_newest = i == 0;
                guard.push_back((v.id.clone(), v.version_type.as_str().to_string(), is_selected, is_current, is_newest));
            }
        } else if self.uid == "net.fabricmc.fabric-loader"
            || self.uid == "org.quiltmc.quilt-loader"
            || self.uid == "net.minecraftforge"
            || self.uid == "net.neoforged"
        {
            let mut filtered = self.loader_versions.clone();
            if !self.search_text.is_empty() {
                let query = self.search_text.to_lowercase();
                filtered.retain(|v| v.version.to_lowercase().contains(&query));
            }

            for (i, v) in filtered.iter().take(50).enumerate() {
                let suffix = if v.stable { "Stable" } else { "Experimental" };
                let is_selected = self.selected_version.as_ref() == Some(&v.version);
                let is_current = self.current_version.as_ref() == Some(&v.version);
                let is_newest = i == 0;
                guard.push_back((v.version.clone(), suffix.to_string(), is_selected, is_current, is_newest));
            }
        }
    }
}
