pub mod helpers;
pub mod tabs;

pub use tabs::console::{ConsoleInput, ConsoleOutput, InstanceConsole};
pub use tabs::editor::{EditorTabInput, EditorTabOutput, InstanceEditorTab};
pub use tabs::settings::{InstanceSettingsTab, SettingsTabInput, SettingsTabOutput};
pub use tabs::summary::{InstanceSummary, SummaryInput, SummaryOutput};
