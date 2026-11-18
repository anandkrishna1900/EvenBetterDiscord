//! Local marketplace packages, integrity checks, and publisher trust.
//!
//! Hashes detect corruption or tampering; signatures bind a package to a
//! publisher key in an explicit trust store. A package is never executed by
//! this crate — installation only validates and copies data.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PACKAGE_FILE: &str = "package.json";
pub const SIGNATURE_FILE: &str = "signature.sig";
pub const TRUST_FILE: &str = "trust.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageKind {
    Plugin,
    Theme,
}

impl PackageKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            PackageKind::Plugin => "plugin",
            PackageKind::Theme => "theme",
        }
    }

    pub const fn manifest_file(self) -> &'static str {
        match self {
            PackageKind::Plugin => "plugin.json",
            PackageKind::Theme => "theme.json",
        }
    }
}

/// Describes a package and the hashes of its content files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageManifest {
    pub id: String,
    pub version: String,
    pub kind: PackageKind,
    pub publisher: String,
    /// Relative file path to expected lowercase hex SHA-256 digest.
    pub files: BTreeMap<String, String>,
}

/// A verified, installed package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledPackage {
    pub id: String,
    pub version: String,
    pub path: PathBuf,
}

/// Publisher keys the user has explicitly trusted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustStore {
    #[serde(default)]
    publishers: BTreeMap<String, String>,
}

impl TrustStore {
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        match fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map_err(|error| RegistryError::Malformed(error.to_string())),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(RegistryError::Io(path.to_owned(), error)),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), RegistryError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|error| RegistryError::Io(parent.to_owned(), error))?;
            }
        }
        let contents = serde_json::to_string_pretty(self)
            .map_err(|error| RegistryError::Malformed(error.to_string()))?;
        fs::write(path, contents).map_err(|error| RegistryError::Io(path.to_owned(), error))
    }

    /// Adds or replaces a publisher key (lowercase hex, 32 bytes).
    pub fn trust(&mut self, publisher: &str, key_hex: &str) -> Result<(), RegistryError> {
        let bytes: [u8; 32] = decode_hex(key_hex, 32, "publisher key")?
            .try_into()
            .map_err(|_| RegistryError::InvalidKey("publisher key has the wrong length".into()))?;
        let _ = VerifyingKey::from_bytes(&bytes)
            .map_err(|_| RegistryError::InvalidKey("not a valid ed25519 public key".into()))?;
        self.publishers
            .insert(publisher.to_owned(), hex::encode(bytes));
        Ok(())
    }

    pub fn publishers(&self) -> &BTreeMap<String, String> {
        &self.publishers
    }

    fn key(&self, publisher: &str) -> Result<VerifyingKey, RegistryError> {
        let hex_key = self
            .publishers
            .get(publisher)
            .ok_or_else(|| RegistryError::UntrustedPublisher(publisher.to_owned()))?;
        let bytes: [u8; 32] = decode_hex(hex_key, 32, "trusted publisher key")?
            .try_into()
            .map_err(|_| RegistryError::InvalidKey("stored key has the wrong length".into()))?;
        VerifyingKey::from_bytes(&bytes)
            .map_err(|_| RegistryError::InvalidKey("stored key is invalid".into()))
    }
}

/// Installs and removes packages under one local root.
#[derive(Debug, Clone)]
pub struct Registry {
    root: PathBuf,
    trust: TrustStore,
}

impl Registry {
    pub fn new(root: PathBuf, trust: TrustStore) -> Self {
        Self { root, trust }
    }

    /// Validates a package directory without changing anything.
    pub fn verify(&self, package_dir: &Path) -> Result<PackageManifest, RegistryError> {
        verify_package(package_dir, &self.trust)
    }

