use adw::prelude::*;
use relm4::prelude::*;
use relm4::factory::FactoryVecDeque;
use crate::backend::download::manager::{NetworkJob, NetworkJobStatus};

#[derive(Debug, Clone, PartialEq)]
pub enum DownloadState {
    Idle,
    Starting,
    Downloading {
        task: String,
        current: usize,
        total: usize,
        item_name: String,
        progress: f32,
    },
    Finished,
    Failed(String),
}

// ---------------------------------------------------------------------------
// 1. Factory Component for Row item in ListBox
// ---------------------------------------------------------------------------

pub struct DownloadJobRow {
    pub job: NetworkJob,
}

#[derive(Debug)]
pub enum DownloadJobRowInput {
    Update(NetworkJob),
}

#[derive(Debug)]
pub enum DownloadJobRowOutput {
    Remove(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for DownloadJobRow {
    type Init = NetworkJob;
    type Input = DownloadJobRowInput;
    type Output = DownloadJobRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ExpanderRow {
            #[watch]
            set_title: &self.job.title,
            #[watch]
            set_subtitle: &self.get_subtitle(),
            set_enable_expansion: true,

            add_prefix = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 4,
                set_valign: gtk::Align::Center,
                
                gtk::Image {
                    #[watch]
                    set_icon_name: Some(self.get_icon_name()),

                    #[watch]
                    set_visible: !matches!(self.job.status, NetworkJobStatus::Running { .. }),     
                },
                
                adw::Spinner {
                    #[watch]
                    set_visible: matches!(self.job.status, NetworkJobStatus::Running { .. }),          
                },
            },

            add_suffix = &gtk::Button {
                set_icon_name: "edit-delete-symbolic",
                set_css_classes: &["flat", "circular"],
                set_tooltip_text: Some("Remove"),
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: !matches!(self.job.status, NetworkJobStatus::Running { .. }),
                connect_clicked[sender, id = self.job.id.clone()] => move |_| {
                    sender.output(DownloadJobRowOutput::Remove(id.clone())).ok();
                }
            },

            add_row = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,
                set_margin_start: 12,
                set_margin_end: 12,

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_vscrollbar_policy: gtk::PolicyType::Automatic,
                    set_min_content_height: 200,
                    set_max_content_height: 200,
                    set_propagate_natural_height: true,

                    #[wrap(Some)]
                    set_child = &gtk::TextView {
                        set_editable: false,
                        set_cursor_visible: false,
                        set_wrap_mode: gtk::WrapMode::Word,
                        set_monospace: true,
                        set_css_classes: &["terminal-log-view"],
                        #[watch]
                        set_buffer: Some(&self.get_log_buffer()),
                    }
                }
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            job: init,
        }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            DownloadJobRowInput::Update(job) => {
                self.job = job;
            }
        }
    }
}

impl DownloadJobRow {
    fn get_subtitle(&self) -> String {
        match &self.job.status {
            NetworkJobStatus::Pending => "Queued • Pending".to_string(),
            NetworkJobStatus::Running { active_task_name, progress } => {
                format!("Running: {} ({:.0}%)", active_task_name, progress * 100.0)
            }
            NetworkJobStatus::Completed => "Completed successfully".to_string(),
            NetworkJobStatus::Failed(err) => format!("Failed: {}", err),
        }
    }

    fn get_icon_name(&self) -> &'static str {
        match &self.job.status {
            NetworkJobStatus::Pending => "media-playlist-consecutive-symbolic",
            NetworkJobStatus::Running { .. } => "",
            NetworkJobStatus::Completed => "object-select-symbolic",
            NetworkJobStatus::Failed(_) => "dialog-error-symbolic",
        }
    }

    fn get_log_buffer(&self) -> gtk::TextBuffer {
        let buffer = gtk::TextBuffer::new(None);
        if self.job.log.is_empty() {
            buffer.set_text("No logs recorded yet.");
        } else {
            buffer.set_text(&self.job.log.join("\n"));
        }
        // Scroll to end so the latest log entries are visible
        let end_iter = buffer.end_iter();
        buffer.place_cursor(&end_iter);
        buffer
    }
}

