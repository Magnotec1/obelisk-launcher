use adw::prelude::*;
use relm4::prelude::*;

pub struct InstanceSharerDialog {
    visible: bool,
    instance_index: Option<usize>,
    is_loading: bool,
    loading_title: String,
    loading_subtitle: String,
    progress: f64,
    show_progress: bool,
}

#[derive(Debug)]
pub enum SharerInput {
    Open(usize),
    Close,
    GenerateCode,
    SetLoading(bool, String, String, bool),
    SetProgress(f64),
    ExportToZip,
    ExportZipTo(std::path::PathBuf),
}

#[derive(Debug)]
pub enum SharerOutput {
    Generate(usize),
    ExportZip(usize, std::path::PathBuf),
}

#[relm4::component(pub)]
impl SimpleComponent for InstanceSharerDialog {
    type Init = ();
    type Input = SharerInput;
    type Output = SharerOutput;

    view! {
        adw::Dialog {
            set_title: "Share Instance",
            set_content_width: 450,
            set_content_height: 300,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                     set_title_widget = &adw::WindowTitle {
                        set_title: "Share Instance",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                    add_named[Some("modes")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 24,

                        gtk::Label {
                            set_label: "Choose how you would like to share this instance with others.",
                            set_wrap: true,
                            set_justify: gtk::Justification::Center,
                            set_css_classes: &["dim-label"],
                        },

                        gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_css_classes: &["boxed-list"],

                            adw::ActionRow {
                                set_title: "Generate Share Code",
                                set_subtitle: "Create a portable code that includes mods and metadata.",
                                set_activatable: true,
                                add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => SharerInput::GenerateCode,
                            },

                            adw::ActionRow {
                                set_title: "Export as Zip",
                                set_subtitle: "Create a compressed archive of the entire instance.",
                                set_activatable: true,
                                add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => SharerInput::ExportToZip,
                            },
                        }
                    },

                    add_named[Some("loading")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_valign: gtk::Align::Center,
                        set_margin_all: 32,

                        gtk::Label {
                            #[watch]
                            set_label: &model.loading_title,
                            set_css_classes: &["title-3"],
                        },
                        
                        gtk::Label {
                            #[watch]
                            set_label: &model.loading_subtitle,
                            set_css_classes: &["dim-label"],
                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                            set_lines: 1,
                            set_max_width_chars: 40,
                        },
                        
                        adw::Spinner {
                            #[watch]
                            set_visible: !model.show_progress,
                            set_width_request: 48,
                            set_height_request: 48,
                            set_halign: gtk::Align::Center,
                        },

                        gtk::ProgressBar {
                            #[watch]
                            set_visible: model.show_progress,
                            #[watch]
                            set_fraction: model.progress,
                            set_show_text: true,
                            set_width_request: 280,
                            set_halign: gtk::Align::Center,
                            set_margin_top: 8,
                            set_margin_bottom: 8,
                        },
                    },

                    // Must come after add_named so children exist on first render
                    #[watch]
                    set_visible_child_name: if model.is_loading { "loading" } else { "modes" },
                }
            }
        }
    }

    fn init(_init: Self::Init, _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = InstanceSharerDialog {
            visible: false,
            instance_index: None,
            is_loading: false,
            loading_title: String::new(),
            loading_subtitle: String::new(),
            progress: 0.0,
            show_progress: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SharerInput::Open(idx) => {
                self.instance_index = Some(idx);
                self.visible = true;
                self.is_loading = false;
                self.show_progress = false;
                self.progress = 0.0;
            }
            SharerInput::Close => {
                self.visible = false;
            }
            SharerInput::GenerateCode => {
                if let Some(idx) = self.instance_index {
                    self.is_loading = true;
                    self.loading_title = "Generating Code".to_string();
                    self.loading_subtitle = "Hashing your mods...".to_string();
                    self.progress = 0.0;
                    self.show_progress = false;
                    sender.output(SharerOutput::Generate(idx)).unwrap();
                }
            }
            SharerInput::SetLoading(loading, title, subtitle, show_progress) => {
                self.is_loading = loading;
                self.loading_title = title;
                self.loading_subtitle = subtitle;
                self.show_progress = show_progress;
                if !loading {
                    self.progress = 0.0;
                }
            }
            SharerInput::SetProgress(p) => {
                self.progress = p;
                self.show_progress = true;
            }
            SharerInput::ExportToZip => {
                if let Some(_idx) = self.instance_index {
                    let dialog = gtk::FileDialog::builder()
                        .title("Export Instance to Zip")
                        .initial_name("instance.zip")
                        .build();
                    
                    let sender = sender.clone();
                    dialog.save(relm4::main_application().active_window().as_ref(), gtk::gio::Cancellable::NONE, move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                sender.input(SharerInput::ExportZipTo(path));
                            }
                        }
                    });
                }
            }
            SharerInput::ExportZipTo(path) => {
                if let Some(idx) = self.instance_index {
                    self.is_loading = true;
                    self.loading_title = "Exporting Zip".to_string();
                    self.loading_subtitle = "Preparing files...".to_string();
                    self.progress = 0.0;
                    self.show_progress = true;
                    sender.output(SharerOutput::ExportZip(idx, path)).unwrap();
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportStep {
    Selection,
    CodeEntry,
    Progress,
}

pub struct ImportDialog {
    visible: bool,
    step: ImportStep,
    code: String,
    error: Option<String>,
    is_loading: bool,
    log_buffer: gtk::TextBuffer,
    code_entry: Option<gtk::Entry>,
}

#[derive(Debug)]
pub enum ImportInput {
    Open,
    Close,
    SetStep(ImportStep),
    SetCode(String),
    Confirm,
    AddLog(String),
    SetLoading(bool),
    ImportFromZip,
    ZipSelected(std::path::PathBuf),
}

#[derive(Debug)]
pub enum ImportOutput {
    Import(String),
    ImportZip(std::path::PathBuf),
}

#[relm4::component(pub)]
impl SimpleComponent for ImportDialog {
    type Init = ();
    type Input = ImportInput;
    type Output = ImportOutput;

    view! {
        adw::Dialog {
            set_title: "Import Instance",
            set_content_width: 550,
            set_content_height: 400,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                     set_title_widget = &adw::WindowTitle {
                        set_title: "Import Instance",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,

                    add_named[Some("selection")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 24,

                        gtk::Label {
                            set_label: "Choose how you would like to import a new instance.",
                            set_wrap: true,
                            set_justify: gtk::Justification::Center,
                            set_css_classes: &["dim-label"],
                        },

                        gtk::ListBox {
                            set_selection_mode: gtk::SelectionMode::None,
                            set_css_classes: &["boxed-list"],

                            adw::ActionRow {
                                set_title: "Import from Code",
                                set_subtitle: "Use a sharing code to download a pre-configured instance.",
                                set_activatable: true,
                                add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => ImportInput::SetStep(ImportStep::CodeEntry),
                            },

                            adw::ActionRow {
                                set_title: "Import from Zip",
                                set_subtitle: "Select a zip archive containing an exported instance.",
                                set_activatable: true,
                                add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                                connect_activated => ImportInput::ImportFromZip,
                            },
                        }
                    },

                    add_named[Some("entry")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 32,
                        set_valign: gtk::Align::Center,

                        gtk::Label {
                            set_label: "Paste the sharing code you received below to import the instance setup and automatically download mods.",
                            set_wrap: true,
                            set_justify: gtk::Justification::Center,
                            set_css_classes: &["dim-label"],
                        },

                        #[name = "code_entry"]
                        gtk::Entry {
                            set_placeholder_text: Some("Paste code here..."),
                            set_hexpand: true,
                            connect_changed[sender] => move |entry| {
                                sender.input(ImportInput::SetCode(entry.text().to_string()));
                            },
                        },

                        gtk::Label {
                            #[watch]
                            set_visible: model.error.is_some(),
                            #[watch]
                            set_label: model.error.as_deref().unwrap_or(""),
                            set_css_classes: &["error"],
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            set_halign: gtk::Align::Center,
                            set_margin_top: 12,

                            gtk::Button {
                                set_label: "Back",
                                set_css_classes: &["pill"],
                                connect_clicked => ImportInput::SetStep(ImportStep::Selection),
                            },

                            gtk::Button {
                                set_label: "Import Instance",
                                set_css_classes: &["suggested-action", "pill"],
                                #[watch]
                                set_sensitive: !model.code.is_empty(),
                                connect_clicked => ImportInput::Confirm,
                            },
                        }
                    },

                    add_named[Some("progress")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_margin_all: 24,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            adw::Spinner {
                                set_width_request: 24,
                                set_height_request: 24,
                            },
                            gtk::Label {
                                set_label: "Importing instance...",
                                set_css_classes: &["title-3"],
                                set_hexpand: true,
                                set_halign: gtk::Align::Start,
                            },
                        },

                        gtk::ScrolledWindow {
                            set_vexpand: true,
                            set_css_classes: &["card"],
                            set_has_frame: true,
                            #[wrap(Some)]
                            set_child = &gtk::TextView {
                                set_editable: false,
                                set_cursor_visible: false,
                                set_wrap_mode: gtk::WrapMode::WordChar,
                                set_buffer: Some(&model.log_buffer),
                                set_css_classes: &["caption"],
                                set_margin_all: 8,
                            }
                        },

                        gtk::ProgressBar {
                            set_css_classes: &["pill"],
                            #[watch]
                            set_fraction: 0.5,
                        }
                    },

                    // Must come after add_named so children exist on first render
                    #[watch]
                    set_visible_child_name: match model.step {
                        ImportStep::Selection => "selection",
                        ImportStep::CodeEntry => "entry",
                        ImportStep::Progress => "progress",
                    },
                }
            }
        }
    }

    fn init(_init: Self::Init, _root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = ImportDialog {
            visible: false,
            step: ImportStep::Selection,
            code: String::new(),
            error: None,
            is_loading: false,
            log_buffer: gtk::TextBuffer::new(None),
            code_entry: None,
        };
        let widgets = view_output!();
        let mut model = model;
        model.code_entry = Some(widgets.code_entry.clone());
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ImportInput::Open => {
                self.visible = true;
                self.step = ImportStep::Selection;
                self.code.clear();
                if let Some(entry) = &self.code_entry {
                    entry.set_text("");
                }
                self.error = None;
                self.is_loading = false;
                self.log_buffer.set_text("");
            }
            ImportInput::Close => {
                self.visible = false;
            }
            ImportInput::SetStep(step) => {
                self.step = step;
                if step == ImportStep::Progress {
                    self.is_loading = true;
                } else {
                    self.is_loading = false;
                }
            }
            ImportInput::SetCode(code) => {
                self.code = code;
                self.error = None;
            }
            ImportInput::Confirm => {
                if !self.code.trim().is_empty() {
                    self.step = ImportStep::Progress;
                    self.is_loading = true;
                    self.log_buffer.set_text("Initializing import...\n");
                    sender.output(ImportOutput::Import(self.code.trim().to_string())).unwrap();
                }
            }
            ImportInput::AddLog(status) => {
                let mut end_iter = self.log_buffer.end_iter();
                self.log_buffer.insert(&mut end_iter, &format!("{}\n", status));
            }
            ImportInput::SetLoading(loading) => {
                self.is_loading = loading;
            }
            ImportInput::ImportFromZip => {
                let dialog = gtk::FileDialog::builder()
                    .title("Import Instance from Zip")
                    .build();
                
                let filter = gtk::FileFilter::new();
                filter.add_suffix("zip");
                filter.set_name(Some("Zip files"));
                let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                dialog.set_filters(Some(&filters));

                let sender = sender.clone();
                dialog.open(relm4::main_application().active_window().as_ref(), gtk::gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res {
                        if let Some(path) = file.path() {
                            sender.input(ImportInput::ZipSelected(path));
                        }
                    }
                });
            }
            ImportInput::ZipSelected(path) => {
                self.step = ImportStep::Progress;
                self.is_loading = true;
                self.log_buffer.set_text("Opening zip archive...\n");
                sender.output(ImportOutput::ImportZip(path)).unwrap();
            }
        }
    }
}
