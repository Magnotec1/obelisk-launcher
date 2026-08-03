use crate::backend::download::manager::{NetworkJob, NetworkJobStatus, TaskItemStatus, DownloadedItemType, TaskItemDetail};
use adw::prelude::*;
use relm4::factory::FactoryVecDeque;
use relm4::prelude::*;

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
    ShowDetails(String),
}

#[relm4::factory(pub)]
impl FactoryComponent for DownloadJobRow {
    type Init = NetworkJob;
    type Input = DownloadJobRowInput;
    type Output = DownloadJobRowOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            #[watch]
            set_title: &self.job.title,
            #[watch]
            set_subtitle: &self.get_subtitle(),
            set_activatable: true,
            connect_activated[sender, id = self.job.id.clone()] => move |_| {
                sender.output(DownloadJobRowOutput::ShowDetails(id.clone())).ok();
            },

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
                set_icon_name: "user-trash-symbolic",
                set_css_classes: &["flat", "circular"],
                set_tooltip_text: Some("Remove"),
                set_valign: gtk::Align::Center,
                #[watch]
                set_visible: !matches!(self.job.status, NetworkJobStatus::Running { .. }),
                connect_clicked[sender, id = self.job.id.clone()] => move |_| {
                    sender.output(DownloadJobRowOutput::Remove(id.clone())).ok();
                }
            },
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { job: init }
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
            NetworkJobStatus::Running {
                active_task_name,
                progress,
            } => {
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
}

// ---------------------------------------------------------------------------
// 2. Download Job Details Dialog Component
// ---------------------------------------------------------------------------

pub struct DownloadedItemRow {
    pub item: TaskItemDetail,
}

#[derive(Debug)]
pub enum DownloadedItemRowInput {
    Update(TaskItemDetail),
}

#[relm4::factory(pub)]
impl FactoryComponent for DownloadedItemRow {
    type Init = TaskItemDetail;
    type Input = DownloadedItemRowInput;
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        adw::ActionRow {
            #[watch]
            set_title: &self.item.name,
            #[watch]
            set_subtitle: &self.get_subtitle(),
            
            add_prefix = &gtk::Image {
                #[watch]
                set_icon_name: Some(self.get_icon_name()),
                #[watch]
                set_css_classes: &[self.get_icon_css_class()],
            },
            
            add_suffix = &gtk::Label {
                set_text: self.get_type_label(),
                set_css_classes: &["dim-label"],
                set_valign: gtk::Align::Center,
            }
        }
    }

    fn init_model(init: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { item: init }
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            DownloadedItemRowInput::Update(item) => {
                self.item = item;
            }
        }
    }
}

impl DownloadedItemRow {
    fn get_subtitle(&self) -> String {
        match &self.item.status {
            TaskItemStatus::Failed(e) => format!("Error: {}", e),
            TaskItemStatus::Success => "Completed".to_string(),
            TaskItemStatus::Pending => "Pending...".to_string(),
            TaskItemStatus::Running(info) => info.clone(),
        }
    }

    fn get_icon_name(&self) -> &'static str {
        match &self.item.status {
            TaskItemStatus::Success => "object-select-symbolic",
            TaskItemStatus::Failed(_) => "dialog-error-symbolic",
            TaskItemStatus::Pending | TaskItemStatus::Running(_) => "media-playlist-consecutive-symbolic",
        }
    }

    fn get_icon_css_class(&self) -> &'static str {
        match &self.item.status {
            TaskItemStatus::Success => "success",
            TaskItemStatus::Failed(_) => "error",
            TaskItemStatus::Pending | TaskItemStatus::Running(_) => "dim-label",
        }
    }

    fn get_type_label(&self) -> &'static str {
        match &self.item.item_type {
            DownloadedItemType::Mod => "Mod",
            DownloadedItemType::ResourcePack => "Resource Pack",
            DownloadedItemType::ShaderPack => "Shader Pack",
            DownloadedItemType::World => "World",
            DownloadedItemType::MinecraftComponent => "Component",
            DownloadedItemType::Java => "Java",
        }
    }
}

pub struct DownloadJobDetailsDialog {
    pub job: Option<NetworkJob>,
    pub items: FactoryVecDeque<DownloadedItemRow>,
    pub log_buffer: gtk::TextBuffer,
}

