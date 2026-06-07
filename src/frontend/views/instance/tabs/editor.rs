use crate::backend::instance::manager::Instance;
use crate::config::Config;
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug, Clone)]
pub struct ComponentRowData {
    pub name: String,
    pub version: String,
    pub uid: String,
}

#[derive(Debug)]
pub enum EditorTabOutput {
    OpenComponentSwap(String),
    RemoveComponent(String),
    SelectModLoaderRequest,
    SetInstanceJavaDefault,
    OpenJavaSelector,
    EditMods,
    ExploreMods,
    EditComponents,
    EditResourcePacks,
    ExploreResourcePacks,
    EditShaderPacks,
    ExploreShaderPacks,
    EditWorlds,
    OpenScreenshotsFolder,
}

#[derive(Debug)]
pub struct ComponentRow {
    pub data: ComponentRowData,
}

#[relm4::factory(pub)]
impl FactoryComponent for ComponentRow {
    type Init = ComponentRowData;
    type Input = ();
    type Output = EditorTabOutput;
    type CommandOutput = ();
    type ParentWidget = adw::ExpanderRow;

    view! {
        adw::ActionRow {
            #[watch]
            set_title: if self.data.uid == "add_mod_loader" { "Add Mod Loader" } else { &self.data.name },
            #[watch]
            set_subtitle: if self.data.uid == "add_mod_loader" { "Install Fabric, Forge, or Quilt" } else { &self.data.version },
            set_title_lines: 1,
            set_subtitle_lines: 1,
            #[watch]
            set_activatable: self.data.uid == "add_mod_loader",

            add_prefix = &gtk::Image {
                #[watch]
                set_icon_name: Some(if self.data.uid == "add_mod_loader" { "list-add-symbolic" } else { "" }),
                #[watch]
                set_visible: self.data.uid == "add_mod_loader",
            },

            add_suffix = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 6,
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: self.data.uid != "add_mod_loader",

                gtk::Button {
                    set_icon_name: "document-edit-symbolic",
                    set_tooltip_text: Some("Edit Version"),
                    set_css_classes: &["flat", "circular"],
                    #[watch]
                    set_visible: (self.data.uid == "net.minecraft" || crate::backend::instance::manager::is_loader_component(&self.data.uid)) && self.data.uid != "net.fabricmc.intermediary",
                    connect_clicked[sender, uid = self.data.uid.clone()] => move |_| {
                        sender.output(EditorTabOutput::OpenComponentSwap(uid.clone())).unwrap();
                    },
                },

                gtk::Button {
                    set_icon_name: "user-trash-symbolic",
                    set_tooltip_text: Some("Delete Modloader"),
                    set_css_classes: &["flat", "circular", "destructive-action"],
                    #[watch]
                    set_visible: crate::backend::instance::manager::is_loader_component(&self.data.uid) && self.data.uid != "net.fabricmc.intermediary",
                    connect_clicked[sender, uid = self.data.uid.clone()] => move |_| {
                        sender.output(EditorTabOutput::RemoveComponent(uid.clone())).unwrap();
                    },
                },
            },

            connect_activated[sender, uid = self.data.uid.clone()] => move |_| {
                if uid == "add_mod_loader" {
                    sender.output(EditorTabOutput::SelectModLoaderRequest).unwrap();
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { data: init }
    }
}

pub struct InstanceEditorTab {
    pub instance: Option<Instance>,
    pub config: Config,
    pub components_list: FactoryVecDeque<ComponentRow>,
}

#[derive(Debug)]
pub enum EditorTabInput {
    Update(Option<Instance>, Config),
}

#[relm4::component(pub)]
impl Component for InstanceEditorTab {
    type Init = (Option<Instance>, Config);
    type Input = EditorTabInput;
    type Output = EditorTabOutput;
    type CommandOutput = ();

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[wrap(Some)]
            set_child = &gtk::ListBox {
                set_selection_mode: gtk::SelectionMode::None,
                set_css_classes: &["boxed-list"],
                set_margin_all: 20,
                set_valign: gtk::Align::Start,

                #[local_ref]
                components_expander -> adw::ExpanderRow {
                    set_title: "Components",
                    add_prefix = &gtk::Image::from_icon_name("package-x-generic-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} components", model.instance.as_ref().map(|inst| inst.components.len()).unwrap_or(0) + 1),
                    set_expanded: true,

                    add_row = &adw::ActionRow {
                        set_title: "Java",
                        #[watch]
                        set_subtitle: &{
                            let is_custom = model.instance.as_ref().and_then(|inst| inst.java_path.as_ref()).is_some();
                            if is_custom {
                                model.instance.as_ref()
                                    .and_then(|inst| inst.java_path.as_ref())
                                    .map(|p| format!("Custom: {}", p.to_string_lossy()))
                                    .unwrap_or_default()
                            } else {
                                "Automatic".to_string()
                            }
                        },
                        set_subtitle_lines: 1,
                        set_margin_start: 12,
                        add_suffix = &gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,
                            set_valign: gtk::Align::Center,

                            gtk::Button {
                                set_icon_name: "view-refresh-symbolic",
                                set_tooltip_text: Some("Reset to Automatic"),
                                set_css_classes: &["flat", "circular"],
                                #[watch]
                                set_visible: model.instance.as_ref().and_then(|inst| inst.java_path.as_ref()).is_some(),
                                connect_clicked[sender] => move |_| {
                                    sender.output(EditorTabOutput::SetInstanceJavaDefault).unwrap();
                                }
                            },
                            gtk::Button {
                                set_icon_name: "document-edit-symbolic",
                                set_tooltip_text: Some("Select Custom Java"),
                                set_css_classes: &["flat", "circular"],
                                connect_clicked[sender] => move |_| {
                                    sender.output(EditorTabOutput::OpenJavaSelector).unwrap();
                                }
                            },
                        },
                        set_activatable: false,
                    }
                },

                adw::ActionRow {
                    set_title: "Mods",
                    add_prefix = &gtk::Image::from_icon_name("application-x-addon-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} mods installed", model.instance.as_ref().map(|inst| inst.mods.len()).unwrap_or(0)),
                    set_subtitle_lines: 1,
                    set_activatable: false,
                    add_suffix = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_valign: gtk::Align::Center,

                        gtk::Button {
                            set_icon_name: "web-browser-symbolic",
                            set_tooltip_text: Some("Explore Mods (Modrinth)"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::ExploreMods).unwrap();
                            },
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            set_tooltip_text: Some("Edit mods"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::EditMods).unwrap();
                            },
                        },
                    },
                },

                adw::ActionRow {
                    set_title: "Resource Packs",
                    add_prefix = &gtk::Image::from_icon_name("preferences-desktop-wallpaper-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} resource packs installed", model.instance.as_ref().map(|inst| inst.resource_packs.len()).unwrap_or(0)),
                    set_subtitle_lines: 1,
                    set_activatable: false,
                    add_suffix = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_valign: gtk::Align::Center,

                        gtk::Button {
                            set_icon_name: "web-browser-symbolic",
                            set_tooltip_text: Some("Explore Resource Packs (Modrinth)"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::ExploreResourcePacks).unwrap();
                            },
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            set_tooltip_text: Some("Edit resource packs"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::EditResourcePacks).unwrap();
                            },
                        },
                    },
                },

                adw::ActionRow {
                    set_title: "Shader Packs",
                    add_prefix = &gtk::Image::from_icon_name("video-display-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} shader packs installed", model.instance.as_ref().map(|inst| inst.shader_packs.len()).unwrap_or(0)),
                    set_subtitle_lines: 1,
                    set_activatable: false,
                    add_suffix = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 6,
                        set_valign: gtk::Align::Center,

                        gtk::Button {
                            set_icon_name: "web-browser-symbolic",
                            set_tooltip_text: Some("Explore Shader Packs (Modrinth)"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::ExploreShaderPacks).unwrap();
                            },
                        },
                        gtk::Button {
                            set_icon_name: "document-edit-symbolic",
                            set_tooltip_text: Some("Edit shader packs"),
                            set_css_classes: &["flat", "circular"],
                            connect_clicked[sender] => move |_| {
                                sender.output(EditorTabOutput::EditShaderPacks).unwrap();
                            },
                        },
                    },
                },

                adw::ActionRow {
                    set_title: "Worlds",
                    add_prefix = &gtk::Image::from_icon_name("view-list-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} worlds", model.instance.as_ref().map(|inst| inst.worlds.len()).unwrap_or(0)),
                    set_subtitle_lines: 1,
                    set_activatable: false,
                    add_suffix = &gtk::Button {
                        set_icon_name: "document-edit-symbolic",
                        set_tooltip_text: Some("Edit worlds"),
                        set_css_classes: &["flat", "circular"],
                        set_valign: gtk::Align::Center,
                        connect_clicked[sender] => move |_| {
                            sender.output(EditorTabOutput::EditWorlds).unwrap();
                        },
                    },
                },

                adw::ActionRow {
                    set_title: "Screenshots",
                    add_prefix = &gtk::Image::from_icon_name("camera-photo-symbolic"),
                    #[watch]
                    set_subtitle: &format!("{} screenshots", model.instance.as_ref().map(|inst| inst.screenshot_count).unwrap_or(0)),
                    set_subtitle_lines: 1,
                    set_activatable: false,
                    add_suffix = &gtk::Button {
                        set_icon_name: "folder-open-symbolic",
                        set_tooltip_text: Some("Open screenshots folder"),
                        set_css_classes: &["flat", "circular"],
                        set_valign: gtk::Align::Center,
                        connect_clicked[sender] => move |_| {
                            sender.output(EditorTabOutput::OpenScreenshotsFolder).unwrap();
                        },
                    },
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let components_expander = adw::ExpanderRow::builder()
            .title("Components")
            .expanded(true)
            .build();

        let mut model = InstanceEditorTab {
            instance: init.0.clone(),
            config: init.1,
            components_list: FactoryVecDeque::builder()
                .launch(components_expander.clone())
                .forward(sender.output_sender(), |msg| msg),
        };

        if let Some(inst) = &init.0 {
            let mut guard = model.components_list.guard();
            let mut has_loader = false;
            for comp in &inst.components {
                if crate::backend::instance::manager::is_loader_component(&comp.uid) {
                    has_loader = true;
                }
                guard.push_back(ComponentRowData {
                    name: comp.name.clone(),
                    version: comp.version.clone(),
                    uid: comp.uid.clone(),
                });
            }
            if !has_loader {
                guard.push_back(ComponentRowData {
                    name: "Add Mod Loader".to_string(),
                    version: "Install Fabric, Forge, or Quilt".to_string(),
                    uid: "add_mod_loader".to_string(),
                });
            }
        }

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            EditorTabInput::Update(inst, config) => {
                self.instance = inst.clone();
                self.config = config;

                let mut guard = self.components_list.guard();
                guard.clear();

                if let Some(inst) = inst {
                    let mut has_loader = false;
                    for comp in &inst.components {
                        if crate::backend::instance::manager::is_loader_component(&comp.uid) {
                            has_loader = true;
                        }
                        guard.push_back(ComponentRowData {
                            name: comp.name.clone(),
                            version: comp.version.clone(),
                            uid: comp.uid.clone(),
                        });
                    }
                    if !has_loader {
                        guard.push_back(ComponentRowData {
                            name: "Add Mod Loader".to_string(),
                            version: "Install Fabric, Forge, or Quilt".to_string(),
                            uid: "add_mod_loader".to_string(),
                        });
                    }
                }
            }
        }
    }
}
