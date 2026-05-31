use crate::backend::runtime::java::{find_java_versions, JavaInstance, JavaSource};
use crate::config::Config;
use crate::frontend::dialogs::system::install_java::{
    InstallJavaDialog, InstallJavaInput, InstallJavaOutput,
};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;
use std::path::PathBuf;

pub struct SettingsDialog {
    config: Config,
    active_page: String,
    java_versions: Vec<JavaInstance>,
    java_installer: Controller<InstallJavaDialog>,
    launcher_java_factory: FactoryVecDeque<JavaRow>,
    system_java_factory: FactoryVecDeque<JavaRow>,
}

#[derive(Debug)]
pub struct JavaRow {
    name: String,
    version: String,
    path: PathBuf,
}

#[relm4::factory(pub)]
impl FactoryComponent for JavaRow {
    type Init = JavaInstance;
    type Input = ();
    type Output = PathBuf;
    type CommandOutput = ();
    type ParentWidget = adw::ExpanderRow;

    view! {
        adw::ActionRow {
            set_title: &self.name,
            set_subtitle: &format!("Version: {} - {}", self.version, self.path.display()),

            add_suffix = &gtk::Button {
                #[watch]
                set_visible: self.path.to_string_lossy().contains("/java/"),
                set_icon_name: "edit-delete-symbolic",
                set_css_classes: &["flat", "circular"],
                connect_clicked[sender, path = self.path.clone()] => move |_| {
                    let _ = sender.output(path.clone());
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            name: init.name,
            version: init.version,
            path: init.path,
        }
    }
}

#[derive(Debug)]
pub enum SettingsInput {
    UpdateConfig(Config),
    SetPage(String),
    SetInstancesPath(PathBuf),
    ClearInstancesPath,
    SetSharedPath(PathBuf),
    ClearSharedPath,
    SetMaxMemory(u32),
    SetMinMemory(u32),
    SetClientId(String),
    SetDefaultIcon(PathBuf),
    ClearDefaultIcon,
    RefreshJava,
    OpenJavaInstaller,
    DeleteJava(PathBuf),
    SetJavaVersions(Vec<JavaInstance>),
}

#[derive(Debug)]
pub enum SettingsOutput {
    ConfigUpdated(Config),
    OpenAccountManager,
}

#[relm4::component(pub)]
impl SimpleComponent for SettingsDialog {
    type Init = Config;
    type Input = SettingsInput;
    type Output = SettingsOutput;

    view! {
        adw::Dialog {
            set_title: "Preferences",
            set_content_width: 650,
            set_content_height: 550,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::ViewSwitcher {
                        set_policy: adw::ViewSwitcherPolicy::Wide,
                        #[watch]
                        set_stack: Some(&settings_stack),
                    },
                },

                #[wrap(Some)]
                #[name = "settings_stack"]
                set_content = &adw::ViewStack {
                    set_vexpand: true,
                    connect_visible_child_name_notify[sender] => move |stack| {
                        if let Some(name) = stack.visible_child_name() {
                            sender.input(SettingsInput::SetPage(name.to_string()));
                        }
                    },

                add_titled_with_icon[Some("general"), "General", "preferences-other-symbolic"] = &adw::PreferencesPage {
                    adw::PreferencesGroup {
                        set_title: "Path Configuration",

                        adw::ActionRow {
                            set_title: "Instance Folder",
                            #[watch]
                            set_subtitle: &model.config.instances_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Not set".to_string()),

                            add_suffix = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 8,
                                set_valign: gtk::Align::Center,

                                gtk::Button {
                                    set_label: "Select",
                                    set_tooltip_text: Some("Select instance folder"),
                                    connect_clicked[sender] => move |_| {
                                        let dialog = gtk::FileDialog::builder().title("Select Instance Folder").build();
                                        let sender = sender.clone();
                                        dialog.select_folder(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, move |res| {
                                            if let Ok(f) = res { if let Some(p) = f.path() { sender.input(SettingsInput::SetInstancesPath(p)); } }
                                        });
                                    }
                                },

                                gtk::Button {
                                    set_icon_name: "edit-clear-symbolic",
                                    set_css_classes: &["flat", "circular"],
                                    set_tooltip_text: Some("Clear instance folder path"),
                                    #[watch]
                                    set_sensitive: model.config.instances_path.is_some(),
                                    connect_clicked => SettingsInput::ClearInstancesPath,
                                }
                            }
                        },

                        adw::ActionRow {
                            set_title: "Shared Assets Folder",
                            #[watch]
                            set_subtitle: &model.config.shared_data_path
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Not set".to_string()),

                            add_suffix = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 8,
                                set_valign: gtk::Align::Center,

                                gtk::Button {
                                    set_label: "Select",
                                    set_tooltip_text: Some("Select shared assets folder"),
                                    connect_clicked[sender] => move |_| {
                                        let dialog = gtk::FileDialog::builder().title("Select Shared Assets Folder").build();
                                        let sender = sender.clone();
                                        dialog.select_folder(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, move |res| {
                                            if let Ok(f) = res { if let Some(p) = f.path() { sender.input(SettingsInput::SetSharedPath(p)); } }
                                        });
                                    }
                                },

                                gtk::Button {
                                    set_icon_name: "edit-clear-symbolic",
                                    set_css_classes: &["flat", "circular"],
                                    set_tooltip_text: Some("Clear shared assets folder path"),
                                    #[watch]
                                    set_sensitive: model.config.shared_data_path.is_some(),
                                    connect_clicked => SettingsInput::ClearSharedPath,
                                }
                            }
                        }
                    },

                    adw::PreferencesGroup {
                        set_title: "Appearance",

                        adw::ActionRow {
                            set_title: "Default Instance Icon",
                            #[watch]
                            set_subtitle: &model.config.default_instance_icon
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|| "Not set".to_string()),

                            add_suffix = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 8,
                                set_valign: gtk::Align::Center,

                                gtk::Button {
                                    set_label: "Select",
                                    connect_clicked[sender] => move |_| {
                                        let dialog = gtk::FileDialog::builder()
                                            .title("Select Default Instance Icon")
                                            .build();
                                        let filters = gtk::FileFilter::new();
                                        filters.add_mime_type("image/png");
                                        filters.add_mime_type("image/jpeg");
                                        filters.set_name(Some("Images"));
                                        let list_store = gtk::gio::ListStore::new::<gtk::FileFilter>();
                                        list_store.append(&filters);
                                        dialog.set_filters(Some(&list_store));

                                        let sender = sender.clone();
                                        dialog.open(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, move |res| {
                                            if let Ok(f) = res {
                                                if let Some(p) = f.path() {
                                                    sender.input(SettingsInput::SetDefaultIcon(p));
                                                }
                                            }
                                        });
                                    }
                                },

                                gtk::Button {
                                    set_icon_name: "edit-clear-symbolic",
                                    set_css_classes: &["flat", "circular"],
                                    set_tooltip_text: Some("Clear default icon"),
                                    #[watch]
                                    set_sensitive: model.config.default_instance_icon.is_some(),
                                    connect_clicked => SettingsInput::ClearDefaultIcon,
                                }
                            }
                        },
                    },
                },

