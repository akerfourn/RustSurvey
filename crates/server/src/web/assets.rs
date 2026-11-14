use super::{Asset, WebAssets};
use axum::body::Bytes;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{self, Read},
    path::Path,
    sync::Arc,
};

const MAX_ASSETS: usize = 128;
const MAX_FILE_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 512 * 1024;

pub(super) fn invalid_build() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "STATIC_ASSETS_DIRECTORY must contain a complete bounded web build with regular files",
    )
}

pub(super) fn load(directory: &Path) -> io::Result<WebAssets> {
    require_directory(directory)?;
    let index = read_bounded(&directory.join("index.html"), MAX_INDEX_BYTES)?;
    let html = std::str::from_utf8(&index).map_err(|_| invalid_build())?;
    if !html.contains("/assets/") || html.contains("/src/") {
        return Err(invalid_build());
    }
    let assets = directory.join("assets");
    require_directory(&assets)?;
    let mut files = BTreeMap::new();
    let mut total = index.len();
    for (count, entry) in fs::read_dir(assets)
        .map_err(|_| invalid_build())?
        .enumerate()
    {
        if count >= MAX_ASSETS {
            return Err(invalid_build());
        }
        let entry = entry.map_err(|_| invalid_build())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| invalid_build())?;
        let Some(content_type) = content_type(&name) else {
            continue;
        };
        let bytes = read_bounded(&entry.path(), MAX_FILE_BYTES)?;
        total += bytes.len();
        if total > MAX_TOTAL_BYTES {
            return Err(invalid_build());
        }
        let cache_control = if fingerprinted(&name) {
            "public, max-age=31536000, immutable"
        } else {
            "no-cache"
        };
        files.insert(
            format!("/assets/{name}"),
            asset(bytes, content_type, cache_control),
        );
    }
    validate_references(html, &files)?;
    files.insert(
        "/index.html".into(),
        asset(index, "text/html; charset=utf-8", "no-cache"),
    );
    Ok(WebAssets {
        files: Arc::new(files),
    })
}

fn validate_references(html: &str, files: &BTreeMap<String, Asset>) -> io::Result<()> {
    let mut has_script = false;
    for prefix in ["src=\"/assets/", "href=\"/assets/"] {
        for reference in html.split(prefix).skip(1) {
            let Some((name, _)) = reference.split_once('"') else {
                return Err(invalid_build());
            };
            if !files.contains_key(&format!("/assets/{name}")) {
                return Err(invalid_build());
            }
            if prefix.starts_with("src=") && name.ends_with(".js") {
                has_script = true;
            }
        }
    }
    if !has_script {
        return Err(invalid_build());
    }
    Ok(())
}

fn require_directory(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| invalid_build())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(invalid_build());
    }
    Ok(())
}

fn read_bounded(path: &Path, limit: usize) -> io::Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).map_err(|_| invalid_build())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > limit as u64 {
        return Err(invalid_build());
    }
    let file = File::open(path).map_err(|_| invalid_build())?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid_build())?;
    if bytes.len() > limit {
        return Err(invalid_build());
    }
    Ok(bytes)
}

fn asset(bytes: Vec<u8>, content_type: &'static str, cache_control: &'static str) -> Asset {
    use std::fmt::Write;
    let mut etag = String::with_capacity(66);
    etag.push('"');
    for byte in Sha256::digest(&bytes) {
        write!(etag, "{byte:02x}").expect("writing to a string");
    }
    etag.push('"');
    Asset {
        body: Bytes::from(bytes),
        content_type,
        cache_control,
        etag,
    }
}

fn content_type(name: &str) -> Option<&'static str> {
    if name.len() > 160
        || name.starts_with('.')
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return None;
    }
    match name.rsplit_once('.')?.1 {
        "js" => Some("text/javascript; charset=utf-8"),
        "css" => Some("text/css; charset=utf-8"),
        "woff2" => Some("font/woff2"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "ico" => Some("image/x-icon"),
        _ => None,
    }
}

fn fingerprinted(name: &str) -> bool {
    let Some((stem, _)) = name.rsplit_once('.') else {
        return false;
    };
    let Some((_, hash)) = stem.rsplit_once('-') else {
        return false;
    };
    (8..=32).contains(&hash.len())
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
