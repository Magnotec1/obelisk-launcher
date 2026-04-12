use crate::backend::download::java::JavaDownloadManager;
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct VersionRow {
    version: u32,
    name: String,
    description: String,
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = u32;
    type Input = ();
    type Output = u32;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.name,
            set_subtitle: &self.description,
            set_activatable: true,
            add_suffix = &gtk::Label {
                set_label: if [8, 11, 16, 17, 21, 26].contains(&self.version) { "⭐" } else { "" },
                set_css_classes: &["dim-label", "caption"],
                set_valign: gtk::Align::Center,
            },
            connect_activated[sender, version = self.version] => move |_| {
                sender.output(version).ok();
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        let (name, description) = match init {
            26 => ("Java 26".to_string(), "Latest".to_string()),
            21 => ("Java 21".to_string(), "Required for 1.20.5+".to_string()),
            17 => ("Java 17".to_string(), "Required for 1.18 to 1.20.4".to_string()),
            16 => ("Java 16".to_string(), "Specifically for 1.17".to_string()),
            11 => ("Java 11".to_string(), "Common for Modpacks".to_string()),
            8 => ("Java 8".to_string(), "Required for 1.16.5 and older".to_string()),
            v => (format!("Java {}", v), String::new()),
        };

        Self {
            version: init,
            name,
            description,
        }
    }
}

pub struct InstallJavaDialog {
    visible: bool,
    installing: bool,
    status: String,
    target_dir: PathBuf,
    selected_version: Option<u32>,
    progress: f32,
    version_factory: FactoryVecDeque<VersionRow>,
    loading_versions: bool,
    cancel_flag: Option<Arc<AtomicBool>>,
}

#[derive(Debug)]
pub enum InstallJavaInput {
    Open,
    Close,
    SelectVersion(u32),
    Cancel,
    Progress(crate::backend::download::java::JavaDownloadProgress),
    VersionsLoaded(Result<Vec<u32>, String>),
}

#[derive(Debug)]
pub enum InstallJavaOutput {
    Finished,
}

#[relm4::component(pub)]
impl SimpleComponent for InstallJavaDialog {
    type Init = PathBuf;
    type Input = InstallJavaInput;
    type Output = InstallJavaOutput;

    view! {
        adw::Window {
            set_title: Some("Runtime Installation"),
            set_default_width: 500,
            set_default_height: 480,
            set_modal: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(InstallJavaInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Java Installer",
                        set_subtitle: "Managed Runtime Environment",
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 0,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_vexpand: true,
                        #[watch]
                        set_visible: !model.installing && model.status.is_empty(),

                        gtk::Label {
                            set_label: "Available Runtimes",
                            set_halign: gtk::Align::Start,
                            set_css_classes: &["title-4"],
                            set_margin_top: 24,
                            set_margin_start: 32,
                            set_margin_bottom: 8,
                        },

                        gtk::Label {
                            set_label: "Choose a Java version to install locally.",
                            set_halign: gtk::Align::Start,
                            set_css_classes: &["dim-label"],
                            set_margin_start: 32,
                            set_margin_bottom: 16,
                        },

                        gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vexpand: true,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_bottom: 16,
                                set_margin_top: 16,
                                set_margin_start: 24,
                                set_margin_end: 24,
                                set_spacing: 12,

                                #[local_ref]
                                version_list_box -> gtk::ListBox {
                                    set_css_classes: &["boxed-list"],
                                    set_selection_mode: gtk::SelectionMode::None,
                                },

                                gtk::Spinner {
                                    #[watch]
                                    set_visible: model.loading_versions,
                                    set_spinning: true,
                                    set_halign: gtk::Align::Center,
                                    set_margin_all: 12,
                                },
                            },
                        },
                    },

                    // Loading/Status section
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 24,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,
                        #[watch]
                        set_visible: model.installing || !model.status.is_empty(),

