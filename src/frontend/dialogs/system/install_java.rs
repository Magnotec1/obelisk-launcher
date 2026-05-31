use crate::backend::download::manager::{fetch_java_packages, JavaPackage};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub struct VersionRow {
    package: JavaPackage,
    selected: bool,
}

#[derive(Debug)]
pub enum VersionRowInput {
    SetSelected(bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for VersionRow {
    type Init = JavaPackage;
    type Input = VersionRowInput;
    type Output = String; // Return the package ID
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &format!("{} {}", self.package.distribution, self.package.java_version),
            set_subtitle: &format!("Java {} • {}", self.package.major_version, self.package.architecture),
            set_activatable: true,
            add_suffix = &gtk::Image {
                set_icon_name: Some("object-select-symbolic"),
                #[watch]
                set_visible: self.selected,
            },
            connect_activated[sender, id = self.package.id.clone()] => move |_| {
                sender.output(id.clone()).ok();
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            package: init,
            selected: false,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            VersionRowInput::SetSelected(selected) => {
                self.selected = selected;
            }
        }
    }
}

pub struct InstallJavaDialog {
    visible: bool,
    installing: bool,
    finished: bool,
    has_error: bool,
    status: String,
    target_dir: PathBuf,
    selected_package: Option<JavaPackage>,
    all_packages: Vec<JavaPackage>,
    search_query: String,
    filter_distro: Option<String>,
    progress: f32,
    version_factory: FactoryVecDeque<VersionRow>,
    loading_versions: bool,
    cancel_flag: Option<Arc<AtomicBool>>,
}

#[derive(Debug)]
pub enum InstallJavaInput {
    Open,
    Close,
    Refresh,
    Search(String),
    FilterDistro(Option<String>),
    SelectPackage(String),
    Install,
    Cancel,
    Progress(crate::backend::download::sources::java::JavaDownloadProgress),
    VersionsLoaded(Result<Vec<JavaPackage>, String>),
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
        #[name = "dialog"]
        adw::Dialog {
            set_title: "Runtime Installation",
            set_content_width: 550,
            set_content_height: 600,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Java Installer",
                    },
                    pack_end = &gtk::Button {
                        set_icon_name: "view-refresh-symbolic",
                        connect_clicked => InstallJavaInput::Refresh,
                        #[watch]
                        set_visible: !model.installing && !model.finished,
                        #[watch]
                        set_sensitive: !model.loading_versions,
                    }
                },
                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_halign: gtk::Align::Center,
                    #[watch]
                    set_visible: !model.installing,

