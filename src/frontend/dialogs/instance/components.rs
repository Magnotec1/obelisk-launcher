#![allow(unused_assignments)]
use crate::frontend::utils::{VersionSelector, VersionSelectorInput, VersionSelectorOutput};
use adw::prelude::*;
use relm4::prelude::*;

pub struct ComponentEditorDialog {
    visible: bool,
    uid: String,
    title: String,
    selected_version: Option<String>,
    current_version: Option<String>,
    version_selector: Controller<VersionSelector>,
}

#[derive(Debug)]
pub enum ComponentEditorInput {
    Open(String, Option<String>, Option<String>), // UID, MC version, Current Version
    Close,
    SelectVersion(String),
    ConfirmInstall,
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

                    #[local_ref]
                    version_selector_widget -> gtk::Box {
                        set_vexpand: true,
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
                            set_sensitive: model.selected_version.is_some() && model.selected_version != model.current_version,
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
        let version_selector = VersionSelector::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                VersionSelectorOutput::VersionSelected { version, .. } => {
                    ComponentEditorInput::SelectVersion(version)
                }
            });

        let model = ComponentEditorDialog {
            visible: false,
            uid: String::new(),
            title: "Select Version".to_string(),
            selected_version: None,
            current_version: None,
            version_selector,
        };

        let version_selector_widget = model.version_selector.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ComponentEditorInput::Open(uid, mc_version, current_ver) => {
                self.visible = true;
                self.uid = uid.clone();
                self.current_version = current_ver.clone();
                self.selected_version = current_ver.clone();

                match uid.as_str() {
                    "net.minecraft" => {
                        self.title = "Select Minecraft Version".to_string();
                    }
                    "net.fabricmc.fabric-loader"
                    | "org.quiltmc.quilt-loader"
                    | "net.minecraftforge"
                    | "net.neoforged" => {
                        let name = match uid.as_str() {
                            "net.fabricmc.fabric-loader" => "Fabric Loader",
                            "org.quiltmc.quilt-loader" => "Quilt Loader",
                            "net.minecraftforge" => "Forge",
                            "net.neoforged" => "NeoForge",
                            _ => "",
                        };
                        self.title = format!("Select {} Version", name);
                    }
                    _ => {
                        self.title = "Select Version".to_string();
                    }
                }

                self.version_selector.emit(VersionSelectorInput::Load {
                    uid,
                    mc_version,
                    current_version: current_ver.clone(),
                    selected_version: current_ver,
                });
            }
            ComponentEditorInput::Close => {
                self.visible = false;
            }
            ComponentEditorInput::SelectVersion(v) => {
                self.selected_version = Some(v);
            }
            ComponentEditorInput::ConfirmInstall => {
                if let Some(v) = &self.selected_version {
                    sender
                        .output(ComponentEditorOutput::SetVersion(
                            self.uid.clone(),
                            v.clone(),
                        ))
                        .ok();
                    self.visible = false;
                }
            }
        }
    }
}
