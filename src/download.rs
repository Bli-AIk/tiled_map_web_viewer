use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Cursor, Write};

use bevy::prelude::Resource;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::Deserialize;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::MapManifestEntry;

#[derive(Resource, Clone)]
pub(crate) struct AssetRootPath(pub(crate) String);

#[derive(Clone, Debug, Default)]
pub(crate) struct DownloadStatus {
    pub(crate) message: String,
    pub(crate) is_error: bool,
}

#[derive(Resource, Clone, Debug, Default)]
pub(crate) struct DownloadUiState {
    pub(crate) last_status: Option<DownloadStatus>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DownloadBundle {
    pub(crate) archive_name: String,
    pub(crate) files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadError(String);

impl DownloadError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for DownloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DownloadError {}

trait BundleSource {
    fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, DownloadError>;
}

fn build_download_bundle(
    source: &impl BundleSource,
    entry: &MapManifestEntry,
) -> Result<DownloadBundle, DownloadError> {
    let root_path = normalize_relative_path(&entry.path)?;
    let mut files = BTreeMap::new();
    let mut visited = BTreeSet::new();
    collect_bundle_file(source, &root_path, &mut visited, &mut files)?;
    Ok(DownloadBundle {
        archive_name: bundle_archive_name(entry),
        files,
    })
}

fn bundle_to_zip_bytes(bundle: &DownloadBundle) -> Result<Vec<u8>, DownloadError> {
    let cursor = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(cursor);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    for (path, bytes) in &bundle.files {
        zip.start_file(path, options).map_err(|err| {
            DownloadError::new(format!("failed to start zip entry '{path}': {err}"))
        })?;
        zip.write_all(bytes).map_err(|err| {
            DownloadError::new(format!("failed to write zip entry '{path}': {err}"))
        })?;
    }

    zip.finish()
        .map(|cursor| cursor.into_inner())
        .map_err(|err| DownloadError::new(format!("failed to finalize zip archive: {err}")))
}

pub(crate) fn trigger_download(
    asset_root: &str,
    entry: &MapManifestEntry,
) -> Result<String, DownloadError> {
    #[cfg(target_arch = "wasm32")]
    {
        let source = WebAssetSource::new(asset_root);
        let bundle = build_download_bundle(&source, entry)?;
        let zip_bytes = bundle_to_zip_bytes(&bundle)?;
        download_zip_bytes_web(&bundle.archive_name, &zip_bytes)?;
        return Ok(format!(
            "Downloaded '{}' ({} files)",
            bundle.archive_name,
            bundle.files.len()
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let source = NativeAssetSource::new(asset_root);
        let bundle = build_download_bundle(&source, entry)?;
        let zip_bytes = bundle_to_zip_bytes(&bundle)?;
        let output_dir = std::env::current_dir()
            .map_err(|err| {
                DownloadError::new(format!("failed to resolve current directory: {err}"))
            })?
            .join("downloads");
        std::fs::create_dir_all(&output_dir).map_err(|err| {
            DownloadError::new(format!(
                "failed to create '{}': {err}",
                output_dir.display()
            ))
        })?;
        let output_path = output_dir.join(&bundle.archive_name);
        std::fs::write(&output_path, zip_bytes).map_err(|err| {
            DownloadError::new(format!(
                "failed to write '{}': {err}",
                output_path.display()
            ))
        })?;
        Ok(format!(
            "Saved '{}' ({} files)",
            output_path.display(),
            bundle.files.len()
        ))
    }
}

fn bundle_archive_name(entry: &MapManifestEntry) -> String {
    let mut name = entry.display_title().trim().to_ascii_lowercase();
    if name.is_empty() {
        name = "map".into();
    }
    let sanitized: String = name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | '0'..='9' => ch,
            _ => '-',
        })
        .collect();
    let collapsed = sanitized
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let stem = if collapsed.is_empty() {
        "map"
    } else {
        &collapsed
    };
    format!("{stem}.zip")
}

fn collect_bundle_file(
    source: &impl BundleSource,
    relative_path: &str,
    visited: &mut BTreeSet<String>,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), DownloadError> {
    let relative_path = normalize_relative_path(relative_path)?;
    if !visited.insert(relative_path.clone()) {
        return Ok(());
    }

    let bytes = source.read_bytes(&relative_path)?;
    files.insert(relative_path.clone(), bytes.clone());

    match extension_of(&relative_path).as_deref() {
        Some("world") => {
            let text = decode_text_file(&relative_path, &bytes)?;
            for child in world_references(&text)? {
                let child_path = resolve_reference_path(&relative_path, &child)?;
                collect_bundle_file(source, &child_path, visited, files)?;
            }
        }
        Some("tmx") | Some("tsx") => {
            let text = decode_text_file(&relative_path, &bytes)?;
            for child in xml_references(&text)? {
                let child_path = resolve_reference_path(&relative_path, &child)?;
                collect_bundle_file(source, &child_path, visited, files)?;
            }
        }
        _ => {}
    }

    Ok(())
}

fn decode_text_file(path: &str, bytes: &[u8]) -> Result<String, DownloadError> {
    String::from_utf8(bytes.to_vec())
        .map_err(|err| DownloadError::new(format!("'{path}' is not valid UTF-8: {err}")))
}

fn extension_of(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

fn normalize_relative_path(path: &str) -> Result<String, DownloadError> {
    let mut parts = Vec::new();
    for part in path.replace('\\', "/").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop().ok_or_else(|| {
                    DownloadError::new(format!(
                        "reference escapes asset root: '{}'",
                        path.replace('\\', "/")
                    ))
                })?;
            }
            value => parts.push(value.to_string()),
        }
    }