    /// Installs a verified package. Existing versions of the same release are left alone.
    pub fn install(&self, package_dir: &Path) -> Result<InstalledPackage, RegistryError> {
        let manifest = self.verify(package_dir)?;
        let destination = self
            .root
            .join("packages")
            .join(&manifest.id)
            .join(&manifest.version);
        if destination.exists() {
            return Err(RegistryError::AlreadyInstalled(
                manifest.id.clone(),
                destination,
            ));
        }

        fs::create_dir_all(&destination)
            .map_err(|error| RegistryError::Io(destination.clone(), error))?;
        let install_result = (|| -> Result<(), RegistryError> {
            for file in manifest.files.keys() {
                let source = package_dir.join(file);
                let target = destination.join(file);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| RegistryError::Io(parent.to_owned(), error))?;
                }
                fs::copy(&source, &target)
                    .map_err(|error| RegistryError::Io(source.clone(), error))?;
            }
            for metadata in [PACKAGE_FILE, SIGNATURE_FILE] {
                fs::copy(package_dir.join(metadata), destination.join(metadata))
                    .map_err(|error| RegistryError::Io(PathBuf::from(metadata), error))?;
            }
            Ok(())
        })();

        if let Err(error) = install_result {
            let _ = fs::remove_dir_all(&destination);
            return Err(error);
        }

        Ok(InstalledPackage {
            id: manifest.id,
            version: manifest.version,
            path: destination,
        })
    }

    /// Lists installed packages, newest version first per id.
    pub fn list(&self) -> Result<Vec<InstalledPackage>, RegistryError> {
        let packages_root = self.root.join("packages");
        if !packages_root.exists() {
            return Ok(Vec::new());
        }

        let mut installed = Vec::new();
        for id_dir in read_dirs(&packages_root)? {
            for version_dir in read_dirs(&id_dir)? {
                let manifest_path = version_dir.join(PACKAGE_FILE);
                if !manifest_path.exists() {
                    continue;
                }
                let contents = fs::read_to_string(&manifest_path)
                    .map_err(|error| RegistryError::Io(manifest_path.clone(), error))?;
                let manifest: PackageManifest = serde_json::from_str(&contents)
                    .map_err(|error| RegistryError::Malformed(error.to_string()))?;
                installed.push(InstalledPackage {
                    id: manifest.id,
                    version: manifest.version,
                    path: version_dir,
                });
            }
        }
        installed.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| compare_versions(&right.version, &left.version))
        });
        Ok(installed)
    }

    /// Installs the new version, then removes older versions of the same id.
    pub fn update(&self, package_dir: &Path) -> Result<InstalledPackage, RegistryError> {
        let installed = self.install(package_dir)?;
        for existing in self.list()? {
            if existing.id == installed.id
                && existing.path != installed.path
                && compare_versions(&installed.version, &existing.version)
                    == std::cmp::Ordering::Greater
            {
                fs::remove_dir_all(&existing.path)
                    .map_err(|error| RegistryError::Io(existing.path.clone(), error))?;
            }
        }
        Ok(installed)
    }

    /// Removes every installed version of `id`.
    pub fn uninstall(&self, id: &str) -> Result<(), RegistryError> {
        let path = self.root.join("packages").join(id);
        if !path.exists() {
            return Err(RegistryError::NotFound(id.to_owned()));
        }
        fs::remove_dir_all(&path).map_err(|error| RegistryError::Io(path, error))
    }
}

/// Verifies metadata, signature, and content hashes of a package directory.
pub fn verify_package(
    package_dir: &Path,
    trust: &TrustStore,
) -> Result<PackageManifest, RegistryError> {
    let manifest_path = package_dir.join(PACKAGE_FILE);
    let raw = fs::read(&manifest_path).map_err(|error| RegistryError::Io(manifest_path, error))?;
    let manifest: PackageManifest = serde_json::from_slice(&raw)
        .map_err(|error| RegistryError::Malformed(error.to_string()))?;

    validate_id(&manifest.id)?;
    validate_semver(&manifest.version)?;
    if manifest.publisher.trim().is_empty() || manifest.publisher.len() > 64 {
        return Err(RegistryError::InvalidField("publisher"));
    }
    if manifest.files.is_empty() {
        return Err(RegistryError::InvalidField("files"));
    }
    if !manifest.files.contains_key(manifest.kind.manifest_file()) {
        return Err(RegistryError::InvalidField("files"));
    }

    verify_signature(package_dir, &raw, &manifest, trust)?;

    for (file, expected) in &manifest.files {
        validate_relative_path(file)?;
        let path = package_dir.join(file);
        let actual = hash_file(&path)?;
        if !expected.eq_ignore_ascii_case(&actual) {
            return Err(RegistryError::HashMismatch {
                file: file.clone(),
                expected: expected.clone(),
                actual,
            });
        }
    }
    Ok(manifest)
}