#[derive(Debug)]
pub enum DownloadDetailsInput {
    Open(NetworkJob),
    Update(NetworkJob),
    Retry,
}

#[derive(Debug)]
pub enum DownloadDetailsOutput {
    Retry(String),
}

#[relm4::component(pub)]
impl SimpleComponent for DownloadJobDetailsDialog {
    type Init = ();
    type Input = DownloadDetailsInput;
    type Output = DownloadDetailsOutput;

    view! {
        adw::Dialog {
            #[watch]
            set_title: &model.get_title(),
            set_content_width: 520,
            set_content_height: 480,
            set_can_close: true,
            
            #[wrap(Some)]
            set_child = &adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &adw::ViewSwitcher {
                        #[watch]
                        set_stack: Some(&details_stack),
                    },
                },
                
                #[wrap(Some)]
                #[name = "details_stack"]
                set_content = &adw::ViewStack {
                    set_vexpand: true,
                    
                    add_titled_with_icon[Some("summary"), "Summary", "format-justify-left-symbolic"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,
                        
                        gtk::Label {
                            #[watch]
                            set_label: &model.get_status_text(),
                            set_css_classes: &["title-4", "bold"],
                            set_halign: gtk::Align::Start,
                        },
                        
                        gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,
                            set_propagate_natural_height: true,
                            set_max_content_height: 320,
                            
                            #[local_ref]
                            items_list -> gtk::ListBox {
                                set_selection_mode: gtk::SelectionMode::None,
                                set_css_classes: &["boxed-list"],
                                set_valign: gtk::Align::Start,
                            }
                        }
                    },
                    
                    add_titled_with_icon[Some("log"), "Terminal Log", "utilities-terminal-symbolic"] = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 12,
                        set_margin_all: 16,
                        
                        gtk::ScrolledWindow {
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,
                            set_vexpand: true,
                            
                            #[wrap(Some)]
                            set_child = &gtk::TextView {
                                set_editable: false,
                                set_cursor_visible: false,
                                set_wrap_mode: gtk::WrapMode::Word,
                                set_monospace: true,
                                set_css_classes: &["terminal-log-view"],
                                set_buffer: Some(&model.log_buffer),
                            }
                        }
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
                        set_label: "Close",
                        set_css_classes: &["pill"],
                        connect_clicked[root] => move |_| {
                            root.close();
                        }
                    },
                    
                    gtk::Button {
                        #[watch]
                        set_visible: model.is_failed(),
                        set_label: "Retry Failed",
                        set_css_classes: &["pill", "suggested-action"],
                        connect_clicked[sender] => move |_| {
                            sender.input(DownloadDetailsInput::Retry);
                        }
                    }
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let items = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |_| unreachable!());

        let log_buffer = gtk::TextBuffer::new(None);

        let model = DownloadJobDetailsDialog {
            job: None,
            items,
            log_buffer,
        };

        let items_list = model.items.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DownloadDetailsInput::Open(job) | DownloadDetailsInput::Update(job) => {
                let items_data = job.items.clone();
                self.job = Some(job.clone());

                if job.log.is_empty() {
                    self.log_buffer.set_text("No logs recorded yet.");
                } else {
                    self.log_buffer.set_text(&job.log.join("\n"));
                }
                let end_iter = self.log_buffer.end_iter();
                self.log_buffer.place_cursor(&end_iter);

                let mut guard = self.items.guard();
                let mut to_remove = Vec::new();
                for i in 0..guard.len() {
                    if let Some(row) = guard.get(i) {
                        if !items_data.iter().any(|item| item.name == row.item.name) {
                            to_remove.push(i);
                        }
                    }
                }
                for idx in to_remove.into_iter().rev() {
                    guard.remove(idx);
                }

                let mut updates = Vec::new();
                for item in items_data {
                    let mut found = false;
                    for i in 0..guard.len() {
                        if let Some(row) = guard.get_mut(i) {
                            if row.item.name == item.name {
                                row.item = item.clone();
                                updates.push((i, item.clone()));
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        guard.push_back(item.clone());
                    }
                }
                drop(guard);

                for (i, item) in updates {
                    self.items.send(i, DownloadedItemRowInput::Update(item));
                }
            }
            DownloadDetailsInput::Retry => {
                if let Some(ref j) = self.job {
                    sender.output(DownloadDetailsOutput::Retry(j.id.clone())).ok();
                }
            }
        }
    }
}

