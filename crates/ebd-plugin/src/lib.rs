//! Plugin manifests and explicit permission grants.
//!
//! A manifest describes what a plugin *asks for*. A grant is the (possibly
//! smaller) set the user actually approved. Grants can never add a permission
//! the manifest did not request — escalation must be a deliberate manifest
//! change, not a host-side accident.

use std::collections::BTreeSet;
use std::fmt;

use ebd_contract::Capability;
use serde::{Deserialize, Serialize};

pub const MANIFEST_FILE: &str = "plugin.json";
pub const MANIFEST_VERSION: u32 = 1;

/// Everything a plugin can be allowed to do.
///
/// Deliberately coarse: each permission maps to one contract capability plus
/// `Log` (used for diagnostics). More granular scopes can be added later
/// without changing the manifest format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    ReadCurrentUser,
    ReadGuilds,
    ReadChannels,
    ReadMessages,
    SendMessages,
    OpenExternalUrl,
    Log,
}

impl PluginPermission {
    /// All permissions known to this version of the manifest format.
    pub const ALL: [PluginPermission; 7] = [
        PluginPermission::ReadCurrentUser,
        PluginPermission::ReadGuilds,
        PluginPermission::ReadChannels,
        PluginPermission::ReadMessages,
        PluginPermission::SendMessages,
        PluginPermission::OpenExternalUrl,
        PluginPermission::Log,
    ];

    /// Stable identifier used in manifests and output.
    pub const fn as_str(self) -> &'static str {
        match self {
            PluginPermission::ReadCurrentUser => "read_current_user",
            PluginPermission::ReadGuilds => "read_guilds",
            PluginPermission::ReadChannels => "read_channels",
            PluginPermission::ReadMessages => "read_messages",
            PluginPermission::SendMessages => "send_messages",
            PluginPermission::OpenExternalUrl => "open_external_url",
            PluginPermission::Log => "log",
        }
    }

    /// The contract capability this permission permits, if any.
    pub const fn capability(self) -> Option<Capability> {
        match self {
            PluginPermission::ReadCurrentUser => Some(Capability::ReadCurrentUser),
            PluginPermission::ReadGuilds => Some(Capability::ReadGuilds),
            PluginPermission::ReadChannels => Some(Capability::ReadChannels),
            PluginPermission::ReadMessages => Some(Capability::ReadMessages),
            PluginPermission::SendMessages => Some(Capability::SendMessages),
            PluginPermission::OpenExternalUrl => Some(Capability::OpenExternalUrl),
            PluginPermission::Log => None,
        }
    }

    /// Permission required to use `capability`.
    pub const fn for_capability(capability: Capability) -> Self {
        match capability {
            Capability::ReadCurrentUser => PluginPermission::ReadCurrentUser,
            Capability::ReadGuilds => PluginPermission::ReadGuilds,
            Capability::ReadChannels => PluginPermission::ReadChannels,
            Capability::ReadMessages => PluginPermission::ReadMessages,
            Capability::SendMessages => PluginPermission::SendMessages,
            Capability::OpenExternalUrl => PluginPermission::OpenExternalUrl,
        }
    }
}

