use crate::backend::auth::account::{add_account, create_offline_account};
use crate::backend::download::manager::{fetch_java_packages, JavaPackage};
use crate::backend::download::sources::java::{
    download_and_extract_with_progress, JavaDownloadProgress,
};
use crate::backend::runtime::java::{find_java_versions, get_java_major_version, JavaInstance};
use crate::config::Config;
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct SetupDialog {
    visible: bool,
    config: Config,
    current_step: usize,

    // Step 2: Client ID
    client_id_input: String,
    client_id_changed: bool,

    // Step 3: Offline account username input
    offline_username: String,

    // Step 4: Java installation & verification
    java_versions: Vec<JavaInstance>,
    loading_java: bool,
    all_packages: Vec<JavaPackage>,
    loading_packages: bool,
    installing_version: Option<u32>, // None, Some(8), Some(17), or Some(21)
    java_install_progress: f32,
    java_install_status: String,
    cancel_flag: Option<Arc<AtomicBool>>,
}

#[derive(Debug)]
pub enum SetupInput {
    Open,
    Close,
    UpdateConfig(Config),
    SetStep(usize),
    NextStep,
    PrevStep,

    // Step 1: Instances Path
    SetInstancesPath(PathBuf),

    // Step 2: Client ID
    SetClientId(String),

    // Step 3: Account
    SetOfflineUsername(String),
    AddOfflineAccount,
    StartMicrosoftLogin,

    // Step 4: Java management
    RefreshJava,
    SetJavaVersions(Vec<JavaInstance>),
    PackagesLoaded(Result<Vec<JavaPackage>, String>),
    TriggerInstallPrompt(u32),
    InstallJava(u32),
    JavaProgress(u32, JavaDownloadProgress),
    CancelInstall,
}

#[derive(Debug)]
pub enum SetupOutput {
    StartMicrosoftLogin,
    ConfigUpdated(Config),
}

impl SetupDialog {
    fn is_step_completed(&self, step: usize) -> bool {
        match step {
            0 => true, // Welcome is always complete
            1 => self.config.instances_path.is_some(),
            2 => self
                .config
                .microsoft_client_id
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false),
            3 => !self.config.accounts.is_empty(),
            4 => !self.java_versions.is_empty(), // Javas verified if at least one version exists
            5 => true,                           // Finish page is always complete
            _ => false,
        }
    }

    fn has_java_version(&self, major: u32) -> bool {
        self.java_versions
            .iter()
            .any(|v| get_java_major_version(&v.version) == Some(major))
    }

    #[allow(dead_code)]
    fn get_java_path(&self, major: u32) -> Option<PathBuf> {
        self.java_versions
            .iter()
            .find(|v| get_java_major_version(&v.version) == Some(major))
            .map(|v| v.path.clone())
    }

    fn get_java_name(&self, major: u32) -> Option<String> {
        self.java_versions
            .iter()
            .find(|v| get_java_major_version(&v.version) == Some(major))
            .map(|v| format!("{} ({})", v.name, v.version))
    }

    fn get_java_status(&self, major: u32) -> String {
        if self.has_java_version(major) {
            format!(
                "Installed: {}",
                self.get_java_name(major).unwrap_or_default()
            )
        } else if self.installing_version == Some(major) {
            self.java_install_status.clone()
        } else {
            match major {
                8 => "Missing — Required for Minecraft 1.16.5 and older.".to_string(),
                17 => "Missing — Required for Minecraft 1.17 to 1.20.4.".to_string(),
                21 => "Missing — Required for Minecraft 1.20.5+.".to_string(),
                _ => "Missing".to_string(),
            }
        }
    }
}

fn format_size(bytes: Option<i64>) -> String {
    match bytes {
        Some(b) if b > 0 => {
            if b >= 1024 * 1024 * 1024 {
                format!("{:.1} GB", b as f64 / (1024.0 * 1024.0 * 1024.0))
            } else if b >= 1024 * 1024 {
                format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
            } else if b >= 1024 {
                format!("{:.1} KB", b as f64 / 1024.0)
            } else {
                format!("{} Bytes", b)
            }
        }
        _ => "Estimated ~100 MB".to_string(),
    }
}

