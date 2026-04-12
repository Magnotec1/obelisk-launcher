use crate::backend::instance::manager::Instance;
use crate::config::Config;
use adw::prelude::*;
use relm4::prelude::*;

pub struct InstanceSettingsTab {
    pub instance: Option<Instance>,
    pub config: Config,
}

#[derive(Debug)]
pub enum SettingsTabInput {
    Update(Option<Instance>, Config),
}

#[derive(Debug)]
pub enum SettingsTabOutput {
    SetFeralGameMode(bool),
    SetDiscreteGpu(bool),
    SetZinkVulkan(bool),
}

#[relm4::component(pub)]
impl Component for InstanceSettingsTab {
    type Init = (Option<Instance>, Config);
    type Input = SettingsTabInput;
    type Output = SettingsTabOutput;
    type CommandOutput = ();

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 16,
                set_margin_all: 20,
                set_valign: gtk::Align::Start,

                adw::PreferencesGroup {
                    set_title: "Performance &amp; Graphics",
                    set_description: Some("System-level tweaks to improve game performance."),

                    adw::ActionRow {
                        set_title: "Feral GameMode",
                        set_subtitle: "Optimizes CPU scaling and I/O priority for better performance (requires gamemoded).",
                        set_subtitle_lines: 2,
                        add_prefix = &gtk::Image::from_icon_name("applications-games-symbolic"),
                        #[watch]
                        set_sensitive: model.instance.is_some(),

                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_active: model.instance.as_ref().map(|i| i.feral_gamemode).unwrap_or(false),
                            connect_state_set[sender] => move |_, state| {
                                sender.output(SettingsTabOutput::SetFeralGameMode(state)).unwrap();
                                gtk::glib::Propagation::Proceed
                            }
                        }
                    },

                    adw::ActionRow {
                        set_title: "Discrete GPU Offload",
                        set_subtitle: "Forces the game to run on the dedicated graphics card.",
                        set_subtitle_lines: 2,
                        add_prefix = &gtk::Image::from_icon_name("video-display-symbolic"),
                        #[watch]
                        set_sensitive: model.instance.is_some(),

                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_active: model.instance.as_ref().map(|i| i.discrete_gpu).unwrap_or(false),
                            connect_state_set[sender] => move |_, state| {
                                sender.output(SettingsTabOutput::SetDiscreteGpu(state)).unwrap();
                                gtk::glib::Propagation::Proceed
                            }
                        }
                    },

                    adw::ActionRow {
                        set_title: "Zink (Vulkan) Rendering",
                        set_subtitle: "Translates OpenGL to Vulkan. Can improve performance on some hardware.",
                        set_subtitle_lines: 2,
                        add_prefix = &gtk::Image::from_icon_name("preferences-desktop-display-symbolic"),
                        #[watch]
                        set_sensitive: model.instance.is_some(),

                        add_suffix = &gtk::Switch {
                            set_valign: gtk::Align::Center,
                            #[watch]
                            set_active: model.instance.as_ref().map(|i| i.zink_vulkan).unwrap_or(false),
                            connect_state_set[sender] => move |_, state| {
                                sender.output(SettingsTabOutput::SetZinkVulkan(state)).unwrap();
                                gtk::glib::Propagation::Proceed
                            }
                        }
                    }
                }
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InstanceSettingsTab {
            instance: init.0,
            config: init.1,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            SettingsTabInput::Update(inst, config) => {
                self.instance = inst;
                self.config = config;
            }
        }
    }
}
