pub mod tabs;
pub mod helpers;

pub use tabs::console::{ConsoleInput, ConsoleOutput, InstanceConsole};
pub use tabs::editor::{EditorTabInput, EditorTabOutput, InstanceEditorTab};
pub use tabs::settings::{InstanceSettingsTab, SettingsTabInput, SettingsTabOutput};
pub use tabs::summary::{InstanceSummary, SummaryInput, SummaryOutput};