fn verify_signature(
    package_dir: &Path,
    manifest_bytes: &[u8],
    manifest: &PackageManifest,
    trust: &TrustStore,
) -> Result<(), RegistryError> {
    let key = trust.key(&manifest.publisher)?;
    let path = package_dir.join(SIGNATURE_FILE);
    let contents = fs::read_to_string(&path).map_err(|error| RegistryError::Io(path, error))?;
    let bytes: [u8; 64] = decode_hex(contents.trim(), 64, "signature")?
        .try_into()
        .map_err(|_| RegistryError::InvalidKey("signature has the wrong length".into()))?;
    let signature = Signature::from_bytes(&bytes);
    key.verify(manifest_bytes, &signature)
        .map_err(|_| RegistryError::BadSignature)
}

/// Signs the canonical bytes of a package manifest. Useful for publishers and tests.
pub fn sign_manifest(manifest: &PackageManifest, key: &SigningKey) -> String {
    let bytes = serde_json::to_vec(manifest).expect("package manifest is serialisable");
    hex::encode(key.sign(&bytes).to_bytes())
}

pub fn hash_file(path: &Path) -> Result<String, RegistryError> {
    let bytes = fs::read(path).map_err(|error| RegistryError::Io(path.to_owned(), error))?;
    Ok(hex::encode(Sha256::digest(&bytes)))
}

fn decode_hex(value: &str, expected_bytes: usize, what: &str) -> Result<Vec<u8>, RegistryError> {
    let bytes = hex::decode(value.trim())
        .map_err(|_| RegistryError::InvalidKey(format!("{what} is not valid hex")))?;
    if bytes.len() != expected_bytes {
        return Err(RegistryError::InvalidKey(format!(
            "{what} must be {expected_bytes} bytes"
        )));
    }
    Ok(bytes)
}

fn validate_relative_path(path: &str) -> Result<(), RegistryError> {
    let valid = !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.split('/').any(|part| part.is_empty() || part == "..")
        && path != PACKAGE_FILE
        && path != SIGNATURE_FILE;
    if valid {
        Ok(())
    } else {
        Err(RegistryError::InvalidField("files"))
    }
}

fn validate_id(id: &str) -> Result<(), RegistryError> {
    let valid = (3..=64).contains(&id.len())
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
        });
    if valid {
        Ok(())
    } else {
        Err(RegistryError::InvalidField("id"))
    }
}

fn validate_semver(version: &str) -> Result<(), RegistryError> {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    if valid {
        Ok(())
    } else {
        Err(RegistryError::InvalidField("version"))
    }
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |version: &str| -> Vec<u64> {
        version
            .split(['-', '+'])
            .next()
            .unwrap_or_default()
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect()
    };
    parse(left).cmp(&parse(right))
}