                add_titled_with_icon[Some("java"), "Java", "system-run-symbolic"] = &adw::PreferencesPage {
                    adw::PreferencesGroup {
                        set_title: "Java Configuration",



                        adw::ActionRow {
                            set_title: "Maximum Memory (MB)",
                            add_suffix = &gtk::SpinButton {
                                set_valign: gtk::Align::Center,
                                set_adjustment: &gtk::Adjustment::new(4096.0, 512.0, 65536.0, 512.0, 1024.0, 0.0),
                                #[watch]
                                set_value: model.config.max_memory as f64,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(SettingsInput::SetMaxMemory(spin.value() as u32));
                                }
                            }
                        },

                        adw::ActionRow {
                            set_title: "Minimum Memory (MB)",
                            add_suffix = &gtk::SpinButton {
                                set_valign: gtk::Align::Center,
                                set_adjustment: &gtk::Adjustment::new(512.0, 128.0, 16384.0, 128.0, 512.0, 0.0),
                                #[watch]
                                set_value: model.config.min_memory as f64,
                                connect_value_changed[sender] => move |spin| {
                                    sender.input(SettingsInput::SetMinMemory(spin.value() as u32));
                                }
                            }
                        }
                    },

                    adw::PreferencesGroup {
                        set_title: "Java Installations",

                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,
                            set_margin_bottom: 16,

                            adw::ActionRow {
                                set_title: "Install Managed Runtime",
                                set_subtitle: "Download and isolate a new Java version",
                                add_prefix = &gtk::Image::from_icon_name("list-add-symbolic"),
                                add_suffix = &gtk::Image::from_icon_name("go-next-symbolic"),
                                set_activatable: true,
                                connect_activated => SettingsInput::OpenJavaInstaller,
                            },
                        },

                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,

                            #[local_ref]
                            launcher_expander -> adw::ExpanderRow {},