fn find_package_for_version(packages: &[JavaPackage], major: u32) -> Option<JavaPackage> {
    let distros_priority = [
        "Temurin",
        "Zulu",
        "Corretto",
        "Liberica",
        "Microsoft",
        "Oracle",
    ];
    for distro in distros_priority {
        if let Some(pkg) = packages.iter().find(|p| {
            p.major_version == major
                && p.distribution
                    .to_lowercase()
                    .contains(&distro.to_lowercase())
        }) {
            return Some(pkg.clone());
        }
    }
    packages.iter().find(|p| p.major_version == major).cloned()
}

#[relm4::component(pub)]
impl SimpleComponent for SetupDialog {
    type Init = Config;
    type Input = SetupInput;
    type Output = SetupOutput;

    view! {
        #[name = "dialog"]
        adw::Dialog {
            set_title: "Obelisk Launcher Setup Walkthrough",
            set_content_width: 580,
            set_content_height: 520,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        #[watch]
                        set_title: &format!("Obelisk Setup • Step {} of 5", model.current_step),
                        #[watch]
                        set_subtitle: match model.current_step {
                            0 => "Welcome",
                            1 => "Workspace Directory",
                            2 => "Azure Client ID",
                            3 => "Account Configuration",
                            4 => "Java Dependencies",
                            5 => "Finish",
                            _ => "",
                        },
                    },
                    set_show_end_title_buttons: true,
                },

                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,

                    // Left button: Previous
                    gtk::Button {
                        set_label: "Previous",
                        set_css_classes: &["pill"],
                        set_icon_name: "go-previous-symbolic",
                        #[watch]
                        set_visible: model.current_step > 0 && model.current_step < 5,
                        #[watch]
                        set_sensitive: model.installing_version.is_none(),
                        set_tooltip_text: Some("Go back to the previous setup step"),
                        connect_clicked => SetupInput::PrevStep,
                    },

                    gtk::Box {
                        set_hexpand: true,
                    },

                    gtk::Button {
                        set_label: "Cancel",
                        set_css_classes: &["pill"],
                        #[watch]
                        set_visible: model.current_step == 0,
                        set_tooltip_text: Some("Skip setup walkthrough and configure settings later"),
                        connect_clicked[root, sender] => move |_| {
                            sender.input(SetupInput::Close);
                            root.close();
                        }
                    },

                    // Case B: Continue button (Visible when step is completed, up to step 4)
                    gtk::Button {
                        #[watch]
                        set_label: if model.current_step == 0 { "Start Setup" } else { "Continue" },
                        set_css_classes: &["suggested-action", "pill"],
                        #[watch]
                        set_visible: model.current_step < 5 && (model.current_step == 0 || model.is_step_completed(model.current_step)),
                        #[watch]
                        set_sensitive: model.installing_version.is_none(),
                        #[watch]
                        set_tooltip_text: Some(if model.current_step == 0 { "Begin launcher configuration walkthrough" } else { "Save step and proceed to the next section" }),
                        connect_clicked[sender] => move |_| {
                            sender.input(SetupInput::NextStep);
                        }
                    },

                    // Case C: Finish Setup button (Visible on completed step 5)
                    gtk::Button {
                        set_label: "Finish Setup",
                        set_css_classes: &["suggested-action", "pill"],
                        #[watch]
                        set_visible: model.current_step == 5,
                        set_tooltip_text: Some("Save setup walkthrough state and open the launcher"),
                        connect_clicked[root, sender] => move |_| {
                            sender.input(SetupInput::Close);
                            root.close();
                        }
                    },

                    // Case D: Skip Step button (Visible when step is NOT completed)
                    gtk::Button {
                        set_label: "Skip Step",
                        set_css_classes: &["pill"],
                        #[watch]
                        set_visible: model.current_step > 0 && model.current_step < 5 && !model.is_step_completed(model.current_step),
                        #[watch]
                        set_sensitive: model.installing_version.is_none(),
                        set_tooltip_text: Some("Skip this step and proceed to the next section"),
                        connect_clicked => SetupInput::NextStep,
                    },
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::SlideLeftRight,
                    set_margin_start: 24,
                    set_margin_end: 24,
                    set_margin_top: 16,
                    set_margin_bottom: 16,

