use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

/// Result of the icon chooser dialog.
#[derive(Debug, Clone)]
pub enum IconChooserOutput {
    /// User wants to pick an image via the native file chooser.
    ChooseFromFile(usize),
    /// User wants to apply the configured default icon.
    UseDefault(usize),
    /// User selected a previously-used icon (carries the path).
    UseRecent(usize, PathBuf),
}

#[derive(Debug)]
pub enum IconChooserInput {
    /// Open the chooser for instance `idx`.
    /// Provide the global default-icon path (if set) and recent icon paths.
    Open(usize, Option<PathBuf>, Vec<PathBuf>),
    /// Close / hide the dialog.
    Close,
    /// Internal: user clicked "Choose from File…"
    PickFile,
    /// Internal: user clicked "Use Default"
    ApplyDefault,
    /// Internal: user clicked a recent icon thumbnail.
    SelectRecent(PathBuf),
}

pub struct IconChooserDialog {
    visible: bool,
    instance_idx: usize,
    default_icon: Option<PathBuf>,
    recents: Vec<PathBuf>,
    recents_box: gtk::FlowBox,
}

#[relm4::component(pub)]
impl SimpleComponent for IconChooserDialog {
    type Init = ();
    type Input = IconChooserInput;
    type Output = IconChooserOutput;

    view! {
        adw::Window {
            set_title: Some("Choose Instance Icon"),
            set_default_width: 400,
            set_default_height: -1,
            set_modal: true,
            set_resizable: false,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(IconChooserInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Choose Icon",
                        set_subtitle: "Select an icon source",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 0,

                    // ── Source Buttons ────────────────────────────────
                    gtk::ListBox {
                        set_css_classes: &["boxed-list"],
                        set_selection_mode: gtk::SelectionMode::None,
                        set_margin_all: 16,

                        adw::ActionRow {
                            set_title: "Choose from File…",
                            set_subtitle: "Pick an image from your computer",
                            set_activatable: true,
                            add_prefix = &gtk::Image::from_icon_name("document-open-symbolic"),
                            add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                            connect_activated => IconChooserInput::PickFile,
                        },

                        adw::ActionRow {
                            set_title: "Use Default Icon",
                            #[watch]
                            set_subtitle: &model.default_subtitle(),
                            #[watch]
                            set_sensitive: model.default_icon.is_some(),
                            set_activatable: true,
                            add_prefix = &gtk::Image::from_icon_name("starred-symbolic"),
                            connect_activated => IconChooserInput::ApplyDefault,
                        },
                    },

                    // ── Recent Icons ─────────────────────────────────
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_start: 16,
                        set_margin_end: 16,
                        set_margin_bottom: 16,
                        #[watch]
                        set_visible: !model.recents.is_empty(),

                        gtk::Label {
                            set_label: "Recent Icons",
                            set_halign: gtk::Align::Start,
                            set_margin_bottom: 8,
                            set_css_classes: &["heading"],
                        },

                        #[local_ref]
                        recents_box -> gtk::FlowBox {
                            set_css_classes: &["icon-chooser-recents"],
                            set_homogeneous: true,
                            set_row_spacing: 8,
                            set_column_spacing: 8,
                            set_min_children_per_line: 4,
                            set_max_children_per_line: 8,
                            set_selection_mode: gtk::SelectionMode::None,
                            set_margin_all: 12,
                        }
                    }
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let recents_box = gtk::FlowBox::new();

        let model = IconChooserDialog {
            visible: false,
            instance_idx: 0,
            default_icon: None,
            recents: Vec::new(),
            recents_box: recents_box.clone(),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            IconChooserInput::Open(idx, default_icon, recents) => {
                self.instance_idx = idx;
                self.default_icon = default_icon;
                self.recents = recents;
                self.rebuild_recents(&sender);
                self.visible = true;
            }
            IconChooserInput::Close => {
                self.visible = false;
            }
            IconChooserInput::PickFile => {
                self.visible = false;
                sender
                    .output(IconChooserOutput::ChooseFromFile(self.instance_idx))
                    .ok();
            }
            IconChooserInput::ApplyDefault => {
                self.visible = false;
                sender
                    .output(IconChooserOutput::UseDefault(self.instance_idx))
                    .ok();
            }
            IconChooserInput::SelectRecent(path) => {
                self.visible = false;
                sender
                    .output(IconChooserOutput::UseRecent(self.instance_idx, path))
                    .ok();
            }
        }
    }
}

impl IconChooserDialog {
    fn default_subtitle(&self) -> String {
        self.default_icon
            .as_ref()
            .map(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Custom icon")
                    .to_string()
            })
            .unwrap_or_else(|| "No default icon set — configure in Settings".to_string())
    }

    fn rebuild_recents(&self, sender: &ComponentSender<Self>) {
        while let Some(child) = self.recents_box.first_child() {
            self.recents_box.remove(&child);
        }

        for recent_path in &self.recents {
            let btn = gtk::Button::builder()
                .has_frame(false)
                .width_request(48)
                .height_request(48)
                .css_classes(vec!["icon-chooser-recent-btn"])
                .tooltip_text(&*recent_path.to_string_lossy())
                .build();

            let image = if recent_path.exists() {
                if let Ok(texture) = gtk::gdk::Texture::from_filename(recent_path) {
                    gtk::Image::builder()
                        .paintable(&texture)
                        .pixel_size(40)
                        .build()
                } else {
                    gtk::Image::builder()
                        .icon_name("image-missing-symbolic")
                        .pixel_size(40)
                        .build()
                }
            } else {
                gtk::Image::builder()
                    .icon_name("image-missing-symbolic")
                    .pixel_size(40)
                    .build()
            };

            btn.set_child(Some(&image));

            let p = recent_path.clone();
            let s = sender.input_sender().clone();
            btn.connect_clicked(move |_| {
                s.send(IconChooserInput::SelectRecent(p.clone())).ok();
            });

            self.recents_box.append(&btn);
        }
    }
}
