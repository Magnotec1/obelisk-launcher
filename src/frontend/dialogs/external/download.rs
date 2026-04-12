use adw::prelude::*;
use relm4::prelude::*;

pub struct DownloadDialog {
    visible: bool,
    status: String,
    progress: f32,
    current_item: String,
    task_details: String,
}

#[derive(Debug)]
pub enum DownloadDialogInput {
    Show,
    Start,
    UpdateStatus(String, f32),
    UpdateDetailed {
        task: String,
        current: usize,
        total: usize,
        item_name: String,
        progress: f32,
    },
    Close,
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadDialog {
    type Init = ();
    type Input = DownloadDialogInput;
    type Output = ();

    view! {
        adw::Window {
            set_title: Some("Downloads"),
            set_default_width: 400,
            set_default_height: 250,
            set_modal: true,
            #[watch]
            set_transient_for: relm4::main_application().active_window().as_ref(),
            #[watch]
            set_visible: model.visible,
            connect_close_request[sender] => move |_| {
                sender.input(DownloadDialogInput::Close);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Download Status",
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_margin_all: 24,
                    set_valign: gtk::Align::Center,

                    gtk::Label {
                        #[watch]
                        set_label: &model.status,
                        set_css_classes: &["title-4"],
                        set_halign: gtk::Align::Start,
                    },

                    gtk::ProgressBar {
                        #[watch]
                        set_fraction: model.progress as f64,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.task_details,
                        set_css_classes: &["body", "dim-label"],
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.current_item,
                        set_css_classes: &["caption"],
                        set_halign: gtk::Align::Start,
                        set_ellipsize: gtk::pango::EllipsizeMode::End,
                    }
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DownloadDialog {
            visible: false,
            status: "Idle".to_string(),
            progress: 0.0,
            current_item: String::new(),
            task_details: String::new(),
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DownloadDialogInput::Show => self.visible = true,
            DownloadDialogInput::Start => {
                self.status = "Starting...".to_string();
                self.progress = 0.0;
                self.current_item.clear();
                self.task_details.clear();
            }
            DownloadDialogInput::Close => self.visible = false,
            DownloadDialogInput::UpdateStatus(status, progress) => {
                self.status = status;
                self.progress = progress;
                self.current_item.clear();
                self.task_details.clear();
            }
            DownloadDialogInput::UpdateDetailed {
                task,
                current,
                total,
                item_name,
                progress,
            } => {
                self.status = task;
                self.task_details = format!("{} / {}", current, total);
                self.current_item = item_name;
                self.progress = progress;
            }
        }
    }
}

pub struct DownloadStatusBar {
    pub status: String,
    pub progress: f32,
    pub visible: bool,
}

#[derive(Debug)]
pub enum DownloadStatusBarInput {
    Update(String, f32, bool),
}

#[derive(Debug)]
pub enum DownloadStatusBarOutput {
    Clicked,
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadStatusBar {
    type Init = ();
    type Input = DownloadStatusBarInput;
    type Output = DownloadStatusBarOutput;

    view! {
        gtk::Box {
            set_css_classes: &["clickable-bar-container"],
            set_hexpand: true,
            set_margin_start: 4,
            set_margin_end: 4,
            set_margin_bottom: 4,
            set_margin_top: 4,
            #[watch]
            set_visible: model.visible,
            
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 6,
                set_hexpand: true,
                set_css_classes: &["clickable-bar"],

                add_controller = gtk::GestureClick {
                    connect_released[sender] => move |_, _, _, _| {
                        sender.output(DownloadStatusBarOutput::Clicked).unwrap();
                    }
                },

                gtk::Label {
                    #[watch]
                    set_label: &model.status,
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["caption"],
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },
                gtk::ProgressBar {
                    #[watch]
                    set_fraction: model.progress as f64,
                    set_margin_bottom: 4,
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DownloadStatusBar {
            status: String::new(),
            progress: 0.0,
            visible: false,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DownloadStatusBarInput::Update(status, progress, visible) => {
                self.status = status;
                self.progress = progress;
                self.visible = visible;
            }
        }
    }
}
