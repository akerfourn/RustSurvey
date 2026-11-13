use crate::{ByteRange, MAX_OBJECT_BYTES, ObjectKey, ObjectMetadata, ObjectRead, StoreError};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

const METADATA_LIMIT: u64 = 128;
const FORMAT: &str = "rustsurvey-object-v1";

pub(super) fn check_directory(path: &Path) -> Result<(), StoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => StoreError::NotFound,
        _ => StoreError::Unavailable,
    })?;
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        return Ok(());
    }
    Err(StoreError::Unavailable)
}

pub(super) fn prepare_directory(path: &Path) -> Result<PathBuf, StoreError> {
    if path.as_os_str().is_empty() {
        return Err(StoreError::Unavailable);
    }
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(|_| StoreError::Unavailable)?;
    check_directory(path)?;
    fs::canonicalize(path).map_err(|_| StoreError::Unavailable)
}

pub(super) fn require_absent(path: &Path) -> Result<(), StoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(StoreError::Conflict),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StoreError::Unavailable),
    }
}

fn regular_file(path: &Path) -> Result<fs::Metadata, StoreError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| StoreError::Unavailable)?;
    if metadata.is_file() && !metadata.file_type().is_symlink() {
        return Ok(metadata);
    }
    Err(StoreError::Unavailable)
}

pub(super) fn encode_metadata(metadata: &ObjectMetadata) -> Vec<u8> {
    format!("{FORMAT}\n{}\n{}\n", metadata.size, metadata.sha256).into_bytes()
}

pub(super) fn metadata(root: &Path, key: &ObjectKey) -> Result<ObjectMetadata, StoreError> {
    let directory = root.join(key.as_str());
    check_directory(&directory)?;
    let path = directory.join("metadata");
    if regular_file(&path)?.len() > METADATA_LIMIT {
        return Err(StoreError::Unavailable);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| StoreError::Unavailable)?
        .take(METADATA_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Unavailable)?;
    if bytes.len() as u64 > METADATA_LIMIT {
        return Err(StoreError::Unavailable);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| StoreError::Unavailable)?;
    let mut fields = text.split('\n');
    if fields.next() != Some(FORMAT) {
        return Err(StoreError::Unavailable);
    }
    let size = fields
        .next()
        .and_then(|size| size.parse::<u64>().ok())
        .filter(|size| *size <= MAX_OBJECT_BYTES as u64)
        .ok_or(StoreError::Unavailable)?;
    let sha256 = fields.next().ok_or(StoreError::Unavailable)?;
    ObjectKey::parse(sha256).map_err(|_| StoreError::Unavailable)?;
    if fields.next() != Some("") || fields.next().is_some() {
        return Err(StoreError::Unavailable);
    }
    if regular_file(&directory.join("content"))?.len() != size {
        return Err(StoreError::Unavailable);
    }
    Ok(ObjectMetadata {
        size,
        sha256: sha256.to_owned(),
    })
}

pub(super) fn read(
    root: &Path,
    key: &ObjectKey,
    range: Option<ByteRange>,
) -> Result<ObjectRead, StoreError> {
    let metadata = metadata(root, key)?;
    let range = ByteRange::resolve(range, metadata.size)?;
    let mut bytes = Vec::with_capacity(metadata.size as usize);
    File::open(root.join(key.as_str()).join("content"))
        .map_err(|_| StoreError::Unavailable)?
        .take(MAX_OBJECT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError::Unavailable)?;
    if ObjectMetadata::for_bytes(&bytes).map_err(|_| StoreError::Unavailable)? != metadata {
        return Err(StoreError::Unavailable);
    }
    bytes.drain(..range.start as usize);
    bytes.truncate((range.end - range.start) as usize);
    Ok(ObjectRead {
        metadata,
        range,
        bytes,
    })
}

pub(super) fn delete(root: &Path, key: &ObjectKey) -> Result<(), StoreError> {
    match metadata(root, key) {
        Err(StoreError::NotFound) => return Ok(()),
        Err(error) => return Err(error),
        Ok(_) => {}
    }
    let directory = root.join(key.as_str());
    for entry in fs::read_dir(&directory).map_err(|_| StoreError::Unavailable)? {
        let name = entry.map_err(|_| StoreError::Unavailable)?.file_name();
        if name != "content" && name != "metadata" {
            return Err(StoreError::Unavailable);
        }
    }
    match fs::remove_dir_all(directory) {
        Ok(()) => sync_directory(root),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(StoreError::Unavailable),
    }
}

pub(super) fn sync_directory(path: &Path) -> Result<(), StoreError> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| StoreError::Unavailable)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}
