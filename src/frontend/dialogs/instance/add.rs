#![allow(unused_assignments)]
use crate::backend::instance::manager::{create_instance, CreateInstanceOptions, ModLoader};
use crate::backend::runtime::versions::MinecraftVersion;
use crate::frontend::utils::{VersionSelector, VersionSelectorInput, VersionSelectorOutput};
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

pub struct AddInstanceDialog {
    visible: bool,
    instances_path: Option<PathBuf>,
    name: String,
    selected_version: Option<String>,
    selected_version_data: Option<MinecraftVersion>,
    error_message: Option<String>,
    target_group: Option<String>,

    version_selector: Controller<VersionSelector>,

    // UI Widgets for manual updates
    name_entry: Option<adw::EntryRow>,
}

#[derive(Debug)]
pub enum AddInstanceInput {
    Open(Option<String>),
    Close,
    SetName(String),
    SelectVersion(String, Option<MinecraftVersion>),
    Create,
    UpdateInstancesPath(Option<PathBuf>),
}

#[derive(Debug)]
pub enum AddInstanceOutput {
    InstanceCreated(MinecraftVersion, PathBuf, Option<String>),
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

                            #[local_ref]
                            version_selector_widget -> gtk::Box {},
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
        let version_selector = VersionSelector::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                VersionSelectorOutput::VersionSelected { version, mc_version } => {
                    AddInstanceInput::SelectVersion(version, mc_version)
                }
            });

        let model = AddInstanceDialog {
            visible: false,
            instances_path,
            name: String::new(),
            selected_version: None,
            selected_version_data: None,
            error_message: None,
            target_group: None,
            version_selector,
            name_entry: None,
        };

        let version_selector_widget = model.version_selector.widget();
        let widgets = view_output!();

        let mut model = model;
        model.name_entry = Some(widgets.name_entry.clone());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AddInstanceInput::Open(group) => {
                self.visible = true;
                self.name.clear();
                self.target_group = group;
                if let Some(entry) = &self.name_entry {
                    entry.set_text("");
                }
                self.selected_version = None;
                self.selected_version_data = None;

                self.version_selector.emit(VersionSelectorInput::Load {
                    uid: "net.minecraft".to_string(),
                    mc_version: None,
                    current_version: None,
                    selected_version: None,
                });
            }
            AddInstanceInput::Close => {
                self.visible = false;
            }
            AddInstanceInput::SetName(name) => {
                self.name = name;
                self.error_message = None;
            }
            AddInstanceInput::SelectVersion(version, mc_version) => {
                self.selected_version = Some(version);
                self.selected_version_data = mc_version;
                self.error_message = None;
            }
            AddInstanceInput::UpdateInstancesPath(path) => {
                self.instances_path = path;
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

                match create_instance(instances_path, options) {
                    Ok(path) => {
                        self.visible = false;
                        self.error_message = None;
                        if let Some(v) = self.selected_version_data.clone() {
                            sender
                                .output(AddInstanceOutput::InstanceCreated(v, path, self.target_group.clone()))
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