    if parts.is_empty() {
        return Err(DownloadError::new(format!(
            "path resolves to asset root: '{}'",
            path.replace('\\', "/")
        )));
    }

    Ok(parts.join("/"))
}

fn resolve_reference_path(base_file: &str, reference: &str) -> Result<String, DownloadError> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err(DownloadError::new(format!(
            "empty reference found inside '{base_file}'"
        )));
    }
    if reference.contains("://") {
        return Err(DownloadError::new(format!(
            "external URL references are not supported: '{reference}'"
        )));
    }
    if reference.starts_with('/') {
        return Err(DownloadError::new(format!(
            "absolute references are not supported: '{reference}'"
        )));
    }

    let joined = match base_file.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => format!("{dir}/{reference}"),
        _ => reference.to_string(),
    };
    normalize_relative_path(&joined)
}

fn world_references(text: &str) -> Result<Vec<String>, DownloadError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct WorldMapRef {
        file_name: String,
    }

    #[derive(Deserialize)]
    struct WorldDocument {
        #[serde(default)]
        maps: Vec<WorldMapRef>,
    }

    let parsed: WorldDocument = serde_json::from_str(text)
        .map_err(|err| DownloadError::new(format!("failed to parse .world file: {err}")))?;
    Ok(parsed.maps.into_iter().map(|map| map.file_name).collect())
}

fn xml_references(text: &str) -> Result<Vec<String>, DownloadError> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut references = Vec::new();
    let mut buffer = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let tag = event.name();
                if tag.as_ref() == b"tileset" || tag.as_ref() == b"image" {
                    for attr in event.attributes().with_checks(false) {
                        let attr = attr.map_err(|err| {
                            DownloadError::new(format!("failed to parse XML attribute: {err}"))
                        })?;
                        if attr.key.as_ref() == b"source" {
                            let value = attr.decode_and_unescape_value(reader.decoder()).map_err(
                                |err| {
                                    DownloadError::new(format!(
                                        "failed to decode XML attribute value: {err}"
                                    ))
                                },
                            )?;
                            references.push(value.into_owned());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => {
                return Err(DownloadError::new(format!(
                    "failed to parse tiled XML file: {err}"
                )));
            }
        }

        buffer.clear();
    }

    Ok(references)
}