                    gtk::Button {
                        set_label: "Close",
                        set_css_classes: &["pill"],
                        connect_clicked[root, sender] => move |_| {
                            sender.input(InstallJavaInput::Close);
                            root.close();
                        },
                    },
                    gtk::Button {
                        set_label: "Install",
                        #[watch]
                        set_visible: !model.finished && !model.has_error,
                        #[watch]
                        set_sensitive: model.selected_package.is_some(),
                        set_css_classes: &["suggested-action", "pill"],
                        connect_clicked => InstallJavaInput::Install,
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_margin_bottom: 16,

                    add_named[Some("select")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_vexpand: true,
                        set_spacing: 12,
                        #[watch]
                        set_visible: !model.installing && model.status.is_empty(),

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 6,
                            set_margin_top: 12,

                            gtk::SearchEntry {
                                set_hexpand: true,
                                set_placeholder_text: Some("Search versions or distributions..."),
                                connect_search_changed[sender] => move |entry| {
                                    sender.input(InstallJavaInput::Search(entry.text().to_string()));
                                }
                            },

                            gtk::DropDown {
                                set_model: Some(&gtk::StringList::new(&[
                                    "All Distributions",
                                    "Temurin",
                                    "Zulu",
                                    "Corretto",
                                    "Microsoft",
                                    "Oracle",
                                    "SAP Machine",
                                    "Liberica",
                                ])),
                                #[watch]
                                set_selected: match &model.filter_distro {
                                    None => 0,
                                    Some(d) if d == "Temurin" => 1,
                                    Some(d) if d == "Zulu" => 2,
                                    Some(d) if d == "Corretto" => 3,
                                    Some(d) if d == "Microsoft" => 4,
                                    Some(d) if d == "Oracle" => 5,
                                    Some(d) if d == "SAP Machine" => 6,
                                    Some(d) if d == "Liberica" => 7,
                                    _ => 0,
                                },
                                connect_selected_item_notify[sender] => move |dd| {
                                    let distro = match dd.selected() {
                                        0 => None,
                                        i => dd.model().unwrap().dynamic_cast::<gtk::StringList>().unwrap().string(i).map(|s| s.to_string()),
                                    };
                                    sender.input(InstallJavaInput::FilterDistro(distro));
                                }
                            }
                        },

                        gtk::Stack {
                            set_vexpand: true,
                            add_named[Some("list")] = &gtk::ScrolledWindow {
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_vexpand: true,

                                #[local_ref]
                                _version_list_box -> gtk::ListBox {
                                },
                            },

                            add_named[Some("empty")] = &adw::StatusPage {
                                set_icon_name: Some("system-search-symbolic"),
                                set_title: "No Results",
                                set_description: Some("Try adjusting your filters or search query."),
                            },

                            #[watch]
                            set_visible_child_name: if model.version_factory.is_empty() { "empty" } else { "list" },
                        },
                    },
                    add_named[Some("installing")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 32,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,

                        adw::Spinner {
                            #[watch]
                            set_visible: model.progress < 0.0,
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
                            set_visible: model.progress >= 0.0,
                            set_width_request: 300,
                        },

                        gtk::Button {
                            set_label: "Cancel",
                            set_css_classes: &["destructive-action", "pill"],
                            connect_clicked => InstallJavaInput::Cancel,
                        }
                    },
                    add_named[Some("loading")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 16,

                        gtk::Label {
                            set_label: "Loading available runtimes...",
                            set_css_classes: &["dim-label"],
                        },

                        adw::Spinner {
                            set_width_request: 32,
                            set_height_request: 32,
                        },
                    },
                    add_named[Some("success")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,

                        gtk::Image {
                            set_icon_name: Some("object-select-symbolic"),
                            set_pixel_size: 64,
                            set_css_classes: &["success"],
                        },

                        gtk::Label {
                            set_label: "Installation Complete",
                            set_css_classes: &["title-2"],
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &format!(
                                "{} has been successfully installed.",
                                model.selected_package.as_ref().map(|p| format!("{} {}", p.distribution, p.java_version)).unwrap_or_default()
                            ),
                            set_css_classes: &["dim-label"],
                        },
                    },
                    add_named[Some("error")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_vexpand: true,

                        gtk::Image {
                            set_icon_name: Some("window-close-symbolic"),
                            set_pixel_size: 64,
                            set_css_classes: &["destructive"],
                        },

                        gtk::Label {
                            set_label: "Installation Failed",
                            set_css_classes: &["title-2", "error"],
                        },

                        gtk::Label {
                            #[watch]
                            set_label: &model.status,
                            set_css_classes: &["dim-label"],
                            set_wrap: true,
                            set_max_width_chars: 40,
                            set_justify: gtk::Justification::Center,
                        },
                    },
                    #[watch]
                    set_visible_child_name: match (model.loading_versions, model.installing, model.finished, model.has_error) {
                        (true, _, _, _) => "loading",
                        (_, true, _, _) => "installing",
                        (_, _, true, _) => "success",
                        (_, _, _, true) => "error",
                        (false, false, false, false) => "select",
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
        let list_box = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .selection_mode(gtk::SelectionMode::None)
            .build();

        let version_factory = FactoryVecDeque::builder()
            .launch(list_box)
            .forward(sender.input_sender(), InstallJavaInput::SelectPackage);

        let model = InstallJavaDialog {
            visible: false,
            installing: false,
            finished: false,
            has_error: false,
            status: String::new(),
            target_dir: init,
            selected_package: None,
            all_packages: Vec::new(),
            search_query: String::new(),
            filter_distro: None,
            progress: -1.0,
            version_factory,
            loading_versions: false,
            cancel_flag: None,
        };

        let _version_list_box = model.version_factory.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            InstallJavaInput::Open => {
                self.visible = true;
                self.installing = false;
                self.finished = false;
                self.has_error = false;
                self.status.clear();
                self.progress = -1.0;

                if self.all_packages.is_empty() {
                    self.loading_versions = true;
                    let sender_clone = sender.input_sender().clone();
                    std::thread::spawn(move || {
                        let result = fetch_java_packages();
                        let _ = sender_clone.send(InstallJavaInput::VersionsLoaded(result));
                    });
                }
            }
            InstallJavaInput::VersionsLoaded(result) => {
                self.loading_versions = false;
                if let Ok(packages) = result {
                    self.all_packages = packages;
                    self.apply_filters();
                }
            }
            InstallJavaInput::Search(query) => {
                self.search_query = query.to_lowercase();
                self.apply_filters();
            }
            InstallJavaInput::FilterDistro(distro) => {
                self.filter_distro = distro;
                self.apply_filters();
            }
            InstallJavaInput::Close => {
                self.visible = false;
            }
            InstallJavaInput::Refresh => {
                self.loading_versions = true;
                let sender_clone = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let result = fetch_java_packages();
                    let _ = sender_clone.send(InstallJavaInput::VersionsLoaded(result));
                });
            }
            InstallJavaInput::SelectPackage(id) => {
                for i in 0..self.version_factory.len() {
                    let is_selected = self
                        .version_factory
                        .get(i)
                        .map(|r| r.package.id == id)
                        .unwrap_or(false);
                    if is_selected {
                        self.selected_package =
                            self.version_factory.get(i).map(|r| r.package.clone());
                    }
                    self.version_factory
                        .send(i, VersionRowInput::SetSelected(is_selected));
                }
            }
            InstallJavaInput::Install => {
                if let Some(package) = &self.selected_package {
                    self.installing = true;
                    self.has_error = false;
                    self.progress = -1.0;
                    self.status = format!("Starting {} installation...", package.java_version);

                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());

                    let sender_clone = sender.input_sender().clone();
                    let target_dir = self.target_dir.clone();
                    let package_id = package.id.clone();

                    std::thread::spawn(move || {
                        crate::backend::download::sources::java::download_and_extract_with_progress(
                            &package_id,
                            &target_dir,
                            cancel_flag,
                            move |progress| {
                                let _ = sender_clone.send(InstallJavaInput::Progress(progress));
                            },
                        );
                    });
                }
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
                use crate::backend::download::sources::java::JavaDownloadProgress;
                match progress {
                    JavaDownloadProgress::Downloading { current, total } => {
                        self.progress = if total > 0 {
                            current as f32 / total as f32
                        } else {
                            0.0
                        };
                        self.status = format!("Downloading... ({:.1}%)", self.progress * 100.0);
                    }
                    JavaDownloadProgress::Extracting => {
                        self.status = "Extracting runtime...".to_string();
                        self.progress = 1.0;
                    }
                    JavaDownloadProgress::Finished(_) => {
                        self.installing = false;
                        self.finished = true;
                        self.status.clear();
                        self.progress = 0.0;
                        self.visible = false;
                        self.cancel_flag = None;
                        sender.output(InstallJavaOutput::Finished).ok();
                    }
                    JavaDownloadProgress::Error(e) => {
                        self.installing = false;
                        self.has_error = true;
                        self.status = format!("{}", e);
                        self.cancel_flag = None;
                    }
                }
            }
        }
    }
}

impl InstallJavaDialog {
    fn apply_filters(&mut self) {
        let mut guard = self.version_factory.guard();

        // Collect filtered packages
        let filtered: Vec<JavaPackage> = self
            .all_packages
            .iter()
            .filter(|package| {
                if !self.search_query.is_empty() {
                    let search_text = format!(
                        "java {} {} {} jdk jre",
                        package.major_version, package.distribution, package.java_version
                    )
                    .to_lowercase();

                    // Split query into words and ensure all words match
                    for word in self.search_query.split_whitespace() {
                        if !search_text.contains(word) {
                            return false;
                        }
                    }
                }

                if let Some(distro) = &self.filter_distro {
                    if !package
                        .distribution
                        .to_lowercase()
                        .contains(&distro.to_lowercase())
                    {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Only update if the list has actually changed (basic check)
        if guard.len() == filtered.len() {
            let mut identical = true;
            for (i, p) in filtered.iter().enumerate() {
                if guard.get(i).map(|r| &r.package.id) != Some(&p.id) {
                    identical = false;
                    break;
                }
            }
            if identical {
                return;
            }
        }

        guard.clear();
        for package in filtered {
            guard.push_back(package);
        }
    }
}