                        adw::Spinner {
                            #[watch]
                            set_visible: model.installing && model.progress < 0.0,
                            set_width_request: 32,
                            set_height_request: 32,
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &model.status,
                            set_css_classes: &["title-4"],
                            set_wrap: true,
                            set_max_width_chars: 40,
                            set_justify: gtk::Justification::Center,
                        },

                        gtk::ProgressBar {
                            #[watch]
                            set_fraction: model.progress as f64,
                            #[watch]
                            set_visible: model.installing && model.progress >= 0.0,
                            set_width_request: 300,
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            set_halign: gtk::Align::Center,

                            gtk::Button {
                                set_label: "Cancel",
                                #[watch]
                                set_visible: model.installing,
                                set_css_classes: &["destructive-action", "pill"],
                                connect_clicked => InstallJavaInput::Cancel,
                            },

                            gtk::Button {
                                set_label: "Close",
                                #[watch]
                                set_visible: !model.installing && !model.status.is_empty(),
                                set_css_classes: &["pill"],
                                connect_clicked => InstallJavaInput::Close,
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
        let version_factory = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), InstallJavaInput::SelectVersion);

        let model = InstallJavaDialog {
            visible: false,
            installing: false,
            status: String::new(),
            target_dir: init,
            selected_version: None,
            progress: -1.0,
            version_factory,
            loading_versions: false,
            cancel_flag: None,
        };

        let version_list_box = model.version_factory.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            InstallJavaInput::Open => {
                self.visible = true;
                self.installing = false;
                self.status = String::new();
                self.progress = -1.0;

                if self.version_factory.is_empty() {
                    self.loading_versions = true;
                    let sender_clone = sender.input_sender().clone();
                    std::thread::spawn(move || {
                        let result = JavaDownloadManager::get_available_versions();
                        let _ = sender_clone.send(InstallJavaInput::VersionsLoaded(result));
                    });
                }
            }
            InstallJavaInput::VersionsLoaded(result) => {
                self.loading_versions = false;
                if let Ok(versions) = result {
                    let mut guard = self.version_factory.guard();
                    guard.clear();
                    for v in versions {
                        guard.push_back(v);
                    }
                }
            }
            InstallJavaInput::Close => {
                self.visible = false;
            }
            InstallJavaInput::SelectVersion(v) => {
                self.selected_version = Some(v);
                self.installing = true;
                self.progress = -1.0;
                self.status = format!("Starting Java {} installation...", v);

                let cancel_flag = Arc::new(AtomicBool::new(false));
                self.cancel_flag = Some(cancel_flag.clone());

                let sender_clone = sender.input_sender().clone();
                let target_dir = self.target_dir.clone();
                
                std::thread::spawn(move || {
                    JavaDownloadManager::download_and_extract_with_progress(v, &target_dir, cancel_flag, move |progress| {
                        let _ = sender_clone.send(InstallJavaInput::Progress(progress));
                    });
                });
            }
            InstallJavaInput::Cancel => {
                if let Some(flag) = self.cancel_flag.take() {
                    flag.store(true, Ordering::Relaxed);
                }
                self.installing = false;
                self.status = "Installation cancelled.".to_string();
                self.progress = -1.0;
            }
            InstallJavaInput::Progress(progress) => {
                use crate::backend::download::java::JavaDownloadProgress;
                match progress {
                    JavaDownloadProgress::Downloading { current, total } => {
                        self.progress = if total > 0 { current as f32 / total as f32 } else { 0.0 };
                        self.status = format!("Downloading... ({:.1}%)", self.progress * 100.0);
                    }
                    JavaDownloadProgress::Extracting => {
                        self.status = "Extracting runtime...".to_string();
                        self.progress = 1.0;
                    }
                    JavaDownloadProgress::Finished(_) => {
                        self.installing = false;
                        self.visible = false;
                        self.cancel_flag = None;
                        sender.output(InstallJavaOutput::Finished).ok();
                    }
                    JavaDownloadProgress::Error(e) => {
                        self.installing = false;
                        self.status = format!("Installation failed: {}", e);
                        self.cancel_flag = None;
                    }
                }
            }
        }
    }
}
