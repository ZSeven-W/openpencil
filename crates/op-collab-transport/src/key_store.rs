use std::fmt;
#[cfg(unix)]
use std::fs::{File, OpenOptions};
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use x25519_dalek::{PublicKey, StaticSecret};
#[cfg(unix)]
use zeroize::Zeroizing;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::KeyStoreError;

const DEVICE_KEY_FILE: &str = "device-x25519.key";
const PRIVATE_KEY_BYTES: usize = 32;

pub trait StaticKeyStore: Send + Sync {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError>;
}

/// X25519 device static. Debug output and drop never expose or retain private bytes.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DeviceStaticKey {
    private: [u8; PRIVATE_KEY_BYTES],
    #[zeroize(skip)]
    public: [u8; PRIVATE_KEY_BYTES],
}

impl DeviceStaticKey {
    pub fn from_private(private: [u8; PRIVATE_KEY_BYTES]) -> Result<Self, KeyStoreError> {
        if private.iter().all(|byte| *byte == 0) {
            let mut private = private;
            private.zeroize();
            return Err(KeyStoreError::AllZero);
        }
        let secret = StaticSecret::from(private);
        let public = PublicKey::from(&secret).to_bytes();
        Ok(Self { private, public })
    }

    pub fn generate() -> Result<Self, KeyStoreError> {
        let mut private = [0_u8; PRIVATE_KEY_BYTES];
        getrandom::fill(&mut private).map_err(|_| KeyStoreError::Random)?;
        Self::from_private(private)
    }

    pub fn public_key(&self) -> &[u8; PRIVATE_KEY_BYTES] {
        &self.public
    }

    pub(crate) fn private_key(&self) -> &[u8; PRIVATE_KEY_BYTES] {
        &self.private
    }
}

impl fmt::Debug for DeviceStaticKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DeviceStaticKey")
            .field("private", &"[REDACTED]")
            .field("public", &self.public)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct FileKeyStore {
    directory: PathBuf,
}

impl FileKeyStore {
    /// Creates a store rooted at a dedicated key directory.
    ///
    /// The parent directory must already exist. The dedicated directory is
    /// atomically created with owner-only permissions when absent.
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn key_path(&self) -> PathBuf {
        self.directory.join(DEVICE_KEY_FILE)
    }

