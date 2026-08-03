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
    target_group: Option<String>,
    available_groups: Vec<String>,
    selected_group_idx: u32,
    selected_version: Option<String>,
    selected_version_data: Option<MinecraftVersion>,
    error_message: Option<String>,
    current_step: usize,

    group_model: gtk::StringList,
    version_selector: Controller<VersionSelector>,

    // UI Widgets for manual updates
    name_entry: Option<adw::EntryRow>,
}

#[derive(Debug)]
pub enum AddInstanceInput {
    Open {
        target_group: Option<String>,
        available_groups: Vec<String>,
    },
    Close,
    SetName(String),
    SelectGroup(u32),
    SetStep(usize),
    NextStep,
    PrevStep,
    SelectVersion(String, Option<MinecraftVersion>),
    Create,
    UpdateInstancesPath(Option<PathBuf>),
}

#[derive(Debug)]
pub enum AddInstanceOutput {
    InstanceCreated(MinecraftVersion, PathBuf, Option<String>),
}

#[relm4::component(pub)]
impl Component for AddInstanceDialog {
    type Init = Option<PathBuf>;
    type Input = AddInstanceInput;
    type Output = AddInstanceOutput;
    type CommandOutput = ();

    view! {
        adw::Dialog {
            set_title: "New Instance",
            set_content_width: 500,
            set_content_height: 540,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToastOverlay {
                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "New Instance",
                            #[watch]
                            set_subtitle: match model.current_step {
                                0 => "Step 1 of 2: General & Version",
                                _ => "Step 2 of 2: Confirm Details",
                            },
                        },
                    },

                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,

                        gtk::Stack {
                            set_vexpand: true,
                            set_hexpand: true,
                            set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                            set_transition_duration: 250,

                            // Step 1: General Details + Version Selector
                            add_named[Some("general")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 12,
                                set_margin_top: 16,
                                set_margin_start: 16,
                                set_margin_end: 16,
                                set_margin_bottom: 4,

                                adw::PreferencesGroup {
                                    set_title: "Instance Details",
                                    #[name = "name_entry"]
                                    adw::EntryRow {
                                        set_title: "Instance Name",
                                        connect_changed[sender] => move |entry| {
                                            sender.input(AddInstanceInput::SetName(entry.text().to_string()));
                                        },
                                        connect_entry_activated[sender] => move |_| {
                                            sender.input(AddInstanceInput::NextStep);
                                        },
                                    },
                                    adw::ComboRow {
                                        set_title: "Group / Folder",
                                        #[watch]
                                        set_model: Some(&model.group_model),
                                        #[watch]
                                        set_selected: model.selected_group_idx,
                                        connect_selected_notify[sender] => move |combo| {
                                            sender.input(AddInstanceInput::SelectGroup(combo.selected()));
                                        },
                                    },
                                },

                                adw::PreferencesGroup {
                                    set_title: "Minecraft Version",

                                    #[local_ref]
                                    version_selector_widget -> gtk::Box {},
                                },
                            },

                            // Step 2: Confirm Details
                            add_named[Some("confirm")] = &gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_top: 16,
                                set_margin_start: 16,
                                set_margin_end: 16,
                                set_margin_bottom: 4,

                                adw::PreferencesPage {
                                    adw::PreferencesGroup {
                                        set_title: "Summary &amp; Review",
                                        adw::ActionRow {
                                            set_title: "Instance Name",
                                            #[watch]
                                            set_subtitle: if model.name.trim().is_empty() { "Not set" } else { model.name.trim() },
                                        },
                                        adw::ActionRow {
                                            set_title: "Group / Folder",
                                            #[watch]
                                            set_subtitle: model.target_group.as_deref().unwrap_or("None (No Group)"),
                                        },
                                        adw::ActionRow {
                                            set_title: "Minecraft Version",
                                            #[watch]
                                            set_subtitle: model.selected_version.as_deref().unwrap_or("Not selected"),
                                        },
                                    },
                                }
                            },

                            #[watch]
                            set_visible_child_name: match model.current_step {
                                0 => "general",
                                _ => "confirm",
                            },
                        }
                    },

                    add_bottom_bar = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 8,
                        set_margin_bottom: 16,
                        set_margin_start: 16,
                        set_margin_end: 16,

                        gtk::Label {
                            #[watch]
                            set_visible: model.error_message.is_some(),
                            #[watch]
                            set_label: model.error_message.as_deref().unwrap_or(""),
                            set_css_classes: &["error", "caption"],
                            set_wrap: true,
                            set_halign: gtk::Align::Start,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,

                            // Left action button: Cancel on step 0, Back on step 1
                            gtk::Button {
                                #[watch]
                                set_label: if model.current_step == 0 { "Cancel" } else { "Back" },
                                set_css_classes: &["pill"],
                                connect_clicked[sender] => move |_| {
                                    sender.input(AddInstanceInput::PrevStep);
                                }
                            },

                            gtk::Box { set_hexpand: true },

                            // Right action button: Next on step 0, Create Instance on step 1
                            gtk::Button {
                                set_label: "Next",
                                set_css_classes: &["suggested-action", "pill"],
                                #[watch]
                                set_visible: model.current_step == 0,
                                #[watch]
                                set_sensitive: !model.name.trim().is_empty() && model.selected_version.is_some(),
                                connect_clicked[sender] => move |_| {
                                    sender.input(AddInstanceInput::NextStep);
                                }
                            },

                            gtk::Button {
                                set_label: "Create Instance",
                                set_css_classes: &["suggested-action", "pill"],
                                #[watch]
                                set_visible: model.current_step == 1,
                                #[watch]
                                set_sensitive: !model.name.trim().is_empty()
                                    && model.selected_version.is_some()
                                    && model.instances_path.is_some(),
                                connect_clicked => AddInstanceInput::Create,
                            }
                        }
                    }
                }
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

        let group_model = gtk::StringList::new(&["None (No Group)"]);

        let model = AddInstanceDialog {
            visible: false,
            instances_path,
            name: String::new(),
            target_group: None,
            available_groups: Vec::new(),
            selected_group_idx: 0,
            selected_version: None,
            selected_version_data: None,
            error_message: None,
            current_step: 0,
            group_model,
            version_selector,
            name_entry: None,
        };

        let version_selector_widget = model.version_selector.widget();
        let widgets = view_output!();

        let mut model = model;
        model.name_entry = Some(widgets.name_entry.clone());

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            AddInstanceInput::Open { target_group, available_groups } => {
                self.visible = true;
                self.name.clear();
                self.target_group = target_group.clone();
                self.available_groups = available_groups;
                self.current_step = 0;

                // Rebuild group string list model
                let items: Vec<String> = std::iter::once("None (No Group)".to_string())
                    .chain(self.available_groups.iter().cloned())
                    .collect();
                let str_refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
                self.group_model.splice(0, self.group_model.n_items(), &str_refs);

                // Compute selected group index
                self.selected_group_idx = if let Some(ref tg) = target_group {
                    self.available_groups.iter().position(|g| g == tg).map(|i| (i + 1) as u32).unwrap_or(0)
                } else {
                    0
                };

                if let Some(entry) = &self.name_entry {
                    entry.set_text("");
                }
                self.selected_version = None;
                self.selected_version_data = None;
                self.error_message = None;

                self.version_selector.emit(VersionSelectorInput::Load {
                    uid: "net.minecraft".to_string(),
                    mc_version: None,
                    current_version: None,
                    selected_version: None,
                });
            }
            AddInstanceInput::Close => {
                self.visible = false;
                root.close();
            }
            AddInstanceInput::SetName(name) => {
                self.name = name;
                self.error_message = None;
            }
            AddInstanceInput::SelectGroup(idx) => {
                self.selected_group_idx = idx;
                if idx == 0 {
                    self.target_group = None;
                } else if let Some(group) = self.available_groups.get((idx - 1) as usize) {
                    self.target_group = Some(group.clone());
                }
            }
            AddInstanceInput::SetStep(step) => {
                self.current_step = step;
            }
            AddInstanceInput::NextStep => {
                if self.current_step == 0 && !self.name.trim().is_empty() && self.selected_version.is_some() {
                    self.current_step = 1;
                }
            }
            AddInstanceInput::PrevStep => {
                if self.current_step == 0 {
                    self.visible = false;
                    root.close();
                } else {
                    self.current_step = 0;
                }
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
                        root.close();
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
