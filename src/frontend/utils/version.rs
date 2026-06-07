use crate::backend::runtime::versions::{MinecraftVersion, VersionType};

#[derive(Debug, Clone, Copy)]
pub struct VersionFilters {
    pub show_releases: bool,
    pub show_snapshots: bool,
    pub show_betas: bool,
    pub show_alphas: bool,
    pub show_experiments: bool,
}

impl VersionFilters {
    pub fn new() -> Self {
        Self {
            show_releases: true,
            show_snapshots: false,
            show_betas: false,
            show_alphas: false,
            show_experiments: false,
        }
    }

    pub fn active_filters(&self) -> Vec<VersionType> {
        let mut types = Vec::new();
        if self.show_releases {
            types.push(VersionType::Release);
        }
        if self.show_snapshots {
            types.push(VersionType::Snapshot);
        }
        if self.show_betas {
            types.push(VersionType::OldBeta);
        }
        if self.show_alphas {
            types.push(VersionType::OldAlpha);
        }
        if self.show_experiments {
            types.push(VersionType::Experiment);
        }
        types
    }

    pub fn filter_and_limit(
        &self,
        all_versions: &[MinecraftVersion],
        search_text: &str,
        limit: usize,
    ) -> Vec<MinecraftVersion> {
        use crate::backend::runtime::versions::filter_versions;
        let types = self.active_filters();
        let mut filtered = filter_versions(all_versions, &types);
        if !search_text.is_empty() {
            let query = search_text.to_lowercase();
            filtered.retain(|v| v.id.to_lowercase().contains(&query));
        }
        filtered.truncate(limit);
        filtered
    }
}
