use crate::backend::instance::manager::Instance;
use crate::frontend::app::InstanceStatus;
use adw::prelude::*;
use relm4::prelude::*;

trait ImageExt {
    fn set_custom_icon(&self, texture: Option<gtk::gdk::Texture>);
}

impl ImageExt for gtk::Image {
    fn set_custom_icon(&self, texture: Option<gtk::gdk::Texture>) {
        if let Some(tex) = texture {
            self.set_paintable(Some(&tex));
        } else {
            self.set_icon_name(Some("application-x-executable-symbolic"));
        }
    }
}

pub struct InstanceSummary {
    pub instance: Option<Instance>,
    pub status: InstanceStatus,
    pub is_narrow: bool,
    pub sharing_loading: bool,
    pub verifying_loading: bool,
}

#[derive(Debug)]
pub enum SummaryInput {
    Update(Option<Instance>, InstanceStatus),
    SetNarrow(bool),
    SetSharingLoading(bool),
    SetVerifyingLoading(bool),
}

#[derive(Debug)]
pub enum SummaryOutput {
    Launch,
    Verify,
    Kill,
    OpenFolder,
    SwitchToConsole,
    Share,
}

#[relm4::component(pub)]
impl SimpleComponent for InstanceSummary {
    type Init = (Option<Instance>, InstanceStatus);
    type Input = SummaryInput;
    type Output = SummaryOutput;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[wrap(Some)]
            set_child = &adw::Clamp {
                set_maximum_size: 1024,
                set_tightening_threshold: 400,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_valign: gtk::Align::Start,
                    #[watch]
                    set_margin_all: if model.is_narrow { 16 } else { 32 },
                    #[watch]
                    set_spacing: if model.is_narrow { 20 } else { 32 },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        #[watch]
                        set_spacing: if model.is_narrow { 16 } else { 24 },

                        gtk::Image {
                            #[watch]
                            set_pixel_size: if model.is_narrow { 64 } else { 96 },
                            #[watch]
                            set_custom_icon: InstanceSummary::get_icon_texture(&model.instance),
                            #[watch]
                            set_visible: true,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_valign: gtk::Align::Center,
                            set_spacing: 4,
                            set_hexpand: true,

                            gtk::Label {
                                #[watch]
                                set_label: model.instance.as_ref().map(|inst| inst.name.as_str()).unwrap_or(""),
                                set_css_classes: &["title-1"],
                                set_halign: gtk::Align::Start,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_xalign: 0.0,
                            },
                            gtk::Label {
                                #[watch]
                                set_label: &format!("{} hours played", model.instance.as_ref().map(|inst| inst.total_time_played).unwrap_or(0) / 3600),
                                #[watch]
                                set_tooltip_text: Some(&model.format_playtime()),
                                set_css_classes: &["dim-label"],
                                set_halign: gtk::Align::Start,
                                set_ellipsize: gtk::pango::EllipsizeMode::End,
                                set_xalign: 0.0,
                            },
                        }
                    },

                    gtk::ListBox {
                        set_selection_mode: gtk::SelectionMode::None,
                        set_css_classes: &["boxed-list"],

                        adw::ActionRow {
                            set_title: "Version",
                            #[watch]
                            set_subtitle: model.instance.as_ref().and_then(|inst| inst.minecraft_version.as_ref()).map(|s| s.as_str()).unwrap_or("Unknown"),
                            add_prefix = &gtk::Image::from_icon_name("document-properties-symbolic"),
                        },
                        adw::ActionRow {
                            set_title: "Mod Loader",
                            #[watch]
                            set_subtitle: model.instance.as_ref().and_then(|inst| inst.mod_loader.as_ref()).map(|s| s.as_str()).unwrap_or("Vanilla"),
                            add_prefix = &gtk::Image::from_icon_name("applications-engineering-symbolic"),
                        },
                        adw::ActionRow {
                            set_title: "Path",
                            #[watch]
                            set_subtitle: model.instance.as_ref().map(|inst| inst.path.to_string_lossy()).unwrap_or_default().as_ref(),
                            add_prefix = &gtk::Image::from_icon_name("folder-symbolic"),
                            set_activatable: true,
                            connect_activated[sender] => move |_| {
                                sender.output(SummaryOutput::OpenFolder).unwrap();
                            },
                        },
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 8,
                        set_halign: gtk::Align::Center,

                        gtk::Button {
                            #[watch]
                            set_visible: model.status == InstanceStatus::NotRunning,
                            set_tooltip_text: Some("Launch Game"),
                            set_css_classes: &["suggested-action", "pill"],
                            #[wrap(Some)]
                            set_child = &adw::ButtonContent {
                                set_icon_name: "media-playback-start-symbolic",
                                #[watch]
                                set_label: if model.is_narrow { "" } else { "Launch" },
                            },
                            connect_clicked[sender] => move |_| {
                                sender.output(SummaryOutput::Launch).unwrap();
                            },
                        },

                        gtk::Button {
                            #[watch]
                            set_visible: model.status == InstanceStatus::Loading,
                            set_css_classes: &["pill"],
                            set_sensitive: false,
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 6,
                                adw::Spinner {
                                    set_width_request: 16,
                                    set_height_request: 16,
                                },
                                gtk::Label {
                                    #[watch]
                                    set_label: if model.is_narrow { "" } else { "Loading…" },
                                }
                            }
                        },

