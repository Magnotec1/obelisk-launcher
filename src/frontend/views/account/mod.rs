use crate::backend::auth::account::{
    refresh_all_accounts, verify_account_status,
    AccountStatus,
};
use crate::backend::auth::microsoft::{Account, AccountType};
use crate::config::Config;
use crate::frontend::app::AppMsg;
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
                set_icon_name: Some("object-select-symbolic"),
                #[watch]
                set_opacity: if self.is_active { 1.0 } else { 0.0 },
                set_css_classes: &["accent"],
            },

            add_suffix = &gtk::MenuButton {
                set_icon_name: "view-more-symbolic",
                set_css_classes: &["flat", "circular"],
                set_valign: gtk::Align::Center,
                #[wrap(Some)]
                #[name = "row_popover"]
                set_popover = &gtk::Popover {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_css_classes: &["menu-box"],

                        // Copy Username
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-copy-symbolic"),
                                gtk::Label::new(Some("Copy Username")),
                            },
                            connect_clicked[username = self.account.username.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&username);
                            }
                        },

                        // Copy UUID
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-copy-symbolic"),
                                gtk::Label::new(Some("Copy UUID")),
                            },
                            connect_clicked[uuid = self.account.uuid.clone(), row_popover] => move |btn| {
                                row_popover.popdown();
                                btn.display().clipboard().set_text(&uuid);
                            }
                        },

                        gtk::Separator { set_css_classes: &["menu-separator"] },

                        // Remove
                        gtk::Button {
                            set_css_classes: &["flat", "menu-btn", "error"],
                            gtk::Box {
                                set_spacing: 12,
                                gtk::Image::from_icon_name("edit-delete-symbolic"),
                                gtk::Label::new(Some("Remove Account")),
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
            AccountStatus::Valid => "● Ready",
            AccountStatus::ExpiringSoon => "◐ Expiring Soon",
            AccountStatus::Expired => "○ Expired",
            AccountStatus::Offline => "◇ Offline",
            AccountStatus::Unknown(e) => e.as_str(),
        };
        format!("{} · {}", type_str, status_str)
    }
}

#[derive(Debug)]
pub enum AccountRowOutput {
    Switch(String),
    Remove(String),
}

pub struct AccountView {
    config: Config,
    accounts: FactoryVecDeque<AccountRow>,
    visible: bool,
    refreshing: bool,
}

#[derive(Debug)]
pub enum AccountInput {
    UpdateConfig(Config),
    Open,
    ShowToast(String),
    SwitchAccount(String),
    RemoveAccount(String),
    RefreshAll,
    ResetRefreshing,
}

#[relm4::component(pub)]
impl Component for AccountView {
    type Init = Config;
    type Input = AccountInput;
    type Output = AppMsg;
    type CommandOutput = ();

    view! {
        adw::Window {
            set_title: Some("Account Manager"),
            set_default_width: 400,
            set_default_height: 500,
            set_modal: true,
            #[watch]
            set_visible: model.visible,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),

            #[wrap(Some)]
            #[name = "toast_overlay"]
            set_content = &adw::ToastOverlay {
                adw::ToolbarView {
                    add_top_bar = &adw::HeaderBar {
                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            set_title: "Account Manager",
                        },

                        // Refresh all accounts button - always visible, but disabled when refreshing
                        pack_start = &gtk::Button {
                            set_icon_name: "view-refresh-symbolic",
                            set_tooltip_text: Some("Refresh all accounts"),
                            set_css_classes: &["flat", "circular"],
                            #[watch]
                            set_sensitive: !model.refreshing,
                            connect_clicked[sender] => move |_| {
                                sender.input(AccountInput::RefreshAll);
                            }
                        },

                        // Compact "Add Account" button - always visible
                        pack_start = &gtk::MenuButton {
                            set_icon_name: "list-add-symbolic",
                            set_tooltip_text: Some("Add Account"),
                            set_css_classes: &["flat", "circular"],
                            #[watch]
                            set_sensitive: !model.refreshing,
                            #[wrap(Some)]
                            #[name = "add_popover"]
                            set_popover = &gtk::Popover {
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_css_classes: &["menu-box"],

                                    gtk::Button {
                                        set_css_classes: &["flat", "menu-btn"],
                                        gtk::Box {
                                            set_spacing: 12,
                                            gtk::Image::from_icon_name("web-browser-symbolic"),
                                            gtk::Label::new(Some("Microsoft Account")),
                                        },
                                        connect_clicked[sender, add_popover] => move |_| {
                                            add_popover.popdown();
                                            sender.output(AppMsg::LoginStart).ok();
                                        }
                                    },

                                    gtk::Button {
                                        set_css_classes: &["flat", "menu-btn"],
                                        gtk::Box {
                                            set_spacing: 12,
                                            gtk::Image::from_icon_name("network-offline-symbolic"),
                                            gtk::Label::new(Some("Offline Account")),
                                        },
                                        connect_clicked[sender, add_popover] => move |_| {
                                            add_popover.popdown();
                                            sender.output(AppMsg::ShowAddOfflineDialog).ok();
                                        }
                                    },
                                }
                            }
                        }
                    },

                    #[wrap(Some)]
                    set_content = &gtk::ScrolledWindow {
                        set_vexpand: true,
                        set_hscrollbar_policy: gtk::PolicyType::Never,

                        adw::Clamp {
                            set_maximum_size: 600,
                            set_tightening_threshold: 400,

                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,
                                set_margin_all: 16,
                                set_spacing: 10,

                                // ── Active account card ──
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_css_classes: &["card"],
                                    set_margin_start: 2,
                                    set_margin_end: 2,

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_margin_start: 16,
                                        set_margin_end: 16,
                                        set_margin_top: 16,
                                        set_margin_bottom: 8,
                                        set_spacing: 12,

                                        gtk::Image {
                                            set_icon_name: Some("avatar-default-symbolic"),
                                            set_pixel_size: 40,
                                            set_css_classes: &["dim-label"],
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
                                        }
                                    },

                                    gtk::Separator {},

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Horizontal,
                                        set_spacing: 8,
                                        set_margin_start: 16,
                                        set_margin_end: 16,
                                        set_margin_top: 10,
                                        set_margin_bottom: 10,

                                        gtk::Image {
                                            set_icon_name: Some("dialog-information-symbolic"),
                                            set_css_classes: &["dim-label"],
                                        },
                                        gtk::Label {
                                            set_css_classes: &["caption", "dim-label"],
                                            set_halign: gtk::Align::Start,
                                            set_hexpand: true,
                                            #[watch]
                                            set_label: &model.get_active_details(),
                                        }
                                    }
                                },