                    // ── STEP 0: Welcome ──────────────────────────────────────
                    add_named[Some("step0")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 24,
                        set_valign: gtk::Align::Center,

                        gtk::Image {
                            set_icon_name: Some("applications-system-symbolic"),
                            set_pixel_size: 72,
                            set_css_classes: &["accent"],
                        },

                        gtk::Label {
                            set_markup: "<span size='xx-large' weight='bold'>Welcome to Obelisk</span>",
                            set_halign: gtk::Align::Center,
                        },

                        gtk::Label {
                            set_label: "Let's perform a brief initial setup to prepare the launcher. This will verify your workspace directories, authenticators, and Java engines so everything is ready for play.",
                            set_wrap: true,
                            set_justify: gtk::Justification::Center,
                            set_max_width_chars: 50,
                            set_css_classes: &["dim-label"],
                        },
                    },

                    // ── STEP 1: Workspace Folder ──────────────────────────────
                    add_named[Some("step1")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 24,
                        set_valign: gtk::Align::Center,

                        // Compact Custom Header Box
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_halign: gtk::Align::Center,

                            gtk::Image {
                                set_icon_name: Some("folder-open-symbolic"),
                                set_pixel_size: 48,
                                set_css_classes: &["accent"],
                            },

                            gtk::Label {
                                set_markup: "<span size='large' weight='bold'>Workspace Folder</span>",
                                set_halign: gtk::Align::Center,
                            },

                            gtk::Label {
                                set_label: "Choose where Obelisk will save your Minecraft game configurations, modpacks, assets, and play worlds.",
                                set_wrap: true,
                                set_justify: gtk::Justification::Center,
                                set_max_width_chars: 50,
                                set_css_classes: &["dim-label"],
                            }
                        },

                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,