impl fmt::Display for PluginPermission {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A plugin manifest, parsed from `plugin.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    /// Format version; must be [`MANIFEST_VERSION`].
    pub manifest_version: u32,
    /// Stable, lowercase identifier used for storage and grants.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semver-like version string.
    pub version: String,
    /// Optional short description.
    #[serde(default)]
    pub description: Option<String>,
    /// Relative path to the WASM module inside the package.
    pub entry: String,
    /// Permissions the plugin requests.
    #[serde(default)]
    pub requested: Vec<PluginPermission>,
}

impl PluginManifest {
    /// Parses and validates a manifest.
    pub fn parse(contents: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(contents).map_err(ManifestError::Malformed)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Checks all structural rules; call this after constructing a manifest in code too.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedVersion(self.manifest_version));
        }
        validate_id(&self.id)?;
        validate_text("name", &self.name, 1, 80)?;
        validate_semver(&self.version)?;
        if let Some(description) = &self.description {
            validate_text("description", description, 0, 500)?;
        }
        validate_entry(&self.entry)?;

        let mut seen = BTreeSet::new();
        for permission in &self.requested {
            if !seen.insert(*permission) {
                return Err(ManifestError::DuplicatePermission(*permission));
            }
        }
        Ok(())
    }

    /// Creates a grant with exactly the requested permissions.
    pub fn default_grant(&self) -> PluginGrant {
        PluginGrant {
            plugin_id: self.id.clone(),
            permissions: self.requested.iter().copied().collect(),
        }
    }
}

fn validate_id(id: &str) -> Result<(), ManifestError> {
    if id.len() < 3 || id.len() > 64 {
        return Err(ManifestError::InvalidField(
            "id",
            "must be between 3 and 64 characters",
        ));
    }
    if !id.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b'.'
    }) {
        return Err(ManifestError::InvalidField(
            "id",
            "only lowercase letters, digits, '-' and '.' are allowed",
        ));
    }
    if id.starts_with('.') || id.ends_with('.') || id.starts_with('-') || id.ends_with('-') {
        return Err(ManifestError::InvalidField(
            "id",
            "must not start or end with '.' or '-'",
        ));
    }
    Ok(())
}

fn validate_text(
    field: &'static str,
    value: &str,
    min: usize,
    max: usize,
) -> Result<(), ManifestError> {
    let length = value.trim().len();
    if length < min || value.len() > max {
        return Err(ManifestError::InvalidField(
            field,
            "length is outside the allowed range",
        ));
    }
    Ok(())
}

fn validate_semver(version: &str) -> Result<(), ManifestError> {
    let core = version.split(['-', '+']).next().unwrap_or_default();
    let parts: Vec<&str> = core.split('.').collect();
    let valid = parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
    if valid {
        Ok(())
    } else {
        Err(ManifestError::InvalidField(
            "version",
            "expected major.minor.patch",
        ))
    }
}

fn validate_entry(entry: &str) -> Result<(), ManifestError> {
    let valid = !entry.is_empty()
        && !entry.starts_with('/')
        && !entry.contains('\\')
        && !entry.split('/').any(|part| part == ".." || part.is_empty())
        && entry.ends_with(".wasm");
    if valid {
        Ok(())
    } else {
        Err(ManifestError::InvalidField(
            "entry",
            "expected a relative .wasm path without '..' segments",
        ))
    }
}

/// The permission set a user approved for one plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginGrant {
    plugin_id: String,
    permissions: BTreeSet<PluginPermission>,
}

impl PluginGrant {
    /// Builds a grant from an explicit set. Use [`PluginManifest::default_grant`]
    /// for the common case of approving everything the plugin asked for.
    pub fn new(
        plugin_id: impl Into<String>,
        permissions: impl IntoIterator<Item = PluginPermission>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            permissions: permissions.into_iter().collect(),
        }
    }

    /// Validates a user-edited grant against what the manifest requested.
    pub fn checked(
        manifest: &PluginManifest,
        permissions: impl IntoIterator<Item = PluginPermission>,
    ) -> Result<Self, ManifestError> {
        let requested: BTreeSet<_> = manifest.requested.iter().copied().collect();
        let mut grant = Self::new(manifest.id.clone(), permissions);

        for permission in &grant.permissions {
            if !requested.contains(permission) {
                return Err(ManifestError::PermissionNotRequested(*permission));
            }
        }
        if grant.plugin_id != manifest.id {
            return Err(ManifestError::InvalidField(
                "plugin_id",
                "grant does not match the manifest",
            ));
        }
        grant
            .permissions
            .retain(|permission| requested.contains(permission));
        Ok(grant)
    }

    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    pub fn permissions(&self) -> &BTreeSet<PluginPermission> {
        &self.permissions
    }

    pub fn allows(&self, permission: PluginPermission) -> bool {
        self.permissions.contains(&permission)
    }

    /// Whether this grant permits use of a contract capability.
    pub fn allows_capability(&self, capability: Capability) -> bool {
        self.allows(PluginPermission::for_capability(capability))
    }
}

/// Errors while parsing or validating manifests and grants.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("invalid plugin manifest JSON: {0}")]
    Malformed(serde_json::Error),
    #[error("unsupported manifest version {0}; expected {MANIFEST_VERSION}")]
    UnsupportedVersion(u32),
    #[error("invalid manifest field `{0}`: {1}")]
    InvalidField(&'static str, &'static str),
    #[error("duplicate requested permission: {0}")]
    DuplicatePermission(PluginPermission),
    #[error("grant includes `{0}`, which the manifest did not request")]
    PermissionNotRequested(PluginPermission),
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "manifest_version": 1,
        "id": "example.plugin",
        "name": "Example Plugin",
        "version": "1.2.3",
        "entry": "plugin.wasm",
        "requested": ["log", "read_current_user"]
    }"#;

    fn manifest() -> PluginManifest {
        PluginManifest::parse(VALID).expect("valid manifest")
    }

    #[test]
    fn parses_a_valid_manifest() {
        let manifest = manifest();
        assert_eq!(manifest.id, "example.plugin");
        assert_eq!(manifest.requested.len(), 2);
    }

    #[test]
    fn rejects_bad_versions_and_identifiers() {
        assert!(matches!(
            PluginManifest::parse(
                &VALID.replace("\"manifest_version\": 1", "\"manifest_version\": 2")
            ),
            Err(ManifestError::UnsupportedVersion(2))
        ));
        assert!(matches!(
            PluginManifest::parse(&VALID.replace("example.plugin", "Bad ID!")),
            Err(ManifestError::InvalidField("id", _))
        ));
    }

    #[test]
    fn rejects_absolute_or_traversing_entries() {
        for entry in ["../evil.wasm", "/abs.wasm", "a\\\\b.wasm", "plugin.js", ""] {
            let json = VALID.replace("plugin.wasm", entry);
            assert!(
                matches!(
                    PluginManifest::parse(&json),
                    Err(ManifestError::InvalidField("entry", _))
                ),
                "should reject {entry}"
            );
        }
    }

    #[test]
    fn rejects_unknown_fields_and_duplicate_permissions() {
        let extra = VALID.replace("\"entry\"", "\"surprise\": true, \"entry\"");
        assert!(matches!(
            PluginManifest::parse(&extra),
            Err(ManifestError::Malformed(_))
        ));

        let duplicate = VALID.replace("[\"log\", \"read_current_user\"]", "[\"log\", \"log\"]");
        assert!(matches!(
            PluginManifest::parse(&duplicate),
            Err(ManifestError::DuplicatePermission(PluginPermission::Log))
        ));
    }

    #[test]
    fn grants_cannot_escalate() {
        let manifest = manifest();
        let grant = manifest.default_grant();
        assert!(grant.allows(PluginPermission::Log));
        assert!(grant.allows_capability(Capability::ReadCurrentUser));
        assert!(!grant.allows_capability(Capability::SendMessages));

        let approved =
            PluginGrant::checked(&manifest, [PluginPermission::Log]).expect("subset grant");
        assert!(approved.allows(PluginPermission::Log));
        assert!(!approved.allows(PluginPermission::ReadCurrentUser));

        assert!(matches!(
            PluginGrant::checked(&manifest, [PluginPermission::SendMessages]),
            Err(ManifestError::PermissionNotRequested(
                PluginPermission::SendMessages
            ))
        ));
    }

    #[test]
    fn capability_mapping_is_total() {
        for capability in Capability::ALL {
            assert_eq!(
                PluginPermission::for_capability(capability).capability(),
                Some(capability)
            );
        }
    }
}