                                // ── Content area: Spinner or List ──
                                gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_spacing: 10,
                                    
                                    // Spinner is shown when refreshing
                                    adw::Spinner {
                                        #[watch]
                                        set_visible: model.refreshing,
                                        set_halign: gtk::Align::Center,
                                        set_margin_top: 30,
                                        set_margin_bottom: 30,
                                    },

                                    // Account list is hidden when refreshing
                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        #[watch]
                                        set_visible: !model.refreshing,

                                        #[local_ref]
                                        account_list -> gtk::ListBox {
                                            set_css_classes: &["boxed-list"],
                                            set_selection_mode: gtk::SelectionMode::None,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },

            connect_close_request[sender] => move |_| {
                sender.input(AccountInput::Open);
                gtk::glib::Propagation::Stop
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
            });

        let mut model = AccountView {
            config,
            accounts,
            visible: false,
            refreshing: false,
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
                let toast = adw::Toast::new(&text);
                if let Some(overlay) = self.find_toast_overlay(root.clone().upcast()) {
                    overlay.add_toast(toast);
                } else {
                    if let Some(win) = relm4::main_application().active_window() {
                        if let Some(overlay) = self.find_toast_overlay(win.upcast()) {
                            overlay.add_toast(toast);
                        }
                    }
                }
            }
            AccountInput::SwitchAccount(uuid) => {
                let _ = sender.output(AppMsg::SwitchAccount(uuid));
            }
            AccountInput::RemoveAccount(uuid) => {
                let _ = sender.output(AppMsg::RemoveAccount(uuid));
            }
            AccountInput::RefreshAll => {
                self.refreshing = true;
                sender.input(AccountInput::ShowToast("Refreshing all accounts…".to_string()));
                
                let mut config_clone = self.config.clone();
                let sender_out = sender.output_sender().clone();
                let sender_in = sender.input_sender().clone();
                std::thread::spawn(move || {
                    match refresh_all_accounts(&mut config_clone) {
                        Ok(_) => {
                            let _ = sender_in.send(AccountInput::ShowToast(
                                "All accounts refreshed".to_string(),
                            ));
                        }
                        Err(_) => {
                            let _ = sender_in.send(AccountInput::ShowToast(
                                "Some accounts failed to refresh".to_string(),
                            ));
                        }
                    }
                    let _ = sender_out.send(AppMsg::RefreshAccountsAll(config_clone));
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
    fn find_toast_overlay(&self, start: gtk::Widget) -> Option<adw::ToastOverlay> {
        if let Some(overlay) = start.downcast_ref::<adw::ToastOverlay>() {
            return Some(overlay.clone());
        }
        
        let mut child = start.first_child();
        while let Some(c) = child {
            if let Some(found) = self.find_toast_overlay(c.clone()) {
                return Some(found);
            }
            child = c.next_sibling();
        }
        None
    }

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

    fn get_active_details(&self) -> String {
        let active_uuid = self.config.active_account_uuid.clone();
        if let Some(active) = self
            .config
            .accounts
            .iter()
            .find(|a| Some(a.uuid.clone()) == active_uuid)
        {
            let type_str = match active.account_type {
                AccountType::Microsoft => "Microsoft",
                AccountType::Offline => "Offline",
            };
            let status = verify_account_status(active);
            format!("{} · {}", type_str, status)
        } else {
            "Select an account from the list below".to_string()
        }
    }
}
