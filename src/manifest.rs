use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapAssetKind {
    #[default]
    Map,
    World,
}

impl MapAssetKind {
    pub fn from_path(path: &str) -> Self {
        match std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("world") => Self::World,
            _ => Self::Map,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Map => "Map",
            Self::World => "World",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapBadge {
    pub label: String,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapDetail {
    pub label: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapManifestEntry {
    pub path: String,
    pub title: String,
    #[serde(default)]
    pub kind: Option<MapAssetKind>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub badges: Vec<MapBadge>,
    #[serde(default)]
    pub details: Vec<MapDetail>,
}

impl MapManifestEntry {
    pub fn display_title(&self) -> &str {
        if self.title.is_empty() {
            &self.path
        } else {
            &self.title
        }
    }

    pub fn asset_kind(&self) -> MapAssetKind {
        self.kind
            .unwrap_or_else(|| MapAssetKind::from_path(&self.path))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapManifest {
    #[serde(default)]
    pub maps: Vec<MapManifestEntry>,
}

pub(crate) fn manifest_entry_from_path(path: &str) -> MapManifestEntry {
    let normalized = path.replace('\\', "/");
    let title = std::path::Path::new(&normalized)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&normalized)
        .to_string();

    MapManifestEntry {
        path: normalized.clone(),
        title,
        kind: Some(MapAssetKind::from_path(&normalized)),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{MapAssetKind, MapManifestEntry, manifest_entry_from_path};

    #[test]
    fn infer_kind_from_path_extension() {
        assert_eq!(MapAssetKind::from_path("maps/demo.tmx"), MapAssetKind::Map);
        assert_eq!(
            MapAssetKind::from_path("maps/worlds/ruins.world"),
            MapAssetKind::World
        );
    }

    #[test]
    fn default_entry_sets_title_and_kind() {
        let entry = manifest_entry_from_path("maps/ruins.world");
        assert_eq!(entry.title, "ruins");
        assert_eq!(entry.asset_kind(), MapAssetKind::World);
    }

    #[test]
    fn missing_kind_falls_back_to_path_inference() {
        let entry = MapManifestEntry {
            path: "maps/sample.world".into(),
            title: "Sample".into(),
            kind: None,
            ..Default::default()
        };
        assert_eq!(entry.asset_kind(), MapAssetKind::World);
    }
}
