use crate::backend::auth::account::{refresh_all_accounts, verify_account_status, AccountStatus};
use crate::backend::auth::microsoft::{Account, AccountType};
use crate::config::Config;
use crate::frontend::app::AppMsg;
use crate::frontend::dialogs::account_details::{AccountDetailsDialog, AccountDetailsInput};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

#[derive(Debug)]
pub struct AccountRow {
    account: Account,
    is_active: bool,
}

#[relm4::factory(pub)]
impl FactoryComponent for AccountRow {
    type Init = (Account, bool);
    type Input = ();
    type Output = AccountRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            set_title: &self.account.username,
            #[watch]
            set_subtitle: &self.get_subtitle(),
            #[watch]
            set_activatable: !self.is_active,

            connect_activated[sender, uuid = self.account.uuid.clone()] => move |_| {
                sender.output(AccountRowOutput::Switch(uuid.clone())).ok();
            },

            add_prefix = &gtk::Image {
                #[watch]
                set_icon_name: if self.is_active { Some("object-select-symbolic") } else { Some("avatar-default-symbolic") },
                #[watch]
                set_css_classes: if self.is_active { &["accent"] } else { &["dim-label"] },
            },

            add_suffix = &gtk::MenuButton {
                set_icon_name: "view-more-symbolic",
                set_css_classes: &["flat", "circular"],
                set_valign: gtk::Align::Center,
                #[wrap(Some)]
                #[name = "row_popover"]
                set_popover = &gtk::Popover {
                    set_autohide: true,
                    set_has_arrow: true,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_css_classes: &["menu-box"],
                        set_width_request: 160,
                        set_spacing: 4,

                        // Refresh Token (Microsoft accounts only)
                        gtk::Button {
                            #[watch]
                            set_visible: self.account.account_type == AccountType::Microsoft,
                            set_has_frame: false,
                            set_css_classes: &["flat", "menu-btn"],
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Refresh Token",
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Start,
                                },
                            },
                            connect_clicked[sender, uuid = self.account.uuid.clone(), row_popover] => move |_| {
                                row_popover.popdown();
                                sender.output(AccountRowOutput::Refresh(uuid.clone())).ok();
                            }
                        },

                        // Account Details / Info
                        gtk::Button {
                            set_has_frame: false,
                            set_css_classes: &["flat", "menu-btn"],
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Account Info",
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Start,
                                },
                            },
                            connect_clicked[sender, account = self.account.clone(), row_popover] => move |_| {
                                row_popover.popdown();
                                sender.output(AccountRowOutput::ShowDetails(account.clone())).ok();
                            }
                        },

                        gtk::Separator {
                            set_margin_top: 4,
                            set_margin_bottom: 4,
                        },

                        // Copy Username
                        gtk::Button {
                            set_has_frame: false,
                            set_css_classes: &["flat", "menu-btn"],
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Copy Username",
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Start,
                                },
                            },
                            connect_clicked[username = self.account.username.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&username);
                            }
                        },

                        // Copy UUID
                        gtk::Button {
                            set_has_frame: false,
                            set_css_classes: &["flat", "menu-btn"],
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Copy UUID",
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Start,
                                },
                            },
                            connect_clicked[uuid = self.account.uuid.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&uuid);
                            }
                        },

                        gtk::Separator {
                            set_margin_top: 4,
                            set_margin_bottom: 4,
                        },

                        // Remove
                        gtk::Button {
                            set_has_frame: false,
                            set_css_classes: &["flat", "menu-btn", "destructive-action"],
                            #[wrap(Some)]
                            set_child = &gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 12,
                                gtk::Label {
                                    set_label: "Remove Account",
                                    set_hexpand: true,
                                    set_halign: gtk::Align::Start,
                                },
                            },
                            connect_clicked[sender, uuid = self.account.uuid.clone(), row_popover] => move |_| {
                                row_popover.popdown();
                                sender.output(AccountRowOutput::Remove(uuid.clone())).ok();
                            }
                        },
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            account: init.0,
            is_active: init.1,
        }
    }
}

impl AccountRow {
    fn get_subtitle(&self) -> String {
        let type_str = match self.account.account_type {
            AccountType::Microsoft => "Microsoft",
            AccountType::Offline => "Offline",
        };
        let status = verify_account_status(&self.account);
        let status_str = match &status {
            AccountStatus::Valid => "Ready",
            AccountStatus::ExpiringSoon => "Expiring Soon",
            AccountStatus::Expired => "Expired",
            AccountStatus::Unknown(e) => e.as_str(),
        };
        format!("{} • {}", type_str, status_str)
    }
}

#[derive(Debug)]
pub enum AccountRowOutput {
    Switch(String),
    Remove(String),
    ShowDetails(Account),
    Refresh(String),
}

pub struct AccountView {
    config: Config,
    accounts: FactoryVecDeque<AccountRow>,
    details_dialog: Controller<AccountDetailsDialog>,
    visible: bool,
    refreshing: bool,
    refresh_message: String,
}