                            #[local_ref]
                            system_expander -> adw::ExpanderRow {},
                        },
                    },
                },

                add_titled_with_icon[Some("accounts"), "Accounts", "avatar-default-symbolic"] = &adw::PreferencesPage {
                    adw::PreferencesGroup {
                        set_title: "Microsoft Authentication",
                        set_description: Some("Your Azure AD Application client ID."),

                        adw::EntryRow {
                            set_title: "Client ID",
                            set_show_apply_button: true,
                            #[watch]
                            set_text: &model.config.microsoft_client_id.clone().unwrap_or_default(),
                            connect_apply[sender] => move |entry| {
                                sender.input(SettingsInput::SetClientId(entry.text().to_string()));
                            },
                        }
                    },

                },

                // Must come after all add_titled_with_icon so children exist on first render
                #[watch]
                set_visible_child_name: &model.active_page,
            }
            }
        }
    }

    fn init(
        config: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let java_dir = config.minecraft_data_path.join("java");

        let java_installer =
            InstallJavaDialog::builder()
                .launch(java_dir)
                .forward(sender.input_sender(), |out| match out {
                    InstallJavaOutput::Finished => SettingsInput::RefreshJava,
                });

        let launcher_java_row = adw::ExpanderRow::builder()
            .title("Managed Runtimes")
            .subtitle("Downloaded by the launcher")
            .expanded(false)
            .build();
        launcher_java_row.add_prefix(&gtk::Image::from_icon_name("folder-download-symbolic"));

        let launcher_java_factory = FactoryVecDeque::builder()
            .launch(launcher_java_row)
            .forward(sender.input_sender(), SettingsInput::DeleteJava);

        let system_java_row = adw::ExpanderRow::builder()
            .title("System Runtimes")
            .subtitle("Detected on your operating system")
            .expanded(false)
            .build();
        system_java_row.add_prefix(&gtk::Image::from_icon_name("computer-symbolic"));

        let system_java_factory = FactoryVecDeque::builder()
            .launch(system_java_row)
            .forward(sender.input_sender(), |_| unreachable!());

        let model = SettingsDialog {
            config,
            active_page: "general".to_string(),
            java_versions: Vec::new(),
            java_installer,
            launcher_java_factory,
            system_java_factory,
        };

        let launcher_expander = model.launcher_java_factory.widget();
        let system_expander = model.system_java_factory.widget();

        let widgets = view_output!();

        sender.input(SettingsInput::RefreshJava);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            SettingsInput::UpdateConfig(config) => {
                self.config = config;
                sender.input(SettingsInput::RefreshJava);
            }
            SettingsInput::SetPage(page) => {
                self.active_page = page;
            }
            SettingsInput::SetInstancesPath(path) => {
                self.config.instances_path = Some(path);
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::SetSharedPath(path) => {
                self.config.shared_data_path = Some(path);
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }

            SettingsInput::SetMaxMemory(val) => {
                self.config.max_memory = val;
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::SetMinMemory(val) => {
                self.config.min_memory = val;
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::SetClientId(id) => {
                self.config.microsoft_client_id = Some(id);
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::SetDefaultIcon(path) => {
                self.config.default_instance_icon = Some(path);
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::ClearDefaultIcon => {
                self.config.default_instance_icon = None;
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::ClearInstancesPath => {
                self.config.instances_path = None;
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }
            SettingsInput::ClearSharedPath => {
                self.config.shared_data_path = None;
                let _ = self.config.save();
                sender
                    .output(SettingsOutput::ConfigUpdated(self.config.clone()))
                    .unwrap();
            }

            SettingsInput::SetJavaVersions(versions) => {
                self.java_versions = versions.clone();
                let mut launcher_guard = self.launcher_java_factory.guard();
                let mut system_guard = self.system_java_factory.guard();
                launcher_guard.clear();
                system_guard.clear();

                for java in versions {
                    match java.source {
                        JavaSource::Launcher => {
                            launcher_guard.push_back(java);
                        }
                        JavaSource::System => {
                            system_guard.push_back(java);
                        }
                    }
                }
            }
            SettingsInput::RefreshJava => {
                let java_dir = self.config.minecraft_data_path.join("java");
                let sender_clone = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let versions = find_java_versions(Some(&java_dir));
                    let _ = sender_clone.send(SettingsInput::SetJavaVersions(versions));
                });
            }

            SettingsInput::OpenJavaInstaller => {
                self.java_installer.emit(InstallJavaInput::Open);
                let parent = relm4::main_application().active_window();
                self.java_installer.widget().present(parent.as_ref());
            }
            SettingsInput::DeleteJava(path) => {
                let java_dir = self.config.minecraft_data_path.join("java");
                if path.starts_with(&java_dir) {
                    let mut current = path.clone();
                    while let Some(parent) = current.parent() {
                        if parent == java_dir {
                            let _ = std::fs::remove_dir_all(&current);
                            break;
                        }
                        current = parent.to_path_buf();
                    }
                }
                sender.input(SettingsInput::RefreshJava);
            }
        }
    }
}