fn read_dirs(path: &Path) -> Result<Vec<PathBuf>, RegistryError> {
    let entries = fs::read_dir(path).map_err(|error| RegistryError::Io(path.to_owned(), error))?;
    Ok(entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect())
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("could not access {0}: {1}")]
    Io(PathBuf, io::Error),
    #[error("invalid package metadata: {0}")]
    Malformed(String),
    #[error("invalid package field: {0}")]
    InvalidField(&'static str),
    #[error("hash mismatch for {file}: expected {expected}, got {actual}")]
    HashMismatch {
        file: String,
        expected: String,
        actual: String,
    },
    #[error("publisher `{0}` is not in the trust store")]
    UntrustedPublisher(String),
    #[error("package signature does not match the trusted publisher key")]
    BadSignature,
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("version {0} is already installed at {1}")]
    AlreadyInstalled(String, PathBuf),
    #[error("package `{0}` is not installed")]
    NotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ebd-registry-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn fixture(publisher: &str) -> (SigningKey, TrustStore) {
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let mut trust = TrustStore::default();
        trust
            .trust(publisher, &hex::encode(key.verifying_key().to_bytes()))
            .expect("trust");
        (key, trust)
    }

    fn write_package(dir: &Path, key: &SigningKey, publisher: &str, body: &str) -> PackageManifest {
        fs::create_dir_all(dir).expect("package dir");
        fs::write(dir.join("plugin.json"), body).expect("plugin.json");
        let manifest = PackageManifest {
            id: "test.plugin".into(),
            version: "1.2.3".into(),
            kind: PackageKind::Plugin,
            publisher: publisher.into(),
            files: BTreeMap::from([(
                "plugin.json".to_owned(),
                hash_file(&dir.join("plugin.json")).expect("hash"),
            )]),
        };
        fs::write(
            dir.join(PACKAGE_FILE),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("package.json");
        fs::write(dir.join(SIGNATURE_FILE), sign_manifest(&manifest, key)).expect("signature");
        manifest
    }

    #[test]
    fn installs_lists_and_uninstalls_a_signed_package() {
        let dir = temp_dir("install");
        let package = dir.join("package");
        let (key, trust) = fixture("publisher");
        write_package(&package, &key, "publisher", r#"{"manifest_version":1}"#);

        let registry = Registry::new(dir.join("root"), trust);
        registry.verify(&package).expect("verify");
        let installed = registry.install(&package).expect("install");
        assert_eq!(installed.id, "test.plugin");
        assert!(installed.path.join("plugin.json").exists());

        let listed = registry.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].version, "1.2.3");

        registry.uninstall("test.plugin").expect("uninstall");
        assert!(registry.list().expect("list after").is_empty());
    }

    #[test]
    fn tampered_files_are_rejected() {
        let dir = temp_dir("tamper");
        let package = dir.join("package");
        let (key, trust) = fixture("publisher");
        write_package(&package, &key, "publisher", r#"{"manifest_version":1}"#);
        fs::write(package.join("plugin.json"), "changed").expect("tamper");

        let registry = Registry::new(dir.join("root"), trust);
        assert!(matches!(
            registry.verify(&package),
            Err(RegistryError::HashMismatch { .. })
        ));
    }

    #[test]
    fn untrusted_publishers_are_rejected() {
        let dir = temp_dir("untrusted");
        let package = dir.join("package");
        let (key, _) = fixture("publisher");
        write_package(&package, &key, "publisher", r#"{"manifest_version":1}"#);

        let registry = Registry::new(dir.join("root"), TrustStore::default());
        assert!(matches!(
            registry.verify(&package),
            Err(RegistryError::UntrustedPublisher(_))
        ));
    }

    #[test]
    fn modified_metadata_invalidates_the_signature() {
        let dir = temp_dir("signature");
        let package = dir.join("package");
        let (key, trust) = fixture("publisher");
        let mut manifest = write_package(&package, &key, "publisher", r#"{"manifest_version":1}"#);
        manifest.version = "9.9.9".into();
        fs::write(
            package.join(PACKAGE_FILE),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("rewrite");

        let registry = Registry::new(dir.join("root"), trust);
        assert!(matches!(
            registry.verify(&package),
            Err(RegistryError::BadSignature)
        ));
    }

    #[test]
    fn path_traversal_is_rejected() {
        let _dir = temp_dir("traversal");
        assert!(validate_relative_path("../evil.wasm").is_err());
        assert!(validate_relative_path("a/../../evil").is_err());
        assert!(validate_relative_path(PACKAGE_FILE).is_err());
        assert!(validate_relative_path("assets/icon.png").is_ok());
    }

    #[test]
    fn update_replaces_older_versions() {
        let dir = temp_dir("update");
        let older_dir = dir.join("older");
        let (key, trust) = fixture("publisher");
        write_package(&older_dir, &key, "publisher", r#"{"manifest_version":1}"#);

        let registry = Registry::new(dir.join("root"), trust);
        registry.install(&older_dir).expect("install older");

        let newer_dir = dir.join("newer");
        fs::create_dir_all(&newer_dir).expect("newer dir");
        fs::write(newer_dir.join("plugin.json"), "v2").expect("plugin.json");
        let mut manifest = registry.verify(&older_dir).expect("old manifest");
        manifest.version = "2.0.0".into();
        manifest.files.insert(
            "plugin.json".into(),
            hash_file(&newer_dir.join("plugin.json")).expect("hash"),
        );
        fs::write(
            newer_dir.join(PACKAGE_FILE),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("package.json");
        fs::write(
            newer_dir.join(SIGNATURE_FILE),
            sign_manifest(&manifest, &key),
        )
        .expect("sig");

        let updated = registry.update(&newer_dir).expect("update");
        assert_eq!(updated.version, "2.0.0");
        let listed = registry.list().expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].version, "2.0.0");
    }
}
