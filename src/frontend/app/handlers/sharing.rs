use crate::backend::instance::sharing::{
    export_instance, export_instance_to_zip, import_instance_from_zip, import_shared_instance,
    SharedInstance,
};
use crate::frontend::app::msg::AppMsg;
use crate::frontend::app::state::AppModel;
use crate::frontend::dialogs::instance::sharing::{ImportInput, ImportStep, SharerInput};
use crate::frontend::views::instance::SummaryInput;
use adw::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

impl AppModel {
    pub(crate) fn handle_share_instance(&mut self, idx: usize) {
        self.sharer_dialog.emit(SharerInput::Open(idx));
        self.sharer_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_generate_share_code(&mut self, sender: &ComponentSender<AppModel>, idx: usize) {
        if let Some(inst) = self.instances.get(idx) {
            let sender_clone = sender.input_sender().clone();
            let inst_clone = inst.clone();
            self.sharing_loading = true;
            std::thread::spawn(move || {
                match export_instance(&inst_clone) {
                    Ok(shared) => {
                        if let Ok(code) = shared.to_code() {
                            let _ = sender_clone.send(AppMsg::DisplayShareCode(code));
                        } else {
                            let _ = sender_clone.send(AppMsg::DownloadError(
                                "Failed to encode instance".to_string(),
                            ));
                        }
                    }
                    Err(e) => {
                        let _ = sender_clone
                            .send(AppMsg::DownloadError(format!("Export failed: {}", e)));
                    }
                }
                let _ = sender_clone.send(AppMsg::SetSharingLoading(
                    false,
                    String::new(),
                    String::new(),
                    false,
                ));
            });
        }
    }

    pub(crate) fn handle_export_zip(&mut self, sender: &ComponentSender<AppModel>, idx: usize, path: PathBuf) {
        if let Some(inst) = self.instances.get(idx) {
            let sender_clone = sender.input_sender().clone();
            let inst_clone = inst.clone();
            self.sharing_loading = true;
            std::thread::spawn(move || {
                let s_clone = sender_clone.clone();
                match export_instance_to_zip(&inst_clone, &path, move |p, s| {
                    let _ = s_clone.send(AppMsg::UpdateSharingProgress(p, s));
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        let _ = sender_clone.send(AppMsg::DownloadError(format!(
                            "Zip export failed: {}",
                            e
                        )));
                    }
                }
                let _ = sender_clone.send(AppMsg::SetSharingLoading(
                    false,
                    String::new(),
                    String::new(),
                    false,
                ));
            });
        }
    }

    pub(crate) fn handle_display_share_code(&mut self, code: String) {
        self.sharer_dialog.emit(SharerInput::Close);

        let entry = gtk::Entry::builder()
            .text(&code)
            .editable(false)
            .halign(gtk::Align::Center)
            .width_request(300)
            .css_classes(vec!["body".to_string()])
            .build();

        let dialog = adw::AlertDialog::builder()
            .heading("Instance Code Ready")
            .body("Share this code with friends to let them import your exact setup.")
            .extra_child(&entry)
            .close_response("close")
            .default_response("copy")
            .build();

        dialog.add_response("close", "Close");
        dialog.add_response("copy", "Copy to Clipboard");
        dialog.set_response_appearance("copy", adw::ResponseAppearance::Suggested);

        let code_clone = code.clone();
        dialog.choose(&self.window, None::<&gtk::gio::Cancellable>, move |resp| {
            if resp == "copy" {
                let display =
                    gtk::gdk::Display::default().expect("Could not get default display");
                let clipboard = display.clipboard();
                clipboard.set_text(&code_clone);
            }
        });

        entry.select_region(0, -1);
    }

    pub(crate) fn handle_import_request(&mut self) {
        self.import_dialog.emit(ImportInput::Open);
        self.import_dialog.widget().present(Some(&self.window));
    }

    pub(crate) fn handle_confirm_import_from_code(&mut self, sender: &ComponentSender<AppModel>, code: String) {
        if let Ok(shared) = SharedInstance::from_code(&code) {
            let folder_name = shared
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            if let Some(instances_path) = self.config.instances_path.clone() {
                let instance_dir = instances_path.join(&folder_name);
                if instance_dir.exists() {
                    let entry = gtk::Entry::builder()
                        .text(&shared.name)
                        .placeholder_text("New instance name")
                        .halign(gtk::Align::Center)
                        .width_request(300)
                        .build();

                    let dialog = adw::AlertDialog::builder()
                        .heading("Instance Already Exists")
                        .body(format!("A folder for '{}' already exists at your instances path. Please provide a new name for this import.", shared.name))
                        .extra_child(&entry)
                        .close_response("cancel")
                        .default_response("import")
                        .build();
                    dialog.add_response("cancel", "Cancel");
                    dialog.add_response("import", "Import with New Name");
                    dialog.set_response_appearance(
                        "import",
                        adw::ResponseAppearance::Suggested,
                    );

                    let sender_clone = sender.input_sender().clone();
                    let code_clone = code.clone();
                    dialog.choose(
                        self.import_dialog.widget(),
                        None::<&gtk::gio::Cancellable>,
                        move |resp| {
                            if resp == "import" {
                                let new_name = entry.text().to_string();
                                if !new_name.is_empty() {
                                    let _ = sender_clone.send(AppMsg::PerformImport(
                                        code_clone,
                                        Some(new_name),
                                    ));
                                } else {
                                    let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                                }
                            } else {
                                let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                            }
                        },
                    );
                    return;
                }
            }
        }
        sender.input(AppMsg::PerformImport(code, None));
    }

    pub(crate) fn handle_perform_import(
        &mut self,
        sender: &ComponentSender<AppModel>,
        code: String,
        new_name: Option<String>,
    ) {
        self.import_loading = true;
        if let Ok(mut shared) = SharedInstance::from_code(&code) {
            if let Some(name) = new_name {
                shared.name = name;
            }
            if let Some(instances_path) = self.config.instances_path.clone() {
                let sender_clone = sender.input_sender().clone();
                std::thread::spawn(move || {
                    let s_clone = sender_clone.clone();
                    match import_shared_instance(shared, &instances_path, move |s| {
                        let _ = s_clone.send(AppMsg::UpdateImportStatus(s));
                    }) {
                        Ok(_) => {
                            let _ = sender_clone.send(AppMsg::RefreshInstances);
                            let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                        }
                        Err(e) => {
                            let _ = sender_clone.send(AppMsg::DownloadError(format!(
                                "Import failed: {}",
                                e
                            )));
                            let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                        }
                    }
                });
            }
        } else {
            self.import_loading = false;
            crate::frontend::toast::show_toast(&self.window, "Invalid sharing code");
            self.import_dialog.emit(ImportInput::Close);
        }
    }

    pub(crate) fn handle_import_zip(&mut self, sender: &ComponentSender<AppModel>, path: PathBuf) {
        if let Some(instances_path) = self.config.instances_path.clone() {
            let sender_clone = sender.input_sender().clone();
            self.import_loading = true;
            self.import_dialog
                .emit(ImportInput::SetStep(ImportStep::Progress));
            std::thread::spawn(move || {
                let s_clone = sender_clone.clone();
                match import_instance_from_zip(&path, &instances_path, move |p, s| {
                    let _ = s_clone.send(AppMsg::UpdateImportStatus(format!(
                        "{:.0}% - {}",
                        p * 100.0,
                        s
                    )));
                }) {
                    Ok(_) => {
                        let _ = sender_clone.send(AppMsg::RefreshInstances);
                        let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                    }
                    Err(e) => {
                        let _ = sender_clone
                            .send(AppMsg::DownloadError(format!("Zip import failed: {}", e)));
                        let _ = sender_clone.send(AppMsg::SetImportLoading(false));
                    }
                }
            });
        }
    }

    pub(crate) fn handle_set_sharing_loading(
        &mut self,
        loading: bool,
        title: String,
        subtitle: String,
        show_progress: bool,
    ) {
        self.sharing_loading = loading;
        self.instance_summary
            .emit(SummaryInput::SetSharingLoading(loading));
        self.sharer_dialog.emit(SharerInput::SetLoading(
            loading,
            title,
            subtitle,
            show_progress,
        ));
        if !loading {
            self.sharer_dialog.emit(SharerInput::Close);
        }
    }

    pub(crate) fn handle_update_sharing_progress(&mut self, progress: f64, status: String) {
        self.sharer_dialog.emit(SharerInput::SetProgress(progress));
        self.sharer_dialog.emit(SharerInput::SetLoading(
            true,
            "Exporting Zip".to_string(),
            status,
            true,
        ));
    }

    pub(crate) fn handle_set_import_loading(&mut self, loading: bool) {
        self.import_loading = loading;
        if !loading {
            self.import_dialog.emit(ImportInput::SetLoading(false));
            self.import_dialog.emit(ImportInput::Close);
        }
    }

    pub(crate) fn handle_set_verifying_loading(&mut self, loading: bool) {
        self.verifying_loading = loading;
        self.instance_summary
            .emit(SummaryInput::SetVerifyingLoading(loading));
    }

    pub(crate) fn handle_update_import_status(&mut self, status: String) {
        self.import_dialog.emit(ImportInput::AddLog(status));
    }
}