// ---------------------------------------------------------------------------
// 2. Main Downloads Dialog Component
// ---------------------------------------------------------------------------

pub struct DownloadDialog {
    visible: bool,
    job_rows: FactoryVecDeque<DownloadJobRow>,
    has_jobs: bool,
}

#[derive(Debug)]
pub enum DownloadDialogInput {
    Show,
    Start,
    Refresh,
    Close,
    RemoveRow(String),
    ClearFinished,
    UpdateState(DownloadState),
}

#[derive(Debug)]
pub enum DownloadDialogOutput {
    RemoveJob(String),
    ClearFinishedJobs,
}

#[relm4::component(pub)]
impl Component for DownloadDialog {
    type Init = ();
    type Input = DownloadDialogInput;
    type Output = DownloadDialogOutput;
    type CommandOutput = ();

    view! {
        adw::Dialog {
            set_title: "Downloads",
            set_content_width: 500,
            set_content_height: 500,
            set_can_close: true,

            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::WindowTitle {
                        set_title: "Download Manager",
                    },
                    
                    pack_end = &gtk::Button {
                        set_icon_name: "edit-clear-all-symbolic",
                        set_tooltip_text: Some("Clear Finished Downloads"),
                        set_css_classes: &["flat"],
                        connect_clicked[sender] => move |_| {
                            sender.input(DownloadDialogInput::ClearFinished);
                        }
                    }
                },

                #[wrap(Some)]
                set_content = &gtk::Stack {
                    set_transition_type: gtk::StackTransitionType::Crossfade,
                    set_transition_duration: 200,
                    set_vexpand: true,
                    set_hexpand: true,

                    add_named[Some("empty")] = &adw::StatusPage {
                        set_title: "No Downloads",
                        set_description: Some("Your active and historical downloads will show up here."),
                        set_icon_name: Some("system-file-manager-symbolic"),
                        set_vexpand: true,
                    },

                    add_named[Some("jobs")] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,
                        set_hexpand: true,
                        set_vexpand: true,

                        gtk::Label {
                            set_label: "Active & Historical Jobs",
                            set_css_classes: &["title-4"],
                            set_halign: gtk::Align::Start,
                        },

                        gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,
                            set_hexpand: true,
                            set_propagate_natural_height: true,
                            set_max_content_height: 340,

                            #[local_ref]
                            jobs_list -> gtk::ListBox {
                                set_selection_mode: gtk::SelectionMode::None,
                                set_css_classes: &["boxed-list"],
                                set_valign: gtk::Align::Start,
                            }
                        }
                    },