                            adw::ActionRow {
                                #[watch]
                                set_title: &if let Some(ref path) = model.config.instances_path {
                                    format!("Selected: {}", path.display())
                                } else {
                                    "No directory selected".to_string()
                                },
                                #[watch]
                                set_subtitle: if model.config.instances_path.is_some() {
                                    "Your instances directory is valid and active."
                                } else {
                                    "Please select an isolated folder for your instances."
                                },
                                add_prefix = &gtk::Image {
                                    #[watch]
                                    set_icon_name: Some(if model.config.instances_path.is_some() { "object-select-symbolic" } else { "dialog-warning-symbolic" }),
                                    #[watch]
                                    set_css_classes: if model.config.instances_path.is_some() { &["success"] } else { &["warning"] },
                                },
                                add_suffix = &gtk::Button {
                                    set_label: "Browse Folder",
                                    set_valign: gtk::Align::Center,
                                    set_tooltip_text: Some("Browse local directories to select a workspace folder"),
                                    connect_clicked[root, sender] => move |_| {
                                        let _ = &root;
                                        let dialog = gtk::FileDialog::builder()
                                            .title("Select Minecraft Instance Folder")
                                            .build();
                                        let sender_clone = sender.clone();
                                        dialog.select_folder(None::<&gtk::Window>, None::<&gtk::gio::Cancellable>, move |res| {
                                            if let Ok(file) = res {
                                                if let Some(path) = file.path() {
                                                    sender_clone.input(SetupInput::SetInstancesPath(path));
                                                }
                                            }
                                        });
                                    }
                                }
                            }
                        },

                        gtk::Label {
                            set_label: "Information: Choosing a fast SSD directory will drastically speed up game load times and assets extraction processes.",
                            set_wrap: true,
                            set_max_width_chars: 50,
                            set_css_classes: &["dim-label"],
                            set_justify: gtk::Justification::Center,
                        }
                    },

                    // ── STEP 2: Microsoft Client ID ───────────────────────────
                    add_named[Some("step2")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_valign: gtk::Align::Center,

                        // Compact Custom Header Box
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_halign: gtk::Align::Center,

                            gtk::Image {
                                set_icon_name: Some("preferences-other-symbolic"),
                                set_pixel_size: 48,
                                set_css_classes: &["accent"],
                            },

                            gtk::Label {
                                set_markup: "<span size='large' weight='bold'>Azure Client ID</span>",
                                set_halign: gtk::Align::Center,
                            },

                            gtk::Label {
                                set_label: "Obelisk relies on Microsoft OAuth to authenticate official profiles. To guarantee ultimate sovereignty and security, you can input a personal Azure Application Client ID.",
                                set_wrap: true,
                                set_justify: gtk::Justification::Center,
                                set_max_width_chars: 50,
                                set_css_classes: &["dim-label"],
                            }
                        },

                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,

                            adw::EntryRow {
                                set_title: "Client ID",
                                set_show_apply_button: true,
                                #[track = "model.client_id_changed"]
                                set_text: &model.client_id_input,
                                connect_apply[sender] => move |entry| {
                                    sender.input(SetupInput::SetClientId(entry.text().to_string()));
                                }
                            }
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_margin_start: 8,
                            set_margin_end: 8,

                            gtk::Label {
                                set_markup: "<span size='small' weight='semibold'>Why should I configure a Client ID?</span>",
                                set_halign: gtk::Align::Start,
                            },
                            gtk::Label {
                                set_markup: "Microsoft authentication requires registering a Client ID. You can easily generate a free personal ID by navigating to the <b>Microsoft Azure Portal</b>, registering an application as a Mobile/Desktop client, and registering the redirect URI to <tt>https://login.live.com/oauth20_desktop.srf</tt>. If you plan on playing purely offline mode, you can skip this step.",
                                set_wrap: true,
                                set_max_width_chars: 60,
                                set_css_classes: &["dim-label"],
                            }
                        }
                    },

                    // ── STEP 3: Minecraft Account ─────────────────────────────
                    add_named[Some("step3")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 16,
                        set_valign: gtk::Align::Center,

                        // Compact Custom Header Box
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_halign: gtk::Align::Center,

                            gtk::Image {
                                set_icon_name: Some("avatar-default-symbolic"),
                                set_pixel_size: 48,
                                set_css_classes: &["accent"],
                            },

                            gtk::Label {
                                set_markup: "<span size='large' weight='bold'>Configure Profile</span>",
                                set_halign: gtk::Align::Center,
                            },

                            gtk::Label {
                                set_label: "Configure an account profile to identify you when launching Minecraft games. Choose Microsoft accounts for multiplayer servers, or Offline accounts for local networks.",
                                set_wrap: true,
                                set_justify: gtk::Justification::Center,
                                set_max_width_chars: 50,
                                set_css_classes: &["dim-label"],
                            }
                        },

                        // Configured Accounts
                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,
                            #[watch]
                            set_visible: !model.config.accounts.is_empty(),

                            adw::ActionRow {
                                set_title: "Active Profile",
                                #[watch]
                                set_subtitle: &model.config.accounts.iter()
                                    .map(|a| format!("{} ({:?})", a.username, a.account_type))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                add_prefix = &gtk::Image::from_icon_name("object-select-symbolic") {
                                    set_css_classes: &["success"],
                                }
                            }
                        },

                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_spacing: 12,
                            set_halign: gtk::Align::Center,

                            // Microsoft Authenticate
                            gtk::Button {
                                set_label: "Sign in with Microsoft",
                                set_css_classes: &["suggested-action", "pill"],
                                set_icon_name: "web-browser-symbolic",
                                #[watch]
                                set_sensitive: model.config.microsoft_client_id.is_some() && !model.config.microsoft_client_id.as_ref().unwrap().trim().is_empty(),
                                set_tooltip_text: Some("Sign in with your official Microsoft/Minecraft account"),
                                connect_clicked => SetupInput::StartMicrosoftLogin,
                            },

                            // Offline Account setup inside popover
                            gtk::MenuButton {
                                set_label: "Add Offline Account",
                                set_css_classes: &["pill"],
                                set_icon_name: "network-offline-symbolic",
                                set_tooltip_text: Some("Configure a local offline profile for playing without a Microsoft account"),
                                #[wrap(Some)]
                                set_popover: offline_popover = &gtk::Popover {
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_spacing: 8,
                                        set_margin_all: 12,
                                        set_width_request: 220,

                                        gtk::Label {
                                            set_markup: "<b>Offline Profile Name</b>",
                                            set_halign: gtk::Align::Start,
                                        },

                                        gtk::Entry {
                                            set_placeholder_text: Some("Steve"),
                                            #[watch]
                                            set_text: &model.offline_username,
                                            connect_changed[sender] => move |entry| {
                                                sender.input(SetupInput::SetOfflineUsername(entry.text().to_string()));
                                            }
                                        },

                                        gtk::Button {
                                            set_label: "Create Offline Profile",
                                            set_css_classes: &["suggested-action"],
                                            set_tooltip_text: Some("Create and save the specified offline account profile"),
                                            connect_clicked[sender, offline_popover] => move |_| {
                                                sender.input(SetupInput::AddOfflineAccount);
                                                offline_popover.popdown();
                                            }
                                        }
                                    }
                                }
                            }
                        },

                        gtk::Label {
                            set_label: "Requirement: Microsoft accounts require you to set a Client ID in Step 2.",
                            #[watch]
                            set_visible: model.config.microsoft_client_id.is_none() || model.config.microsoft_client_id.as_ref().unwrap().trim().is_empty(),
                            set_css_classes: &["dim-label", "error"],
                            set_justify: gtk::Justification::Center,
                        }
                    },

                    // ── STEP 4: Java Runtime Environment ──────────────────────
                    add_named[Some("step4")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_valign: gtk::Align::Center,

                        // Compact Custom Header Box
                        gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_spacing: 8,
                            set_halign: gtk::Align::Center,

                            gtk::Image {
                                set_icon_name: Some("system-run-symbolic"),
                                set_pixel_size: 48,
                                set_css_classes: &["accent"],
                            },

                            gtk::Label {
                                set_markup: "<span size='large' weight='bold'>Verify Java Installations</span>",
                                set_halign: gtk::Align::Center,
                            },

                            gtk::Label {
                                set_label: "Minecraft requires specific Java versions based on standard release generations. Make sure you have the key engines installed below so you can launch any Minecraft instance seamlessly.",
                                set_wrap: true,
                                set_justify: gtk::Justification::Center,
                                set_max_width_chars: 50,
                                set_css_classes: &["dim-label"],
                            }
                        },

                        gtk::ListBox {
                            set_css_classes: &["boxed-list"],
                            set_selection_mode: gtk::SelectionMode::None,

                            // Java 8 Row
                            adw::ActionRow {
                                set_title: "Java 8",
                                #[watch]
                                set_subtitle: &model.get_java_status(8),

                                add_prefix = &gtk::Image {
                                    #[watch]
                                    set_icon_name: Some(if model.has_java_version(8) { "object-select-symbolic" } else { "dialog-warning-symbolic" }),
                                    #[watch]
                                    set_css_classes: if model.has_java_version(8) { &["success"] } else { &["warning"] },
                                    #[watch]
                                    set_visible: model.installing_version != Some(8),
                                },

                                add_prefix = &adw::Spinner {
                                    #[watch]
                                    set_visible: model.installing_version == Some(8) && model.java_install_progress < 0.0,
                                },

                                add_suffix = &gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 8,
                                    set_valign: gtk::Align::Center,

                                    // Inline Progress Bar
                                    gtk::ProgressBar {
                                        #[watch]
                                        set_visible: model.installing_version == Some(8) && model.java_install_progress >= 0.0,
                                        #[watch]
                                        set_fraction: model.java_install_progress as f64,
                                        set_width_request: 120,
                                    },

                                    // Cancel Button
                                    gtk::Button {
                                        set_icon_name: "window-close-symbolic",
                                        set_css_classes: &["flat", "circular", "destructive-action"],
                                        #[watch]
                                        set_visible: model.installing_version == Some(8),
                                        set_tooltip_text: Some("Cancel JRE 8 installation"),
                                        connect_clicked => SetupInput::CancelInstall,
                                    },

                                    // Install Button
                                    gtk::Button {
                                        set_label: "Install JRE 8",
                                        #[watch]
                                        set_visible: !model.has_java_version(8) && model.installing_version != Some(8),
                                        #[watch]
                                        set_sensitive: model.installing_version.is_none(),
                                        set_tooltip_text: Some("Download and isolate a managed JRE 8 build"),
                                        connect_clicked[root, sender] => move |_| {
                                            let _ = &root;
                                            sender.input(SetupInput::TriggerInstallPrompt(8));
                                        }
                                    }
                                }
                            },

                            // Java 17 Row
                            adw::ActionRow {
                                set_title: "Java 17",
                                #[watch]
                                set_subtitle: &model.get_java_status(17),

                                add_prefix = &gtk::Image {
                                    #[watch]
                                    set_icon_name: Some(if model.has_java_version(17) { "object-select-symbolic" } else { "dialog-warning-symbolic" }),
                                    #[watch]
                                    set_css_classes: if model.has_java_version(17) { &["success"] } else { &["warning"] },
                                    #[watch]
                                    set_visible: model.installing_version != Some(17),
                                },

                                add_prefix = &adw::Spinner {
                                    #[watch]
                                    set_visible: model.installing_version == Some(17) && model.java_install_progress < 0.0,
                                },

                                add_suffix = &gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 8,
                                    set_valign: gtk::Align::Center,

                                    gtk::ProgressBar {
                                        #[watch]
                                        set_visible: model.installing_version == Some(17) && model.java_install_progress >= 0.0,
                                        #[watch]
                                        set_fraction: model.java_install_progress as f64,
                                        set_width_request: 120,
                                        set_valign: gtk::Align::Center,
                                    },

                                    gtk::Button {
                                        set_icon_name: "window-close-symbolic",
                                        set_css_classes: &["flat", "circular", "destructive-action"],
                                        #[watch]
                                        set_visible: model.installing_version == Some(17),
                                        set_tooltip_text: Some("Cancel JRE 17 installation"),
                                        connect_clicked => SetupInput::CancelInstall,
                                    },

                                    gtk::Button {
                                        set_label: "Install JRE 17",
                                        #[watch]
                                        set_visible: !model.has_java_version(17) && model.installing_version != Some(17),
                                        #[watch]
                                        set_sensitive: model.installing_version.is_none(),
                                        set_tooltip_text: Some("Download and isolate a managed JRE 17 build"),
                                        connect_clicked[root, sender] => move |_| {
                                            let _ = &root;
                                            sender.input(SetupInput::TriggerInstallPrompt(17));
                                        }
                                    }
                                }
                            },

                            // Java 21 Row
                            adw::ActionRow {
                                set_title: "Java 21",
                                #[watch]
                                set_subtitle: &model.get_java_status(21),

                                add_prefix = &gtk::Image {
                                    #[watch]
                                    set_icon_name: Some(if model.has_java_version(21) { "object-select-symbolic" } else { "dialog-warning-symbolic" }),
                                    #[watch]
                                    set_css_classes: if model.has_java_version(21) { &["success"] } else { &["warning"] },
                                    #[watch]
                                    set_visible: model.installing_version != Some(21),
                                },

                                add_prefix = &adw::Spinner {
                                    #[watch]
                                    set_visible: model.installing_version == Some(21) && model.java_install_progress < 0.0,
                                },

                                add_suffix = &gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_spacing: 8,
                                    set_valign: gtk::Align::Center,

                                    gtk::ProgressBar {
                                        #[watch]
                                        set_visible: model.installing_version == Some(21) && model.java_install_progress >= 0.0,
                                        #[watch]
                                        set_fraction: model.java_install_progress as f64,
                                        set_width_request: 120,
                                    },

                                    gtk::Button {
                                        set_icon_name: "window-close-symbolic",
                                        set_css_classes: &["flat", "circular", "destructive-action"],
                                        #[watch]
                                        set_visible: model.installing_version == Some(21),
                                        set_tooltip_text: Some("Cancel JRE 21 installation"),
                                        connect_clicked => SetupInput::CancelInstall,
                                    },

                                    gtk::Button {
                                        set_label: "Install JRE 21",
                                        #[watch]
                                        set_visible: !model.has_java_version(21) && model.installing_version != Some(21),
                                        #[watch]
                                        set_sensitive: model.installing_version.is_none(),
                                        set_tooltip_text: Some("Download and isolate a managed JRE 21 build"),
                                        connect_clicked[root, sender] => move |_| {
                                            let _ = &root;
                                            sender.input(SetupInput::TriggerInstallPrompt(21));
                                        }
                                    }
                                }
                            }
                        },

                        gtk::Label {
                            set_label: "Hint: Obelisk downloads managed, secure JREs isolated inside the launcher's folder.",
                            set_wrap: true,
                            set_max_width_chars: 50,
                            set_css_classes: &["dim-label"],
                            set_justify: gtk::Justification::Center,
                        }
                    },

                    // ── STEP 5: Finished ──────────────────────────────────────
                    add_named[Some("step5")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 24,
                        set_valign: gtk::Align::Center,

                        gtk::Image {
                            set_icon_name: Some("object-select-symbolic"),
                            set_pixel_size: 72,
                            set_css_classes: &["success"],
                        },

                        gtk::Label {
                            set_markup: "<span size='xx-large' weight='bold'>Setup Completed!</span>",
                            set_halign: gtk::Align::Center,
                        },

                        gtk::Label {
                            set_markup: "<b>Next Steps:</b> Create your first instance in the using the <b>+ Add</b> button, configure your instance in the <b>Editor</b>, and hit <b>Play</b>! Obelisk Launcher is configured and fully primed to play Minecraft.",
                            set_wrap: true,
                            set_max_width_chars: 50,
                            set_justify: gtk::Justification::Center,
                        }
                    },

                    #[watch]
                    set_visible_child_name: match model.current_step {
                        0 => "step0",
                        1 => "step1",
                        2 => "step2",
                        3 => "step3",
                        4 => "step4",
                        5 => "step5",
                        _ => "step0",
                    }
                }
            }
        }
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let client_id = config.microsoft_client_id.clone().unwrap_or_default();

        let model = SetupDialog {
            visible: false,
            config,
            current_step: 0,
            client_id_input: client_id,
            client_id_changed: true,
            offline_username: String::new(),
            java_versions: Vec::new(),
            loading_java: false,
            all_packages: Vec::new(),
            loading_packages: false,
            installing_version: None,
            java_install_progress: -1.0,
            java_install_status: String::new(),
            cancel_flag: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.client_id_changed = false;
        match msg {
            SetupInput::Open => {
                self.visible = true;
                self.current_step = 0;
                self.offline_username.clear();
                self.installing_version = None;
                self.java_install_progress = -1.0;
                self.java_install_status.clear();

                sender.input(SetupInput::RefreshJava);

                if self.all_packages.is_empty() {
                    self.loading_packages = true;
                    let sender_clone = sender.input_sender().clone();
                    std::thread::spawn(move || {
                        let result = fetch_java_packages();
                        let _ = sender_clone.send(SetupInput::PackagesLoaded(result));
                    });
                }
            }
            SetupInput::Close => {
                self.visible = false;
                sender.input(SetupInput::CancelInstall);
            }
            SetupInput::UpdateConfig(config) => {
                self.config = config;
                let new_id = self.config.microsoft_client_id.clone().unwrap_or_default();
                if self.client_id_input != new_id {
                    self.client_id_input = new_id;
                    self.client_id_changed = true;
                }
            }
            SetupInput::SetStep(step) => {
                if step <= 5 {
                    self.current_step = step;
                }
            }
            SetupInput::NextStep => {
                if self.current_step < 5 {
                    self.current_step += 1;
                }
            }
            SetupInput::PrevStep => {
                if self.current_step > 0 {
                    self.current_step -= 1;
                }
            }
            SetupInput::SetInstancesPath(path) => {
                self.config.instances_path = Some(path);
                let _ = self.config.save();
                sender
                    .output(SetupOutput::ConfigUpdated(self.config.clone()))
                    .ok();
                sender.input(SetupInput::RefreshJava); // Refresh Java as path is now set
            }
            SetupInput::SetClientId(id) => {
                self.client_id_input = id.clone();
                let cleaned = id.trim().to_string();
                let new_val = if cleaned.is_empty() {
                    None
                } else {
                    Some(cleaned)
                };

                if self.config.microsoft_client_id != new_val {
                    self.config.microsoft_client_id = new_val;
                    let _ = self.config.save();
                    sender
                        .output(SetupOutput::ConfigUpdated(self.config.clone()))
                        .ok();
                }
            }
            SetupInput::SetOfflineUsername(name) => {
                self.offline_username = name;
            }
            SetupInput::AddOfflineAccount => {
                let name = self.offline_username.trim().to_string();
                if !name.is_empty() {
                    let account = create_offline_account(&name);
                    add_account(&mut self.config, account);
                    let _ = self.config.save();
                    sender
                        .output(SetupOutput::ConfigUpdated(self.config.clone()))
                        .ok();
                    self.offline_username.clear();
                }
            }
            SetupInput::StartMicrosoftLogin => {
                sender.output(SetupOutput::StartMicrosoftLogin).ok();
            }
            SetupInput::RefreshJava => {
                self.loading_java = true;
                let java_dir = self.config.minecraft_data_path.join("java");
                let sender_clone = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let versions = find_java_versions(Some(&java_dir));
                    let _ = sender_clone.send(SetupInput::SetJavaVersions(versions));
                });
            }
            SetupInput::SetJavaVersions(versions) => {
                self.loading_java = false;
                self.java_versions = versions;
            }
            SetupInput::PackagesLoaded(result) => {
                self.loading_packages = false;
                if let Ok(packages) = result {
                    self.all_packages = packages;
                }
            }
            SetupInput::TriggerInstallPrompt(major_version) => {
                if self.installing_version.is_some() {
                    return;
                }

                // Find matching package
                if let Some(package) = find_package_for_version(&self.all_packages, major_version) {
                    let size_formatted = format_size(package.size);
                    let heading = format!("Install Java {}?", major_version);
                    let body = format!(
                        "Do you want to download and isolate a dedicated Java runtime?\n\nPackage Details:\n• Distribution: {}\n• Version: {}\n• Architecture: {}\n• Type: JDK\n• Size: {}",
                        package.distribution, package.java_version, package.architecture, size_formatted
                    );

                    let dialog = adw::AlertDialog::builder()
                        .heading(heading)
                        .body(body)
                        .close_response("cancel")
                        .default_response("install")
                        .build();

                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("install", "Download & Install");
                    dialog.set_response_appearance("install", adw::ResponseAppearance::Suggested);

                    let sender_clone = sender.input_sender().clone();
                    if let Some(parent) = relm4::main_application().active_window() {
                        dialog.choose(&parent, None::<&gtk::gio::Cancellable>, move |res| {
                            if res == "install" {
                                let _ = sender_clone.send(SetupInput::InstallJava(major_version));
                            }
                        });
                    }
                } else {
                    // Packages haven't loaded or couldn't find one. Try standard refresh or alert
                    let dialog = adw::AlertDialog::builder()
                        .heading("Packages Loading")
                        .body("Java versions catalog is still downloading in the background. Please wait a few seconds and try again.")
                        .close_response("ok")
                        .default_response("ok")
                        .build();
                    dialog.add_response("ok", "OK");
                    if let Some(parent) = relm4::main_application().active_window() {
                        dialog.choose(&parent, None::<&gtk::gio::Cancellable>, |_| {});
                    }
                }
            }
            SetupInput::InstallJava(major_version) => {
                if self.installing_version.is_some() {
                    return;
                }

                if let Some(package) = find_package_for_version(&self.all_packages, major_version) {
                    self.installing_version = Some(major_version);
                    self.java_install_progress = -1.0;
                    self.java_install_status = format!("Initializing {}...", package.distribution);

                    let cancel_flag = Arc::new(AtomicBool::new(false));
                    self.cancel_flag = Some(cancel_flag.clone());

                    let sender_clone = sender.input_sender().clone();
                    let target_dir = self.config.minecraft_data_path.join("java");
                    let package_id = package.id.clone();

                    std::thread::spawn(move || {
                        download_and_extract_with_progress(
                            &package_id,
                            &target_dir,
                            cancel_flag,
                            move |progress| {
                                let _ = sender_clone
                                    .send(SetupInput::JavaProgress(major_version, progress));
                            },
                        );
                    });
                }
            }
            SetupInput::JavaProgress(major_version, progress) => {
                if self.installing_version != Some(major_version) {
                    return;
                }

                match progress {
                    JavaDownloadProgress::Downloading { current, total } => {
                        self.java_install_progress = if total > 0 {
                            current as f32 / total as f32
                        } else {
                            0.0
                        };
                        self.java_install_status = format!(
                            "Downloading JRE... ({:.1}%)",
                            self.java_install_progress * 100.0
                        );
                    }
                    JavaDownloadProgress::Extracting => {
                        self.java_install_progress = -1.0; // Show spinner
                        self.java_install_status = "Extracting files...".to_string();
                    }
                    JavaDownloadProgress::Finished(_) => {
                        self.installing_version = None;
                        self.java_install_progress = -1.0;
                        self.java_install_status.clear();
                        self.cancel_flag = None;
                        sender.input(SetupInput::RefreshJava);
                    }
                    JavaDownloadProgress::Error(e) => {
                        self.installing_version = None;
                        self.java_install_progress = -1.0;
                        self.java_install_status.clear();
                        self.cancel_flag = None;

                        let dialog = adw::AlertDialog::builder()
                            .heading("Installation Failed")
                            .body(format!("Could not install JRE {}:\n\n{}", major_version, e))
                            .close_response("ok")
                            .default_response("ok")
                            .build();
                        dialog.add_response("ok", "OK");
                        if let Some(parent) = relm4::main_application().active_window() {
                            dialog.choose(&parent, None::<&gtk::gio::Cancellable>, |_| {});
                        }
                    }
                }
            }
            SetupInput::CancelInstall => {
                if let Some(flag) = self.cancel_flag.take() {
                    flag.store(true, Ordering::Relaxed);
                }
                self.installing_version = None;
                self.java_install_progress = -1.0;
                self.java_install_status.clear();
            }
        }
    }
}