#[cfg(not(target_arch = "wasm32"))]
struct NativeAssetSource {
    asset_root: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeAssetSource {
    fn new(asset_root: &str) -> Self {
        Self {
            asset_root: std::path::PathBuf::from(asset_root),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl BundleSource for NativeAssetSource {
    fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, DownloadError> {
        let full_path = self.asset_root.join(relative_path);
        std::fs::read(&full_path).map_err(|err| {
            DownloadError::new(format!("failed to read '{}': {err}", full_path.display()))
        })
    }
}

#[cfg(target_arch = "wasm32")]
struct WebAssetSource {
    asset_root: String,
}

#[cfg(target_arch = "wasm32")]
impl WebAssetSource {
    fn new(asset_root: &str) -> Self {
        Self {
            asset_root: asset_root.trim_end_matches('/').to_string(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl BundleSource for WebAssetSource {
    fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, DownloadError> {
        let xhr = web_sys::XmlHttpRequest::new()
            .map_err(|err| DownloadError::new(format!("failed to create XHR: {err:?}")))?;
        let url = format!("{}/{}", self.asset_root, relative_path);
        xhr.open_with_async("GET", &url, false).map_err(|err| {
            DownloadError::new(format!("failed to open XHR for '{url}': {err:?}"))
        })?;
        xhr.set_response_type(web_sys::XmlHttpRequestResponseType::Arraybuffer);
        xhr.send()
            .map_err(|err| DownloadError::new(format!("failed to fetch '{url}': {err:?}")))?;

        let status = xhr.status().map_err(|err| {
            DownloadError::new(format!("failed to read HTTP status for '{url}': {err:?}"))
        })?;
        if !(200..300).contains(&status) {
            return Err(DownloadError::new(format!(
                "failed to fetch '{url}': HTTP {status}"
            )));
        }

        let response = xhr.response().map_err(|err| {
            DownloadError::new(format!("failed to read XHR response for '{url}': {err:?}"))
        })?;
        if response.is_null() || response.is_undefined() {
            return Err(DownloadError::new(format!(
                "empty XHR response for '{url}'"
            )));
        }
        Ok(js_sys::Uint8Array::new(&response).to_vec())
    }
}

#[cfg(target_arch = "wasm32")]
fn download_zip_bytes_web(file_name: &str, bytes: &[u8]) -> Result<(), DownloadError> {
    use wasm_bindgen::JsCast;

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)
        .map_err(|err| DownloadError::new(format!("failed to create Blob: {err:?}")))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|err| DownloadError::new(format!("failed to create object URL: {err:?}")))?;

    let window = web_sys::window().ok_or_else(|| DownloadError::new("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| DownloadError::new("document is unavailable"))?;
    let element = document
        .create_element("a")
        .map_err(|err| DownloadError::new(format!("failed to create anchor element: {err:?}")))?;
    let anchor = element
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| DownloadError::new("failed to cast anchor element"))?;
    anchor.set_href(&url);
    anchor.set_download(file_name);

    let body = document
        .body()
        .ok_or_else(|| DownloadError::new("document body is unavailable"))?;
    body.append_child(&anchor)
        .map_err(|err| DownloadError::new(format!("failed to append anchor: {err:?}")))?;
    anchor.click();
    let _ = body.remove_child(&anchor);
    web_sys::Url::revoke_object_url(&url)
        .map_err(|err| DownloadError::new(format!("failed to revoke object URL: {err:?}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        BundleSource, DownloadError, build_download_bundle, bundle_to_zip_bytes,
        normalize_relative_path, resolve_reference_path,
    };
    use crate::MapManifestEntry;
    use std::collections::HashMap;
    use std::io::Cursor;

    struct FakeSource {
        files: HashMap<String, Vec<u8>>,
    }

    impl FakeSource {
        fn new(files: impl IntoIterator<Item = (&'static str, &'static [u8])>) -> Self {
            Self {
                files: files
                    .into_iter()
                    .map(|(path, bytes)| (path.to_string(), bytes.to_vec()))
                    .collect(),
            }
        }
    }

    impl BundleSource for FakeSource {
        fn read_bytes(&self, relative_path: &str) -> Result<Vec<u8>, DownloadError> {
            self.files
                .get(relative_path)
                .cloned()
                .ok_or_else(|| DownloadError::new(format!("missing test asset '{relative_path}'")))
        }
    }

    fn entry(path: &str, title: &str) -> MapManifestEntry {
        MapManifestEntry {
            path: path.into(),
            title: title.into(),
            ..Default::default()
        }
    }

    #[test]
    fn normalize_and_resolve_references() {
        assert_eq!(
            normalize_relative_path("curated/worlds/undertale/../undertale/room.tmx").unwrap(),
            "curated/worlds/undertale/room.tmx"
        );
        assert_eq!(
            resolve_reference_path(
                "curated/worlds/undertale/ruins.world",
                "../../undertale/room_ruins1.tmx"
            )
            .unwrap(),
            "curated/undertale/room_ruins1.tmx"
        );
    }

    #[test]
    fn reject_paths_that_escape_asset_root() {
        let err = resolve_reference_path("maps/room.tmx", "../../oops.tsx").unwrap_err();
        assert!(err.to_string().contains("escapes asset root"));
    }

    #[test]
    fn bundle_tmx_collects_external_tileset_and_image() {
        let source = FakeSource::new([
            (
                "maps/room.tmx",
                br#"<map><tileset source="../tilesets/test.tsx"/></map>"#.as_ref(),
            ),
            (
                "tilesets/test.tsx",
                br#"<tileset><image source="../images/test.png"/></tileset>"#.as_ref(),
            ),
            ("images/test.png", b"png-data".as_ref()),
        ]);

        let bundle = build_download_bundle(&source, &entry("maps/room.tmx", "Room")).unwrap();
        let paths: Vec<_> = bundle.files.keys().cloned().collect();
        assert_eq!(
            paths,
            vec![
                "images/test.png".to_string(),
                "maps/room.tmx".to_string(),
                "tilesets/test.tsx".to_string(),
            ]
        );
    }

    #[test]
    fn bundle_tmx_collects_inline_tileset_images() {
        let source = FakeSource::new([
            (
                "maps/room.tmx",
                br#"<map><tileset firstgid="1"><image source="../images/inline.png"/></tileset></map>"#
                    .as_ref(),
            ),
            ("images/inline.png", b"png-data".as_ref()),
        ]);

        let bundle = build_download_bundle(&source, &entry("maps/room.tmx", "Room")).unwrap();
        let paths: Vec<_> = bundle.files.keys().cloned().collect();
        assert_eq!(
            paths,
            vec!["images/inline.png".to_string(), "maps/room.tmx".to_string()]
        );
    }

    #[test]
    fn bundle_world_collects_maps_and_shared_dependencies_once() {
        let source = FakeSource::new([
            (
                "worlds/zone.world",
                br#"{"maps":[{"fileName":"../maps/a.tmx"},{"fileName":"../maps/b.tmx"}]}"#.as_ref(),
            ),
            (
                "maps/a.tmx",
                br#"<map><tileset source="../tilesets/shared.tsx"/></map>"#.as_ref(),
            ),
            (
                "maps/b.tmx",
                br#"<map><tileset source="../tilesets/shared.tsx"/></map>"#.as_ref(),
            ),
            (
                "tilesets/shared.tsx",
                br#"<tileset><image source="../images/shared.png"/></tileset>"#.as_ref(),
            ),
            ("images/shared.png", b"png-data".as_ref()),
        ]);

        let bundle =
            build_download_bundle(&source, &entry("worlds/zone.world", "Zone World")).unwrap();
        let paths: Vec<_> = bundle.files.keys().cloned().collect();
        assert_eq!(
            paths,
            vec![
                "images/shared.png".to_string(),
                "maps/a.tmx".to_string(),
                "maps/b.tmx".to_string(),
                "tilesets/shared.tsx".to_string(),
                "worlds/zone.world".to_string(),
            ]
        );
    }

    #[test]
    fn bundle_world_preserves_real_relative_paths() {
        let source = FakeSource::new([
            (
                "curated/worlds/undertale/waterfall.world",
                br#"{"maps":[{"fileName":"../../undertale/room_tundra8.tmx"}]}"#.as_ref(),
            ),
            (
                "curated/undertale/room_tundra8.tmx",
                br#"<map><tileset source="tilesets/bg_tundra.tsx"/></map>"#.as_ref(),
            ),
            (
                "curated/undertale/tilesets/bg_tundra.tsx",
                br#"<tileset><image source="../textures/bg_tundra.png"/></tileset>"#.as_ref(),
            ),
            (
                "curated/undertale/textures/bg_tundra.png",
                b"png-data".as_ref(),
            ),
        ]);

        let bundle = build_download_bundle(
            &source,
            &entry("curated/worlds/undertale/waterfall.world", "Waterfall"),
        )
        .unwrap();
        let paths: Vec<_> = bundle.files.keys().cloned().collect();
        assert_eq!(
            paths,
            vec![
                "curated/undertale/room_tundra8.tmx".to_string(),
                "curated/undertale/textures/bg_tundra.png".to_string(),
                "curated/undertale/tilesets/bg_tundra.tsx".to_string(),
                "curated/worlds/undertale/waterfall.world".to_string(),
            ]
        );
    }

    #[test]
    fn zip_archive_keeps_expected_paths() {
        let source = FakeSource::new([
            (
                "maps/room.tmx",
                br#"<map><tileset source="../tilesets/test.tsx"/></map>"#.as_ref(),
            ),
            (
                "tilesets/test.tsx",
                br#"<tileset><image source="../images/test.png"/></tileset>"#.as_ref(),
            ),
            ("images/test.png", b"png-data".as_ref()),
        ]);

        let bundle = build_download_bundle(&source, &entry("maps/room.tmx", "Room")).unwrap();
        let zip_bytes = bundle_to_zip_bytes(&bundle).unwrap();
        let reader = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(reader).unwrap();
        let mut names = Vec::new();
        for index in 0..archive.len() {
            let file = archive.by_index(index).unwrap();
            names.push(file.name().to_string());
        }
        names.sort();
        assert_eq!(
            names,
            vec![
                "images/test.png".to_string(),
                "maps/room.tmx".to_string(),
                "tilesets/test.tsx".to_string(),
            ]
        );
    }
}
