use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MavenCoordinate {
    pub group: String,
    pub artifact: String,
    pub version: String,
    pub classifier: Option<String>,
    pub extension: String,
}

impl MavenCoordinate {
    /// Parses a Maven coordinate string: `group:artifact:version[:classifier][@extension]`
    pub fn parse(coordinate: &str) -> Option<Self> {
        let parts: Vec<&str> = coordinate.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        let group = parts[0].to_string();
        let artifact = parts[1].to_string();
        let mut version_str = parts[2];
        let mut extension = "jar".to_string();

        if let Some(pos) = version_str.find('@') {
            extension = version_str[pos + 1..].to_string();
            version_str = &version_str[..pos];
        }

        let classifier = if parts.len() > 3 {
            let mut cls = parts[3];
            if let Some(pos) = cls.find('@') {
                extension = cls[pos + 1..].to_string();
                cls = &cls[..pos];
            }
            Some(cls.to_string())
        } else {
            None
        };

        Some(Self {
            group,
            artifact,
            version: version_str.to_string(),
            classifier,
            extension,
        })
    }

    /// Returns the standard filename for this Maven artifact, e.g. `lwjgl-3.3.3-natives-linux.jar`
    pub fn file_name(&self) -> String {
        if let Some(ref cls) = self.classifier {
            format!(
                "{}-{}-{}.{}",
                self.artifact, self.version, cls, self.extension
            )
        } else {
            format!("{}-{}.{}", self.artifact, self.version, self.extension)
        }
    }

    /// Returns the relative path within a Maven repository / libraries directory.
    pub fn to_relative_path(&self) -> PathBuf {
        let group_path = self.group.replace('.', "/");
        Path::new(&group_path)
            .join(&self.artifact)
            .join(&self.version)
            .join(self.file_name())
    }

    /// Resolves full path against a base libraries path.
    pub fn to_full_path(&self, libraries_root: &Path) -> PathBuf {
        libraries_root.join(self.to_relative_path())
    }

    /// Returns the download URL given a base Maven repository URL.
    pub fn to_url(&self, repo_url: &str) -> String {
        let base = repo_url.trim_end_matches('/');
        let rel = self.to_relative_path();
        format!("{}/{}", base, rel.to_string_lossy())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_standard_coordinate() {
        let coord = MavenCoordinate::parse("org.lwjgl:lwjgl:3.3.3").unwrap();
        assert_eq!(coord.group, "org.lwjgl");
        assert_eq!(coord.artifact, "lwjgl");
        assert_eq!(coord.version, "3.3.3");
        assert_eq!(coord.classifier, None);
        assert_eq!(coord.extension, "jar");
        assert_eq!(coord.file_name(), "lwjgl-3.3.3.jar");
        assert_eq!(
            coord.to_relative_path(),
            PathBuf::from("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3.jar")
        );
    }

    #[test]
    fn test_parse_classifier_coordinate() {
        let coord = MavenCoordinate::parse("org.lwjgl:lwjgl:3.3.3:natives-linux").unwrap();
        assert_eq!(coord.classifier, Some("natives-linux".to_string()));
        assert_eq!(coord.file_name(), "lwjgl-3.3.3-natives-linux.jar");
        assert_eq!(
            coord.to_relative_path(),
            PathBuf::from("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
        );
    }

    #[test]
    fn test_parse_extension_coordinate() {
        let coord = MavenCoordinate::parse("com.example:test:1.0.0@zip").unwrap();
        assert_eq!(coord.extension, "zip");
        assert_eq!(coord.file_name(), "test-1.0.0.zip");
        assert_eq!(
            coord.to_relative_path(),
            PathBuf::from("com/example/test/1.0.0/test-1.0.0.zip")
        );
    }
}