                        gtk::Button {
                            #[watch]
                            set_visible: model.status == InstanceStatus::Running,
                            set_tooltip_text: Some("Stop Game"),
                            set_css_classes: &["destructive-action", "pill"],
                            #[wrap(Some)]
                            set_child = &adw::ButtonContent {
                                set_icon_name: "media-playback-stop-symbolic",
                                #[watch]
                                set_label: if model.is_narrow { "" } else { "Stop" },
                            },
                            connect_clicked[sender] => move |_| {
                                sender.output(SummaryOutput::Kill).unwrap();
                            },
                        },

                        gtk::Button {
                            set_icon_name: "view-refresh-symbolic",
                            set_tooltip_text: Some("Verify Instance"),
                            set_css_classes: &["circular"],
                            #[watch]
                            set_sensitive: !model.verifying_loading && model.status == InstanceStatus::NotRunning && model.instance.is_some(),
                            connect_clicked[sender] => move |_| {
                                sender.output(SummaryOutput::Verify).unwrap();
                            },
                        },

                        gtk::Button {
                            set_icon_name: "preferences-system-sharing-symbolic",
                            set_tooltip_text: Some("Share Instance"),
                            set_css_classes: &["circular"],
                            #[watch]
                            set_sensitive: !model.sharing_loading && model.status == InstanceStatus::NotRunning && model.instance.is_some(),
                            connect_clicked[sender] => move |_| {
                                sender.output(SummaryOutput::Share).unwrap();
                            },
                        },
                    }
                },

            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = InstanceSummary {
            instance: init.0,
            status: init.1,
            is_narrow: false,
            sharing_loading: false,
            verifying_loading: false,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            SummaryInput::Update(inst, status) => {
                self.instance = inst;
                self.status = status;
            }
            SummaryInput::SetNarrow(narrow) => {
                self.is_narrow = narrow;
            }
            SummaryInput::SetSharingLoading(loading) => {
                self.sharing_loading = loading;
            }
            SummaryInput::SetVerifyingLoading(loading) => {
                self.verifying_loading = loading;
            }
        }
    }
}

impl InstanceSummary {
    fn format_playtime(&self) -> String {
        let seconds = self
            .instance
            .as_ref()
            .map(|inst| inst.total_time_played)
            .unwrap_or(0);
        let minutes = (seconds as f32) / 60.0;
        let hours = minutes / 60.0;
        let days = hours / 24.0;

        format!("{:.0} minutes\n{:.1} days", minutes, days)
    }

    fn get_icon_texture(instance: &Option<Instance>) -> Option<gtk::gdk::Texture> {
        instance.as_ref().and_then(|inst| {
            let p1 = inst.path.join("icon.png");
            if p1.exists() {
                gtk::gdk::Texture::from_filename(&p1).ok()
            } else {
                let p2 = inst.minecraft_dir.join("icon.png");
                if p2.exists() {
                    gtk::gdk::Texture::from_filename(&p2).ok()
                } else {
                    None
                }
            }
        })
    }
}
