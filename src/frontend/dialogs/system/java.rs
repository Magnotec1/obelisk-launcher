use crate::backend::runtime::java::{find_java_versions, JavaInstance, JavaSource};
use adw::prelude::*;
use gtk::gio;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JavaRow {
    name: String,
    version: String,
    path: PathBuf,
    source: JavaSource,
    selected: bool,
}

#[derive(Debug)]
pub enum JavaRowInput {
    SetSelected(bool),
}

#[relm4::factory(pub)]
impl FactoryComponent for JavaRow {
    type Init = JavaInstance;
    type Input = JavaRowInput;
    type Output = PathBuf;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.name,
            #[watch]
            set_subtitle: &format!(
                "Version: {} - {} [{}]",
                self.version,
                self.path.display(),
                match self.source {
                    JavaSource::System => "System",
                    JavaSource::Launcher => "Launcher",
                }
            ),
            add_prefix = &gtk::Image::from_icon_name(match self.source {
                JavaSource::System => "system-run-symbolic",
                JavaSource::Launcher => "folder-download-symbolic",
            }),
            add_suffix = &gtk::Image {
                set_icon_name: Some("object-select-symbolic"),
                #[watch]
                set_visible: self.selected,
            },
            set_activatable: true,
            connect_activated[sender, path = self.path.clone()] => move |_| {
                let _ = sender.output(path.clone());
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            name: init.name,
            version: init.version,
            path: init.path,
            source: init.source,
            selected: false,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            JavaRowInput::SetSelected(selected) => {
                self.selected = selected;
            }
        }
    }
}

pub struct JavaSelectorDialog {
    visible: bool,
    java_list: FactoryVecDeque<JavaRow>,
    all_versions: Vec<JavaInstance>,
    loading: bool,
    launcher_java_dir: Option<PathBuf>,
    selected_path: Option<PathBuf>,
    search_text: String,
}

#[derive(Debug)]
pub enum JavaSelectorInput {
    Open,
    Close,
    Refresh,
    Detected(Vec<JavaInstance>),
    Select(PathBuf),
    Browse,
    SearchChanged(String),
    Use,
}

#[derive(Debug)]
pub enum JavaSelectorOutput {
    Selected(PathBuf),
}