                    #[watch]
                    set_visible_child_name: if model.has_jobs { "jobs" } else { "empty" },
                },

                add_bottom_bar = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 12,
                    set_margin_bottom: 16,
                    set_margin_start: 16,
                    set_margin_end: 16,
                    set_halign: gtk::Align::Center,

                    gtk::Button {
                        set_label: "Close",
                        set_css_classes: &["pill"],
                        connect_clicked[sender] => move |_| {
                            sender.input(DownloadDialogInput::Close);
                        }
                    },
                }
            }
        }
    }

    fn init(
        _init: (), 
        root: Self::Root, 
        sender: ComponentSender<Self>
    ) -> ComponentParts<Self> {
        let job_rows = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| {
                match output {
                    DownloadJobRowOutput::Remove(id) => DownloadDialogInput::RemoveRow(id),
                }
            });

        let model = DownloadDialog {
            visible: false,
            job_rows,
            has_jobs: false,
        };

        let jobs_list = model.job_rows.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(
        &mut self, 
        msg: Self::Input, 
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            DownloadDialogInput::Show => {
                self.visible = true;
                sender.input(DownloadDialogInput::Refresh);
            }
            DownloadDialogInput::Start => {
                self.visible = true;
                sender.input(DownloadDialogInput::Refresh);
            }
            DownloadDialogInput::Close => {
                self.visible = false;
                root.close();
            }
            DownloadDialogInput::Refresh => {
                let jobs = crate::backend::download::manager::DOWNLOAD_QUEUE.get_jobs();
                
                let mut guard = self.job_rows.guard();
                
                // 1. Remove rows no longer in queue
                let mut to_remove = Vec::new();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get(i) {
                        if !jobs.iter().any(|j| j.id == row.job.id) {
                            to_remove.push(i);
                        }
                    }
                }
                for idx in to_remove.into_iter().rev() {
                    guard.remove(idx);
                }

                // 2. Add or update rows
                for job in jobs {
                    let mut found = false;
                    for i in 0..guard.len() {
                        if let Some(row) = guard.get_mut(i) {
                            if row.job.id == job.id {
                                row.job = job.clone();
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        guard.push_back(job);
                    }
                }

                drop(guard);
                self.has_jobs = !self.job_rows.is_empty();
            }
            DownloadDialogInput::RemoveRow(id) => {
                sender.output(DownloadDialogOutput::RemoveJob(id)).ok();
            }
            DownloadDialogInput::ClearFinished => {
                sender.output(DownloadDialogOutput::ClearFinishedJobs).ok();
            }
            DownloadDialogInput::UpdateState(_) => {
                // Refresh list on progress/state update
                sender.input(DownloadDialogInput::Refresh);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Status Bar Component
// ---------------------------------------------------------------------------

pub struct DownloadStatusBar {
    pub state: DownloadState,
    pub visible: bool,
}

#[derive(Debug)]
pub enum DownloadStatusBarInput {
    Update(DownloadState, bool),
    Dismiss,
}

#[derive(Debug)]
pub enum DownloadStatusBarOutput {
    Clicked,
    Dismiss,
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
            set_visible: true, // Always visible as footer
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            gtk::Button {
                set_icon_name: "folder-download-symbolic",
                set_tooltip_text: Some("Open Download Queue"),
                set_valign: gtk::Align::Center,
                connect_clicked[sender] => move |_| {
                    sender.output(DownloadStatusBarOutput::Clicked).unwrap();
                }
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 2,
                set_hexpand: true,
                set_valign: gtk::Align::Center,

                gtk::Label {
                    #[watch]
                    set_label: &model.get_status_text(),
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["caption"],
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                },
                gtk::ProgressBar {
                    #[watch]
                    set_fraction: model.get_progress(),
                    #[watch]
                    set_visible: matches!(model.state, DownloadState::Starting | DownloadState::Downloading { .. }),
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = DownloadStatusBar {
            state: DownloadState::Idle,
            visible: true,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DownloadStatusBarInput::Update(state, _visible) => {
                self.state = state;
                self.visible = true;
            }
            DownloadStatusBarInput::Dismiss => {
                self.state = DownloadState::Idle;
                self.visible = true;
            }
        }
    }
}

impl DownloadStatusBar {
    fn get_status_text(&self) -> String {
        match &self.state {
            DownloadState::Idle => "Idle".to_string(),
            DownloadState::Starting => "Starting...".to_string(),
            DownloadState::Downloading { task, item_name, .. } => {
                if item_name.is_empty() {
                    task.clone()
                } else {
                    format!("{}: {}", task, item_name)
                }
            }
            DownloadState::Finished => "Download finished".to_string(),
            DownloadState::Failed(err) => format!("Error: {}", err),
        }
    }

    fn get_progress(&self) -> f64 {
        match &self.state {
            DownloadState::Idle => 0.0,
            DownloadState::Starting => 0.0,
            DownloadState::Downloading { progress, .. } => *progress as f64,
            DownloadState::Finished => 1.0,
            DownloadState::Failed(_) => 0.0,
        }
    }
}