    #[cfg(unix)]
    fn ensure_directory(&self) -> Result<(), KeyStoreError> {
        match std::fs::symlink_metadata(&self.directory) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(KeyStoreError::UnsafeDirectory);
                }
                restrict_directory_permissions(&self.directory)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                create_private_directory(&self.directory)?;
                let metadata = std::fs::symlink_metadata(&self.directory)?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(KeyStoreError::UnsafeDirectory);
                }
            }
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    #[cfg(unix)]
    fn load_existing(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        let path = self.key_path();
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(KeyStoreError::UnsafeFile);
        }
        let mut options = OpenOptions::new();
        options.read(true);
        add_no_follow(&mut options);
        let file = options.open(path)?;
        if !file.metadata()?.is_file() {
            return Err(KeyStoreError::UnsafeFile);
        }
        restrict_open_file_permissions(&file)?;
        let mut bytes = Zeroizing::new(Vec::with_capacity(PRIVATE_KEY_BYTES + 1));
        file.take((PRIVATE_KEY_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() != PRIVATE_KEY_BYTES {
            return Err(KeyStoreError::InvalidLength(bytes.len()));
        }
        let mut private = [0_u8; PRIVATE_KEY_BYTES];
        private.copy_from_slice(&bytes);
        DeviceStaticKey::from_private(private)
    }

    #[cfg(unix)]
    fn install_generated(&self, key: &DeviceStaticKey) -> Result<(), KeyStoreError> {
        let mut suffix = [0_u8; 8];
        getrandom::fill(&mut suffix).map_err(|_| KeyStoreError::Random)?;
        let suffix = u64::from_be_bytes(suffix);
        let temporary = self
            .directory
            .join(format!(".device-x25519.{suffix:016x}.tmp"));
        let cleanup = TemporaryKeyFile(temporary.clone());

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        add_private_create_mode(&mut options);
        add_no_follow(&mut options);
        let mut file = options.open(&temporary)?;
        file.write_all(key.private_key())?;
        file.sync_all()?;
        drop(file);

        let final_path = self.key_path();
        match std::fs::hard_link(&temporary, &final_path) {
            Ok(()) => {
                sync_directory(&self.directory)?;
                drop(cleanup);
                let _ = std::fs::remove_file(&temporary);
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                drop(cleanup);
                let _ = std::fs::remove_file(&temporary);
                // A concurrent process won. It must still pass no-follow,
                // size, permission, and all-zero validation.
                self.load_existing().map(|_| ())
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl StaticKeyStore for FileKeyStore {
    fn load_or_generate(&self) -> Result<DeviceStaticKey, KeyStoreError> {
        #[cfg(not(unix))]
        {
            Err(KeyStoreError::UnsupportedPlatform)
        }
        #[cfg(unix)]
        {
            self.ensure_directory()?;
            match self.load_existing() {
                Ok(key) => Ok(key),
                Err(KeyStoreError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                    let key = DeviceStaticKey::generate()?;
                    self.install_generated(&key)?;
                    // Re-open through the hardened read path rather than trusting
                    // that the final path still names the temporary inode.
                    self.load_existing()
                }
                Err(error) => Err(error),
            }
        }
    }
}

#[cfg(unix)]
struct TemporaryKeyFile(PathBuf);

#[cfg(unix)]
impl Drop for TemporaryKeyFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> std::io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = std::fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(unix)]
fn restrict_directory_permissions(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

#[cfg(unix)]
fn restrict_open_file_permissions(file: &File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(unix)]
fn add_private_create_mode(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.mode(0o600);
}

#[cfg(unix)]
fn add_no_follow(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_parent(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let parent = std::env::temp_dir().join(format!(
            "op-collab-key-store-{name}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&parent).unwrap();
        parent
    }

    #[cfg(unix)]
    #[test]
    fn generated_key_is_stable_and_file_is_exactly_32_bytes() {
        let parent = temp_parent("roundtrip");
        let store = FileKeyStore::new(parent.join("keys"));
        let first = store.load_or_generate().unwrap();
        let second = store.load_or_generate().unwrap();
        assert_eq!(first.public_key(), second.public_key());
        assert_eq!(std::fs::read(store.key_path()).unwrap().len(), 32);
    }

    #[cfg(unix)]
    #[test]
    fn directory_and_key_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let parent = temp_parent("mode");
        let store = FileKeyStore::new(parent.join("keys"));
        store.load_or_generate().unwrap();
        assert_eq!(
            std::fs::metadata(store.directory())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(store.key_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_and_key_symlinks_are_rejected() {
        use std::os::unix::fs::symlink;

        let parent = temp_parent("symlink-dir");
        let target = parent.join("target");
        std::fs::create_dir(&target).unwrap();
        let linked = parent.join("keys");
        symlink(&target, &linked).unwrap();
        assert!(matches!(
            FileKeyStore::new(&linked).load_or_generate(),
            Err(KeyStoreError::UnsafeDirectory)
        ));

        let parent = temp_parent("symlink-file");
        let keys = parent.join("keys");
        std::fs::create_dir(&keys).unwrap();
        let target = parent.join("outside");
        std::fs::write(&target, [7_u8; 32]).unwrap();
        symlink(&target, keys.join(DEVICE_KEY_FILE)).unwrap();
        assert!(matches!(
            FileKeyStore::new(keys).load_or_generate(),
            Err(KeyStoreError::UnsafeFile)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn malformed_and_all_zero_files_fail_closed() {
        let parent = temp_parent("malformed");
        let store = FileKeyStore::new(parent.join("keys"));
        std::fs::create_dir(store.directory()).unwrap();
        std::fs::write(store.key_path(), [1_u8; 31]).unwrap();
        assert!(matches!(
            store.load_or_generate(),
            Err(KeyStoreError::InvalidLength(31))
        ));

        std::fs::write(store.key_path(), [0_u8; 32]).unwrap();
        assert!(matches!(
            store.load_or_generate(),
            Err(KeyStoreError::AllZero)
        ));
    }

    #[cfg(not(unix))]
    #[test]
    fn file_store_fails_closed_without_restricted_acl_support() {
        let store = FileKeyStore::new("unused");
        assert!(matches!(
            store.load_or_generate(),
            Err(KeyStoreError::UnsupportedPlatform)
        ));
    }
}