impl JavaSelectorDialog {
    fn apply_filter(&mut self) {
        let query = self.search_text.to_lowercase();
        let mut guard = self.java_list.guard();
        guard.clear();

        for v in &self.all_versions {
            if query.is_empty()
                || v.name.to_lowercase().contains(&query)
                || v.version.to_lowercase().contains(&query)
            {
                guard.push_back(v.clone());
            }
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for JavaSelectorDialog {
    type Init = Option<PathBuf>;
    type Input = JavaSelectorInput;
    type Output = JavaSelectorOutput;

    view! {
            #[name = "dialog"]
            adw::Dialog {
                set_title: "Select Java Version",
                set_content_width: 500,
                set_content_height: 400,
                set_can_close: true,

                #[wrap(Some)]
                set_child = &adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "Java Installations",
                        },
                        pack_end = &gtk::Button {
                            set_icon_name: "view-refresh-symbolic",
                            connect_clicked => JavaSelectorInput::Refresh,
                        }
                    },
                    add_bottom_bar = &gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_margin_bottom: 16,
                        set_margin_start: 16,
                        set_margin_end: 16,
                        set_halign: gtk::Align::Center,

                        gtk::Button {
                            set_label: "Cancel",
                            set_css_classes: &["pill"],
                            connect_clicked[root, sender] => move |_| {
                                sender.input(JavaSelectorInput::Close);
                                root.close();
                            },
                        },
                        gtk::Button {
                            set_label: "Use",
                            #[watch]
                            set_sensitive: model.selected_path.is_some(),
                            set_css_classes: &["suggested-action", "pill"],
                            connect_clicked[root, sender] => move |_| {
                                sender.input(JavaSelectorInput::Use);
                                root.close();
                            },
                        }
                    },
                    #[wrap(Some)]
                    set_content = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 8,
                            set_margin_start: 16,
                            set_margin_end: 16,

                            gtk::SearchEntry {
                                set_hexpand: true,
                                set_placeholder_text: Some("Search versions..."),
                                connect_search_changed[sender] => move |entry| {
                                    sender.input(JavaSelectorInput::SearchChanged(entry.text().to_string()));
                                },
                            },

                            gtk::Button {
                                set_icon_name: "folder-open-symbolic",
                                set_tooltip_text: Some("Browse for Java executable"),
                                connect_clicked => JavaSelectorInput::Browse,
                            },
                        },

                        gtk::Stack {
                            set_margin_start: 16,
                            set_margin_end: 16,
                            set_margin_bottom: 16,
                            set_vexpand: true,

                        add_named[Some("loading")] = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_halign: gtk::Align::Center,
                            set_valign: gtk::Align::Center,
                            set_hexpand: true,
                            set_vexpand: true,
                            set_spacing: 12,

                            adw::Spinner {
                                set_width_request: 32,
                                set_height_request: 32,
                            },

                            gtk::Label {
                                set_label: "Scanning for Java runtimes...",
                                set_css_classes: &["dim-label"],
                            }
                        },

                        add_named[Some("content")] = &gtk::ScrolledWindow {
                            set_vexpand: true,
                            #[local_ref]
                            _java_list_box -> gtk::ListBox {},
                        },
                        #[watch]
                        set_visible_child_name: if model.loading { "loading" } else { "content" },
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
        let list_box = gtk::ListBox::builder().css_classes(["boxed-list"]).build();

        let java_list = FactoryVecDeque::builder()
            .launch(list_box)
            .forward(sender.input_sender(), JavaSelectorInput::Select);

        let model = JavaSelectorDialog {
            visible: false,
            java_list,
            all_versions: Vec::new(),
            loading: false,
            launcher_java_dir: init,
            selected_path: None,
            search_text: String::new(),
        };

        let _java_list_box = model.java_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            JavaSelectorInput::Open => {
                self.visible = true;
                sender.input(JavaSelectorInput::Refresh);
            }
            JavaSelectorInput::Close => {
                self.visible = false;
            }
            JavaSelectorInput::Refresh => {
                self.loading = true;
                let sender_clone = sender.input_sender().clone();
                let java_dir = self.launcher_java_dir.clone();
                std::thread::spawn(move || {
                    let versions = find_java_versions(java_dir.as_deref());
                    let _ = sender_clone.send(JavaSelectorInput::Detected(versions));
                });
            }
            JavaSelectorInput::Detected(versions) => {
                self.loading = false;
                self.all_versions = versions;
                self.apply_filter();
            }
            JavaSelectorInput::SearchChanged(text) => {
                self.search_text = text;
                self.apply_filter();
            }
            JavaSelectorInput::Select(path) => {
                self.selected_path = Some(path.clone());
                for i in 0..self.java_list.len() {
                    let is_selected = self
                        .java_list
                        .get(i)
                        .map(|r| r.path == path)
                        .unwrap_or(false);
                    self.java_list
                        .send(i, JavaRowInput::SetSelected(is_selected));
                }
            }
            JavaSelectorInput::Browse => {
                let sender_clone = sender.input_sender().clone();
                let file_dialog = gtk::FileDialog::builder()
                    .title("Select Java Executable")
                    .build();

                file_dialog.open(
                    None::<&gtk::Window>,
                    None::<&gio::Cancellable>,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                let _ = sender_clone.send(JavaSelectorInput::Select(path));
                            }
                        }
                    },
                );
            }
            JavaSelectorInput::Use => {
                if let Some(path) = self.selected_path.take() {
                    sender.output(JavaSelectorOutput::Selected(path)).ok();
                    self.visible = false;
                }
            }
        }
    }
}
