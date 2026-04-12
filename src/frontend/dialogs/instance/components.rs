#![allow(unused_assignments)]
use crate::backend::runtime::versions::{
    fetch_fabric_versions_for_game, fetch_forge_versions_for_game, fetch_quilt_versions_for_game,
    fetch_versions, filter_versions, LoaderVersion, MinecraftVersion, VersionType,
};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug)]
pub struct VersionRow {
    id: String,
    version_type: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = (String, String);
    type Input = ();
    type Output = usize;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.id,
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
        }
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
}

#[derive(Debug)]
pub enum ComponentEditorInput {
    Open(String, Option<String>), // UID, Current Version (optional)
    Close,
    VersionsLoaded(Result<Vec<MinecraftVersion>, String>),
    LoadersLoaded(Result<Vec<LoaderVersion>, String>),
    SelectVersion(usize),
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
        adw::Window {
            #[watch]
            set_title: Some(&model.title),
            set_default_width: 450,
            set_default_height: 500,
            set_modal: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(ComponentEditorInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
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

                    gtk::Spinner {
                        #[watch]
                        set_visible: model.loading,
                        set_spinning: true,
                        set_halign: gtk::Align::Center,
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[watch]
                        set_visible: !model.loading,
                        #[local_ref]
                        version_list_box -> gtk::ListBox {
                            set_css_classes: &["boxed-list"],
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
        };

        let version_list_box = model.version_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ComponentEditorInput::Open(uid, mc_version) => {
                self.visible = true;
                self.uid = uid.clone();
                self.loading = true;
                self.error = None;
                self.search_text.clear();

                let sender_clone = sender.input_sender().clone();
                match uid.as_str() {
                    "net.minecraft" => {
                        self.title = "Select Minecraft Version".to_string();
                        std::thread::spawn(move || {
                            let res = fetch_versions();
                            let _ = sender_clone.send(ComponentEditorInput::VersionsLoaded(res));
                        });
                    }
                    "net.fabricmc.fabric-loader" => {
                        self.title = "Select Fabric Loader Version".to_string();
                        std::thread::spawn(move || {
                            let res = if let Some(mv) = mc_version {
                                fetch_fabric_versions_for_game(&mv)
                            } else {
                                Ok(vec![])
                            };
                            let _ = sender_clone.send(ComponentEditorInput::LoadersLoaded(res));
                        });
                    }
                    "org.quiltmc.quilt-loader" => {
                        self.title = "Select Quilt Loader Version".to_string();
                        std::thread::spawn(move || {
                            let res = if let Some(mv) = mc_version {
                                fetch_quilt_versions_for_game(&mv)
                            } else {
                                Ok(vec![])
                            };
                            let _ = sender_clone.send(ComponentEditorInput::LoadersLoaded(res));
                        });
                    }
                    "net.minecraftforge" => {
                        self.title = "Select Forge Version".to_string();
                        std::thread::spawn(move || {
                            let res = if let Some(mv) = mc_version {
                                fetch_forge_versions_for_game(&mv)
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
                    sender
                        .output(ComponentEditorOutput::SetVersion(self.uid.clone(), v))
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
            for v in self.filtered_mc_versions.iter().take(50) {
                guard.push_back((v.id.clone(), v.version_type.as_str().to_string()));
            }
        } else if self.uid == "net.fabricmc.fabric-loader"
            || self.uid == "org.quiltmc.quilt-loader"
            || self.uid == "net.minecraftforge"
        {
            let mut filtered = self.loader_versions.clone();
            if !self.search_text.is_empty() {
                let query = self.search_text.to_lowercase();
                filtered.retain(|v| v.version.to_lowercase().contains(&query));
            }

            for v in filtered.iter().take(50) {
                let suffix = if v.stable { "Stable" } else { "Experimental" };
                guard.push_back((v.version.clone(), suffix.to_string()));
            }
        }
    }
}