impl DownloadJobDetailsDialog {
    fn get_title(&self) -> String {
        match &self.job {
            Some(j) => format!("Details: {}", j.title),
            None => "Job Details".to_string(),
        }
    }

    fn get_status_text(&self) -> String {
        match &self.job {
            Some(j) => match &j.status {
                NetworkJobStatus::Pending => "Status: Pending in queue".to_string(),
                NetworkJobStatus::Running { active_task_name, .. } => format!("Status: Running ({})", active_task_name),
                NetworkJobStatus::Completed => "Status: Completed successfully".to_string(),
                NetworkJobStatus::Failed(e) => format!("Status: Failed ({})", e),
            },
            None => "Status: Unknown".to_string(),
        }
    }

    fn is_failed(&self) -> bool {
        match &self.job {
            Some(j) => matches!(j.status, NetworkJobStatus::Failed(_)),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Main Downloads Dialog Component
// ---------------------------------------------------------------------------

pub struct DownloadDialog {
    visible: bool,
    job_rows: FactoryVecDeque<DownloadJobRow>,
    has_jobs: bool,
    details_dialog: Controller<DownloadJobDetailsDialog>,
    active_job_details_id: Option<String>,
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
    ShowDetails(String),
    RetryJob(String),
}

#[derive(Debug)]
pub enum DownloadDialogOutput {
    RemoveJob(String),
    ClearFinishedJobs,
    RetryJob(String),
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
            set_child = &adw::ToastOverlay {
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
                        },
                    },
                },
            }
        }
    }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let job_rows = FactoryVecDeque::builder()
            .launch(gtk::ListBox::new())
            .forward(sender.input_sender(), |output| match output {
                DownloadJobRowOutput::Remove(id) => DownloadDialogInput::RemoveRow(id),
                DownloadJobRowOutput::ShowDetails(id) => DownloadDialogInput::ShowDetails(id),
            });

        let details_dialog = DownloadJobDetailsDialog::builder()
            .launch(())
            .forward(sender.input_sender(), |output| match output {
                DownloadDetailsOutput::Retry(id) => DownloadDialogInput::RetryJob(id),
            });

        let model = DownloadDialog {
            visible: false,
            job_rows,
            has_jobs: false,
            details_dialog,
            active_job_details_id: None,
        };

        let jobs_list = model.job_rows.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DownloadDialogInput::Show => {
                self.visible = true;
                self.active_job_details_id = None;
                sender.input(DownloadDialogInput::Refresh);
            }
            DownloadDialogInput::Start => {
                self.visible = true;
                self.active_job_details_id = None;
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
                let mut updates = Vec::new();
                for job in &jobs {
                    let mut found = false;
                    for i in 0..guard.len() {
                        if let Some(row) = guard.get_mut(i) {
                            if row.job.id == job.id {
                                row.job = job.clone();
                                updates.push((i, job.clone()));
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        guard.push_back(job.clone());
                    }
                }

                drop(guard);

                for (i, job) in updates {
                    self.job_rows.send(i, DownloadJobRowInput::Update(job));
                }

                self.has_jobs = !self.job_rows.is_empty();

                // 3. Update details dialog if open
                if let Some(ref active_id) = self.active_job_details_id {
                    if let Some(job) = jobs.into_iter().find(|j| j.id == *active_id) {
                        self.details_dialog.emit(DownloadDetailsInput::Update(job));
                    }
                }
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
            DownloadDialogInput::ShowDetails(id) => {
                self.active_job_details_id = Some(id.clone());
                let jobs = crate::backend::download::manager::DOWNLOAD_QUEUE.get_jobs();
                if let Some(job) = jobs.into_iter().find(|j| j.id == id) {
                    self.details_dialog.emit(DownloadDetailsInput::Open(job));
                    self.details_dialog.widget().present(Some(root));
                }
            }
            DownloadDialogInput::RetryJob(id) => {
                sender.output(DownloadDialogOutput::RetryJob(id)).ok();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Status Bar Component
// ---------------------------------------------------------------------------

pub trait RedrawOnUpdate {
    fn trigger_redraw(&self, _val: f64);
}

impl RedrawOnUpdate for gtk::DrawingArea {
    fn trigger_redraw(&self, _val: f64) {
        self.queue_draw();
    }
}

pub struct DownloadStatusBar {
    pub state: DownloadState,
    pub visible: bool,
    pub progress_value: std::rc::Rc<std::cell::Cell<f64>>,
    pub is_downloading: std::rc::Rc<std::cell::Cell<bool>>,
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
        gtk::Button {
            set_css_classes: &["flat"],
            set_width_request: 34,
            set_height_request: 34,
            #[watch]
            set_tooltip_text: Some("Downloads"),
            connect_clicked[sender] => move |_| {
                sender.output(DownloadStatusBarOutput::Clicked).unwrap();
            },
            #[wrap(Some)]
            set_child = &gtk::Overlay {
                #[wrap(Some)]
                #[name = "progress_circle"]
                set_child = &gtk::DrawingArea {
                    set_css_classes: &["download-progress-circle"],
                    set_content_width: 16,
                    set_content_height: 16,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::Center,
                    #[watch]
                    trigger_redraw: model.get_progress(),
                },

                add_overlay = &gtk::Image {
                    #[watch]
                    set_icon_name: Some(if model.is_downloading() {
                        "arrow4-down-symbolic"
                    } else {
                        "arrow3-down-symbolic"
                    }),
                    #[watch]
                    set_pixel_size: if model.is_downloading() {
                        12
                    } else {
                        16
                    },
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Fill,
                }
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let progress_value = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let is_downloading = std::rc::Rc::new(std::cell::Cell::new(false));
        let model = DownloadStatusBar {
            state: DownloadState::Idle,
            visible: true,
            progress_value: progress_value.clone(),
            is_downloading: is_downloading.clone(),
        };

        let widgets = view_output!();

        let progress_clone = progress_value.clone();
        let downloading_clone = is_downloading.clone();
        widgets
            .progress_circle
            .set_draw_func(move |_area, cr, width, height| {
                if !downloading_clone.get() {
                    cr.set_operator(gtk::cairo::Operator::Clear);
                    let _ = cr.paint();
                    return;
                }
                cr.set_operator(gtk::cairo::Operator::Over);
                let progress = progress_clone.get();
                let cx = width as f64 / 2.0;
                let cy = height as f64 / 2.0;
                let radius = (width.min(height) as f64 / 2.0) - 1.2; // leave padding

                if radius > 0.0 {
                    // Draw background track (light gray with transparency)
                    cr.set_source_rgba(0.7, 0.7, 0.7, 0.35);
                    cr.set_line_width(2.2);
                    cr.arc(cx, cy, radius, 0.0, 2.0 * std::f64::consts::PI);
                    let _ = cr.stroke();

                    if progress > 0.0 {
                        // Query dynamic theme accent color using non-deprecated Widget::color()
                        let accent_color = _area.color();

                        cr.set_source_rgba(
                            accent_color.red() as f64,
                            accent_color.green() as f64,
                            accent_color.blue() as f64,
                            accent_color.alpha() as f64,
                        );
                        cr.set_line_width(2.2);
                        let start_angle = -std::f64::consts::FRAC_PI_2;
                        let end_angle = start_angle + progress * 2.0 * std::f64::consts::PI;
                        cr.arc(cx, cy, radius, start_angle, end_angle);
                        let _ = cr.stroke();
                    }
                }
            });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            DownloadStatusBarInput::Update(state, _visible) => {
                self.state = state;
                self.visible = true;
                self.progress_value.set(self.get_progress());
                self.is_downloading.set(self.is_downloading());
            }
            DownloadStatusBarInput::Dismiss => {
                self.state = DownloadState::Idle;
                self.visible = true;
                self.progress_value.set(0.0);
                self.is_downloading.set(false);
            }
        }
    }
}

impl DownloadStatusBar {
    fn is_downloading(&self) -> bool {
        matches!(
            self.state,
            DownloadState::Starting | DownloadState::Downloading { .. }
        )
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
