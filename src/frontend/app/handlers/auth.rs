use crate::backend::auth::account::{
    add_account, create_offline_account, remove_account, switch_account,
};
use crate::backend::auth::microsoft::{self as auth, Account};
use crate::backend::instance::manager::scan_instances;
use crate::config::Config;
use crate::frontend::app::msg::AppMsg;
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::instance::add::AddInstanceInput;
use crate::frontend::dialogs::system::setup::SetupInput;
use crate::frontend::views::account::AccountInput;
use crate::frontend::views::instance::{EditorTabInput, SettingsTabInput};
use crate::frontend::views::library::OverviewInput;
use crate::frontend::views::settings::SettingsInput;
use crate::frontend::views::sidebar::{SidebarInput, SidebarPage};
use adw::prelude::*;
use relm4::prelude::*;

impl AppModel {
    pub(crate) fn handle_account_action(&mut self) {
        self.active_sidebar_page = SidebarPage::Accounts;
        self.sidebar
            .emit(SidebarInput::SetSelected(SidebarPage::Accounts));
    }

    pub(crate) fn handle_login_start(&mut self, sender: &ComponentSender<AppModel>) {
        if let Some(client_id) = self.config.microsoft_client_id.clone() {
            self.auth_in_progress = true;
            let sender_clone = sender.input_sender().clone();

            crate::backend::core::tasks::spawn_io(move || {
                match auth::start_device_code_flow(&client_id) {
                    Ok(dc) => {
                        let _ = sender_clone.send(AppMsg::LoginDeviceCode(
                            dc.user_code.clone(),
                            dc.verification_uri.clone(),
                        ));

                        crate::frontend::utils::file::open_url(&dc.verification_uri);

                        match auth::poll_for_ms_token(
                            &client_id,
                            &dc.device_code,
                            dc.interval,
                            dc.expires_in,
                        ) {
                            Ok((access_token, refresh_token)) => {
                                match auth::complete_auth(&access_token, &refresh_token) {
                                    Ok(account) => {
                                        let _ = sender_clone
                                            .send(AppMsg::LoginResult(Ok(account)));
                                    }
                                    Err(e) => {
                                        let _ = sender_clone
                                            .send(AppMsg::LoginResult(Err(e.to_string())));
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = sender_clone.send(AppMsg::LoginResult(Err(e.to_string())));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender_clone.send(AppMsg::LoginResult(Err(e.to_string())));
                    }
                }
            });
        } else {
            let dialog = adw::AlertDialog::builder()
                .heading("No Client ID")
                .body("Please set your Microsoft Azure Client ID in the app settings before signing in.")
                .close_response("ok")
                .default_response("ok")
                .build();
            dialog.add_response("ok", "OK");
            dialog.choose(
                self.account_view.widget(),
                None::<&gtk::gio::Cancellable>,
                |_| {},
            );
        }
    }

    pub(crate) fn handle_login_device_code(&self, code: String, uri: String) {
        let entry = gtk::Entry::builder()
            .text(&code)
            .editable(false)
            .halign(gtk::Align::Center)
            .width_request(200)
            .css_classes(vec!["title-2".to_string()])
            .build();

        let dialog = adw::AlertDialog::builder()
            .heading("Sign in with Microsoft")
            .body(format!(
                "A browser window has been opened.\n\nGo to:\n{}\n\nAnd enter this code below:",
                uri
            ))
            .extra_child(&entry)
            .close_response("ok")
            .default_response("ok")
            .build();

        dialog.add_response("ok", "OK");
        dialog.choose(
            self.account_view.widget(),
            None::<&gtk::gio::Cancellable>,
            |_| {},
        );

        entry.select_region(0, -1);
    }

    pub(crate) fn handle_login_result(&mut self, result: Result<Account, String>) {
        self.auth_in_progress = false;
        match result {
            Ok(account) => {
                let name = account.username.clone();
                add_account(&mut self.config, account);
                let _ = self.config.save();

                self.account_view
                    .emit(AccountInput::UpdateConfig(self.config.clone()));
                self.setup_dialog
                    .emit(SetupInput::UpdateConfig(self.config.clone()));
                self.settings_dialog.emit(SettingsInput::RefreshJava);

                let dialog = adw::AlertDialog::builder()
                    .heading("Login Successful")
                    .body(format!("Welcome, {}!", name))
                    .close_response("ok")
                    .default_response("ok")
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(
                    self.account_view.widget(),
                    None::<&gtk::gio::Cancellable>,
                    |_| {},
                );
            }
            Err(err) => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Login Failed")
                    .body(&err)
                    .close_response("ok")
                    .default_response("ok")
                    .build();
                dialog.add_response("ok", "OK");
                dialog.choose(
                    self.account_view.widget(),
                    None::<&gtk::gio::Cancellable>,
                    |_| {},
                );
            }
        }
    }

    pub(crate) fn handle_logout(&mut self) {
        self.config.accounts.clear();
        self.config.active_account_uuid = None;
        let _ = self.config.save();
        self.account_view
            .emit(AccountInput::UpdateConfig(self.config.clone()));
    }

    pub(crate) fn handle_switch_account(&mut self, uuid: String) {
        if switch_account(&mut self.config, &uuid).is_ok() {
            let _ = self.config.save();
            self.account_view
                .emit(AccountInput::UpdateConfig(self.config.clone()));
            crate::frontend::toast::show_toast(&self.window, "Switched account");
        }
    }

    pub(crate) fn handle_remove_account(&mut self, uuid: String) {
        remove_account(&mut self.config, &uuid);
        let _ = self.config.save();
        self.account_view
            .emit(AccountInput::UpdateConfig(self.config.clone()));
        crate::frontend::toast::show_toast(&self.window, "Account removed");
    }

    pub(crate) fn handle_add_offline_account(&mut self, username: String) {
        let account = create_offline_account(&username);
        let name = account.username.clone();
        add_account(&mut self.config, account);
        let _ = self.config.save();
        self.account_view
            .emit(AccountInput::UpdateConfig(self.config.clone()));
        crate::frontend::toast::show_toast(
            &self.window,
            format!("Added offline account: {}", name),
        );
    }

    pub(crate) fn handle_refresh_account_result(&mut self, result: Result<Account, String>) {
        if let Ok(account) = result {
            add_account(&mut self.config, account);
            let _ = self.config.save();
            self.account_view
                .emit(AccountInput::UpdateConfig(self.config.clone()));
        }
    }

    pub(crate) fn handle_refresh_accounts_request(&self) {
        self.account_view.emit(AccountInput::RefreshAll);
    }

    pub(crate) fn handle_refresh_accounts_all(&mut self, new_config: Config) {
        self.config.accounts = new_config.accounts;
        let _ = self.config.save();
        self.account_view
            .emit(AccountInput::UpdateConfig(self.config.clone()));
    }

    pub(crate) fn handle_show_add_offline_dialog(&self, sender: &ComponentSender<AppModel>) {
        let dialog = adw::AlertDialog::builder()
            .heading("Add Offline Account")
            .body("Enter a username for offline play.")
            .close_response("cancel")
            .default_response("add")
            .build();

        dialog.add_response("cancel", "Cancel");
        dialog.add_response("add", "Add Account");
        dialog.set_response_appearance("add", adw::ResponseAppearance::Suggested);

        let entry = adw::EntryRow::builder().title("Username").build();

        let clamp = adw::Clamp::new();
        clamp.set_maximum_size(400);
        clamp.set_margin_start(12);
        clamp.set_margin_end(12);

        let list = gtk::ListBox::new();
        list.set_css_classes(&["boxed-list"]);
        list.set_selection_mode(gtk::SelectionMode::None);
        list.append(&entry);
        clamp.set_child(Some(&list));

        dialog.set_extra_child(Some(&clamp));

        let sender_clone = sender.input_sender().clone();
        let entry_clone = entry.clone();
        dialog.choose(
            self.account_view.widget(),
            None::<&gtk::gio::Cancellable>,
            move |response| {
                if response == "add" {
                    let username = entry_clone.text().to_string();
                    let username = username.trim().to_string();
                    if !username.is_empty() {
                        let _ = sender_clone.send(AppMsg::AddOfflineAccount(username));
                    }
                }
            },
        );
    }

    pub(crate) fn handle_open_account_settings(&mut self) {
        self.settings_dialog
            .emit(SettingsInput::UpdateConfig(self.config.clone()));
        self.settings_dialog
            .emit(SettingsInput::SetPage("accounts".to_string()));
        self.settings_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_config_updated(
        &mut self,
        sender: &ComponentSender<AppModel>,
        new_config: Config,
    ) {
        self.config = new_config.clone();
        self.setup_dialog
            .emit(SetupInput::UpdateConfig(new_config.clone()));
        self.account_view
            .emit(AccountInput::UpdateConfig(new_config.clone()));
        self.instance_editor_tab.emit(EditorTabInput::Update(
            self.selected_instance
                .and_then(|i| self.instances.get(i).cloned()),
            new_config.clone(),
        ));
        self.instance_settings_tab.emit(SettingsTabInput::Update(
            Box::new(
                self.selected_instance
                    .and_then(|i| self.instances.get(i).cloned()),
            ),
            Box::new(new_config.clone()),
        ));

        self.add_instance_dialog
            .emit(AddInstanceInput::UpdateInstancesPath(
                self.config.instances_path.clone(),
            ));

        if let Some(path) = &self.config.instances_path {
            let path_clone = path.clone();
            let sender_clone = sender.input_sender().clone();
            self.overview_grid.emit(OverviewInput::SetLoading(true));
            self.overview_grid.emit(OverviewInput::GoBack);
            crate::backend::core::tasks::spawn_io(move || {
                let insts = scan_instances(&path_clone);
                let _ = sender_clone.send(AppMsg::InstancesUpdated(insts));
            });
        } else {
            self.instances.clear();
            self.selected_instance = None;
            self.rebuild_overview();
        }
    }
}