#[derive(Debug)]
pub enum AccountInput {
    UpdateConfig(Config),
    Open,
    ShowToast(String),
    SwitchAccount(String),
    RemoveAccount(String),
    ShowDetails(Account),
    RefreshAll,
    RefreshSingle(String),
    ResetRefreshing,
}

#[relm4::component(pub)]
impl Component for AccountView {
    type Init = Config;
    type Input = AccountInput;
    type Output = AppMsg;
    type CommandOutput = ();

    view! {
    adw::Bin {
        set_vexpand: true,

        #[wrap(Some)]
        set_child = &gtk::ScrolledWindow {
            set_vexpand: true,
            set_hscrollbar_policy: gtk::PolicyType::Never,

                adw::Clamp {
                    set_maximum_size: 1024,
                    set_tightening_threshold: 400,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_margin_all: 16,
                        set_spacing: 16,

                            // ── Active account card ──
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_css_classes: &["card"],

                                gtk::Box {
                                    set_orientation: gtk::Orientation::Horizontal,
                                    set_margin_all: 16,
                                    set_spacing: 16,

                                    gtk::Image {
                                        set_pixel_size: 40,
                                        #[watch]
                                        set_paintable: model.get_active_avatar().as_ref().map(|t| t as &gtk::gdk::Texture),
                                        #[watch]
                                        set_icon_name: if model.get_active_avatar().is_none() { Some("avatar-default-symbolic") } else { None },
                                        set_css_classes: &["accent"],
                                    },

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_hexpand: true,
                                        set_spacing: 2,
                                        set_valign: gtk::Align::Center,

                                        gtk::Label {
                                            set_label: "Active Account",
                                            set_css_classes: &["heading"],
                                            set_halign: gtk::Align::Start,
                                        },
                                        gtk::Label {
                                            set_css_classes: &["title-2"],
                                            set_halign: gtk::Align::Start,
                                            set_ellipsize: gtk::pango::EllipsizeMode::End,
                                            #[watch]
                                            set_label: &model.get_active_name(),
                                        },
                                        gtk::Label {
                                            set_css_classes: &["dim-label", "caption"],
                                            set_halign: gtk::Align::Start,
                                            #[watch]
                                            set_label: &model.get_active_subtitle(),
                                        },
                                    },
                                },
                            },

                            // ── Content area: Spinner or List ──
                            gtk::Stack {
                                add_named[Some("loading")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_halign: gtk::Align::Center,
                                    set_valign: gtk::Align::Center,
                                    set_spacing: 16,
                                    set_margin_top: 32,
                                    set_margin_bottom: 32,

                                    adw::Spinner {
                                        set_width_request: 48,
                                        set_height_request: 48,
                                    },

                                    gtk::Label {
                                        #[watch]
                                        set_label: &model.refresh_message,
                                        set_css_classes: &["dim-label"],
                                    }
                                },
                                add_named[Some("list")] = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 12,
                                    gtk::Label {
                                        set_label: "All Accounts",
                                        set_css_classes: &["heading"],
                                        set_halign: gtk::Align::Start,
                                        set_margin_start: 4,
                                        set_margin_top: 8,
                                    },

                                    #[local_ref]
                                    account_list -> gtk::ListBox {
                                        set_css_classes: &["boxed-list"],
                                        set_selection_mode: gtk::SelectionMode::None,
                                    }
                                },

                                add_named[Some("empty")] = &adw::StatusPage {
                                    set_title: "No Accounts Added",
                                    set_description: Some("Add a Microsoft or Offline Minecraft account to start playing."),
                                    set_icon_name: Some("avatar-default-symbolic"),
                                    set_vexpand: true,

                                    #[wrap(Some)]
                                    set_child = &gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_halign: gtk::Align::Center,
                                        set_spacing: 12,

                                        gtk::Button {
                                            set_label: "Add Microsoft Account",
                                            set_css_classes: &["suggested-action", "pill"],
                                            connect_clicked[sender] => move |_| {
                                                let _ = sender.output(AppMsg::LoginStart);
                                            }
                                        },

                                        gtk::Button {
                                            set_label: "Add Offline Account",
                                            set_css_classes: &["pill"],
                                            connect_clicked[sender] => move |_| {
                                                let _ = sender.output(AppMsg::ShowAddOfflineDialog);
                                            }
                                        },
                                    }
                                },

                                #[watch]
                                set_visible_child_name: if model.refreshing {
                                    "loading"
                                } else if model.config.accounts.is_empty() {
                                    "empty"
                                } else {
                                    "list"
                                },
                            }
                        }
                    }
                }
            }
    }

    fn init(
        config: Config,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let accounts = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| match output {
                AccountRowOutput::Switch(uuid) => AccountInput::SwitchAccount(uuid),
                AccountRowOutput::Remove(uuid) => AccountInput::RemoveAccount(uuid),
                AccountRowOutput::ShowDetails(acct) => AccountInput::ShowDetails(acct),
                AccountRowOutput::Refresh(uuid) => AccountInput::RefreshSingle(uuid),
            });

        let details_dialog = AccountDetailsDialog::builder().launch(()).detach();

        let mut model = AccountView {
            config,
            accounts,
            details_dialog,
            visible: false,
            refreshing: false,
            refresh_message: String::new(),
        };

        model.populate_accounts();
        let account_list = model.accounts.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            AccountInput::UpdateConfig(config) => {
                self.config = config;
                self.populate_accounts();
            }
            AccountInput::Open => {
                self.visible = !self.visible;
            }
            AccountInput::ShowToast(text) => {
                crate::frontend::toast::show_toast(root, text);
            }
            AccountInput::SwitchAccount(uuid) => {
                let _ = sender.output(AppMsg::SwitchAccount(uuid));
            }
            AccountInput::RemoveAccount(uuid) => {
                let _ = sender.output(AppMsg::RemoveAccount(uuid));
            }
            AccountInput::ShowDetails(account) => {
                self.details_dialog.emit(AccountDetailsInput::Show(account));
                if let Some(win) = root.ancestor(gtk::Window::static_type()) {
                    if let Ok(parent_window) = win.downcast::<gtk::Window>() {
                        self.details_dialog.widget().present(Some(&parent_window));
                    }
                }
            }
            AccountInput::RefreshAll => {
                self.refreshing = true;
                self.refresh_message = "Refreshing all accounts…".to_string();

                let mut config_clone = self.config.clone();
                let sender_out = sender.output_sender().clone();
                let sender_in = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let _ = refresh_all_accounts(&mut config_clone);
                    let _ = sender_out.send(AppMsg::RefreshAccountsAll(config_clone));
                    let _ = sender_in.send(AccountInput::ResetRefreshing);
                });
            }
            AccountInput::RefreshSingle(uuid) => {
                self.refreshing = true;
                self.refresh_message = "Refreshing account token…".to_string();

                let mut config_clone = self.config.clone();
                let sender_out = sender.output_sender().clone();
                let sender_in = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let client_id = config_clone
                        .microsoft_client_id
                        .clone()
                        .unwrap_or_else(|| "00000000402b5328".to_string());
                    if let Some(acct) = config_clone.accounts.iter().find(|a| a.uuid == uuid) {
                        if let Ok(refreshed) = crate::backend::auth::account::refresh_single_account(acct, &client_id) {
                            crate::backend::auth::account::add_account(&mut config_clone, refreshed);
                            let _ = sender_out.send(AppMsg::RefreshAccountsAll(config_clone));
                        }
                    }
                    let _ = sender_in.send(AccountInput::ResetRefreshing);
                });
            }
            AccountInput::ResetRefreshing => {
                self.refreshing = false;
            }
        }
    }
}

   impl AccountView {
    fn populate_accounts(&mut self) {
        self.refreshing = false;
        let mut guard = self.accounts.guard();
        guard.clear();
        let active_uuid = self.config.active_account_uuid.clone();
        for acct in &self.config.accounts {
            let is_active = Some(acct.uuid.clone()) == active_uuid;
            guard.push_back((acct.clone(), is_active));
        }
    }

    fn get_active_name(&self) -> String {
        let active_uuid = self.config.active_account_uuid.clone();
        if let Some(active) = self
            .config
            .accounts
            .iter()
            .find(|a| Some(a.uuid.clone()) == active_uuid)
        {
            active.username.clone()
        } else {
            "No active account".to_string()
        }
    }

    fn get_active_subtitle(&self) -> String {
        let active_uuid = self.config.active_account_uuid.clone();
        if let Some(active) = self
            .config
            .accounts
            .iter()
            .find(|a| Some(a.uuid.clone()) == active_uuid)
        {
            let type_str = match active.account_type {
                AccountType::Microsoft => "Microsoft Account",
                AccountType::Offline => "Offline Profile",
            };
            let status = verify_account_status(active);
            let status_str = match &status {
                AccountStatus::Valid => "Ready",
                AccountStatus::ExpiringSoon => "Expiring Soon",
                AccountStatus::Expired => "Expired",
                AccountStatus::Unknown(e) => e.as_str(),
            };
            format!("{} • {}", type_str, status_str)
        } else {
            "Select or add an account below".to_string()
        }
    }

    fn get_active_avatar(&self) -> Option<gtk::gdk::Texture> {
        let active_uuid = self.config.active_account_uuid.clone()?;
        let active_account = self.config.accounts.iter().find(|a| a.uuid == active_uuid)?;
        let cache_path = crate::backend::auth::avatar::get_avatar_cache_path(&active_account.uuid);
        if cache_path.exists() {
            gtk::gdk::Texture::from_filename(&cache_path).ok()
        } else {
            let uuid = active_account.uuid.clone();
            std::thread::spawn(move || {
                let _ = crate::backend::auth::avatar::fetch_and_cache_avatar(&uuid);
            });
            None
        }
    }
}
