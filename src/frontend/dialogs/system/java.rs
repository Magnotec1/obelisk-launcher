use crate::backend::runtime::java::{find_java_versions, JavaInstance, JavaSource};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct JavaRow {
    name: String,
    version: String,
    path: PathBuf,
    source: JavaSource,
}

#[relm4::factory(pub)]
impl FactoryComponent for JavaRow {
    type Init = JavaInstance;
    type Input = ();
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
        }
    }
}

pub struct JavaSelectorDialog {
    visible: bool,
    java_list: FactoryVecDeque<JavaRow>,
    loading: bool,
    launcher_java_dir: Option<PathBuf>,
}

#[derive(Debug)]
pub enum JavaSelectorInput {
    Open,
    Close,
    Refresh,
    Detected(Vec<JavaInstance>),
}

#[derive(Debug)]
pub enum JavaSelectorOutput {
    Selected(PathBuf),
}

#[relm4::component(pub)]
impl SimpleComponent for JavaSelectorDialog {
    type Init = Option<PathBuf>;
    type Input = JavaSelectorInput;
    type Output = JavaSelectorOutput;

    view! {
        adw::Window {
            set_title: Some("Select Java Version"),
            set_default_width: 500,
            set_default_height: 400,
            set_modal: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(JavaSelectorInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Detect Java Installations",
                    },
                    pack_start = &gtk::Button {
                        set_icon_name: "view-refresh-symbolic",
                        connect_clicked => JavaSelectorInput::Refresh,
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 20,
                    set_spacing: 12,

                    gtk::Label {
                        set_label: "Scanning your system for Java runtimes...",
                        set_halign: gtk::Align::Start,
                        set_css_classes: &["dim-label"],
                        #[watch]
                        set_visible: model.loading,
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        _java_list_box -> gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 12,
                        set_halign: gtk::Align::End,

                        gtk::Button {
                            set_label: "Browse...",
                            set_css_classes: &["pill"],
                            connect_clicked => JavaSelectorInput::Close, // Placeholder
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
        let java_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.output_sender(), JavaSelectorOutput::Selected);

        let model = JavaSelectorDialog {
            visible: false,
            java_list,
            loading: false,
            launcher_java_dir: init,
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
                let mut guard = self.java_list.guard();
                guard.clear();
                for v in versions {
                    guard.push_back(v);
                }
            }
        }
    }
}
