use super::filesystem;
use crate::{ObjectKey, StoreError};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

pub(super) struct PendingDirectory {
    path: PathBuf,
    published: bool,
}
impl PendingDirectory {
    pub(super) fn create(
        root: &Path,
        key: &ObjectKey,
        serial: &AtomicU64,
    ) -> Result<Self, StoreError> {
        for _ in 0..256 {
            let next = serial.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                ".pending-{}-{next}-{}",
                std::process::id(),
                key.as_str()
            ));
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&path) {
                Ok(()) => {
                    return Ok(Self {
                        path,
                        published: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(StoreError::Unavailable),
            }
        }
        Err(StoreError::Unavailable)
    }
    pub(super) fn write(&self, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(self.path.join(name))
            .map_err(|_| StoreError::Unavailable)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| StoreError::Unavailable)
    }
    pub(super) fn publish(mut self, destination: &Path) -> Result<(), StoreError> {
        filesystem::sync_directory(&self.path)?;
        filesystem::require_absent(destination)?;
        if fs::rename(&self.path, destination).is_err() {
            return match filesystem::require_absent(destination) {
                Err(StoreError::Conflict) => Err(StoreError::Conflict),
                _ => Err(StoreError::Unavailable),
            };
        }
        self.published = true;
        filesystem::sync_directory(destination.parent().ok_or(StoreError::Unavailable)?)
    }
}
impl Drop for PendingDirectory {
    fn drop(&mut self) {
        if self.published {
            return;
        }
        if let Err(error) = fs::remove_dir_all(&self.path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(kind = ?error.kind(), "Object staging cleanup failed; remove abandoned pending directories while the store is stopped");
        }
    }
}

#[cfg(test)]
#[path = "../../tests/common/directory.rs"]
mod test_directory;

#[cfg(test)]
mod tests {
    use super::test_directory::Directory;
    use super::*;
    #[test]
    fn test_failed_write_should_leave_no_visible_object_and_remove_its_staging_directory() {
        let directory = Directory::create();
        let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
        {
            let pending =
                PendingDirectory::create(directory.path(), &key, &AtomicU64::new(0)).unwrap();
            pending.write("content", b"partial upload").unwrap();
            assert_eq!(
                pending.write("content", b"must not replace").unwrap_err(),
                StoreError::Unavailable
            );
            assert!(!directory.path().join(key.as_str()).exists());
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
    #[test]
    fn test_failed_publication_should_preserve_the_destination_and_remove_only_its_own_staging_directory()
     {
        let directory = Directory::create();
        let key = ObjectKey::parse(&"a".repeat(64)).unwrap();
        let pending = PendingDirectory::create(directory.path(), &key, &AtomicU64::new(0)).unwrap();
        pending.write("content", b"new").unwrap();
        let destination = directory.path().join(key.as_str());
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("foreign"), b"kept").unwrap();
        assert_eq!(
            pending.publish(&destination).unwrap_err(),
            StoreError::Conflict
        );
        assert_eq!(fs::read(destination.join("foreign")).unwrap(), b"kept");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
