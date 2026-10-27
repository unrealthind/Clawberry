//! Named-credential broker. Models can request a capability name but never read
//! its value; only a trusted integration callback receives decrypted material.

use super::secrets::SecretStore;
use crate::assistants::AgentId;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialGrant {
    pub name: String,
    pub agents: Vec<AgentId>,
    pub capabilities: Vec<String>,
    pub encrypted_value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
}
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialManifest {
    pub credentials: BTreeMap<String, CredentialGrant>,
}
pub struct SecretBroker {
    store: SecretStore,
    manifest: CredentialManifest,
    manifest_path: PathBuf,
}
impl SecretBroker {
    pub(crate) fn revoke_reference(&mut self, name: &str) -> Result<()> {
        let _registration = RegistrationLock::acquire(&self.manifest_path.with_extension("lock"))?;
        let mut manifest = self.load_manifest()?;
        manifest.credentials.remove(name);
        let temporary = self
            .manifest_path
            .with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let result = (|| -> Result<()> {
            let mut file = options.open(&temporary)?;
            file.write_all(&serde_json::to_vec(&manifest)?)?;
            file.sync_all()?;
            std::fs::rename(&temporary, &self.manifest_path)?;
            std::fs::File::open(self.manifest_path.parent().unwrap())?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
        self.manifest = manifest;
        Ok(())
    }
    pub fn open(root: &Path, encryption: bool) -> Result<Self> {
        if !encryption {
            bail!("The agent credential broker requires encryption");
        }
        let root_meta = root.symlink_metadata()?;
        if !root_meta.is_dir() || root_meta.file_type().is_symlink() {
            bail!("Credential storage must be a real private directory");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if root_meta.permissions().mode() & 0o077 != 0 {
                bail!("Credential directory requires mode 0700");
            }
        }
        let path = root.join("credential-manifest.json");
        check_private_file(&path)?;
        if path
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 1_048_576)
        {
            bail!("Credential manifest exceeds limit");
        }
        check_private_file(&root.join(".secret_key"))?;
        let manifest = if path.exists() {
            serde_json::from_slice(&std::fs::read(&path)?)?
        } else {
            CredentialManifest::default()
        };
        let broker = Self {
            store: SecretStore::new(root, encryption),
            manifest,
            manifest_path: path,
        };
        for (name, grant) in &broker.manifest.credentials {
            if name != &grant.name
                || !valid_name(name)
                || !SecretStore::is_secure_encrypted(&grant.encrypted_value)
            {
                bail!("Invalid encrypted credential manifest");
            }
        }
        if !broker.manifest.credentials.is_empty() && !root.join(".secret_key").is_file() {
            bail!("Credential key is missing; restore the key rather than creating a replacement");
        }
        Ok(broker)
    }
    pub fn register(
        &mut self,
        name: &str,
        agents: Vec<AgentId>,
        capabilities: Vec<String>,
        raw_value: &str,
    ) -> Result<()> {
        self.register_with_expiry(name, agents, capabilities, raw_value, None)
    }
    pub(crate) fn register_expiring(
        &mut self,
        name: &str,
        agents: Vec<AgentId>,
        capabilities: Vec<String>,
        raw_value: &str,
        expires_at: i64,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        if expires_at <= now || expires_at > now + 300 {
            bail!("Invalid credential session expiry")
        }
        self.register_with_expiry(name, agents, capabilities, raw_value, Some(expires_at))
    }
    fn register_with_expiry(
        &mut self,
        name: &str,
        agents: Vec<AgentId>,
        capabilities: Vec<String>,
        raw_value: &str,
        expires_at: Option<i64>,
    ) -> Result<()> {
        if !valid_name(name)
            || raw_value.is_empty()
            || raw_value.len() > 16_384
            || agents.is_empty()
            || agents.len() > 2
            || capabilities.is_empty()
            || capabilities.len() > 32
            || capabilities
                .iter()
                .any(|value| value.is_empty() || value.len() > 128)
        {
            bail!("Invalid credential grant");
        }
        let _registration = RegistrationLock::acquire(&self.manifest_path.with_extension("lock"))?;
        self.manifest = self.load_manifest()?;
        self.manifest.credentials.retain(|_, grant| {
            grant
                .expires_at
                .is_none_or(|expiry| expiry > chrono::Utc::now().timestamp())
        });
        if !self.manifest.credentials.contains_key(name) && self.manifest.credentials.len() >= 128 {
            bail!("Credential count limit reached");
        }
        let previous = self.manifest.credentials.insert(
            name.into(),
            CredentialGrant {
                name: name.into(),
                agents,
                capabilities,
                encrypted_value: self.store.encrypt(raw_value)?,
                expires_at,
            },
        );
        // Persist only ciphertext and grants.  Raw values never enter this
        // serializable manifest or a model-visible return value.
        let encoded = serde_json::to_vec_pretty(&self.manifest)?;
        if encoded.len() > 1_048_576 {
            if let Some(old) = previous {
                self.manifest.credentials.insert(name.into(), old);
            } else {
                self.manifest.credentials.remove(name);
            }
            bail!("Encrypted credential manifest exceeds limit");
        }
        let temp = self
            .manifest_path
            .with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
        let persisted = (|| -> Result<()> {
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(&encoded)?;
            file.sync_all()?;
            std::fs::rename(&temp, &self.manifest_path)?;
            std::fs::File::open(self.manifest_path.parent().unwrap())?.sync_all()?;
            Ok(())
        })();
        if persisted.is_err() {
            if let Some(old) = previous {
                self.manifest.credentials.insert(name.into(), old);
            } else {
                self.manifest.credentials.remove(name);
            }
            let _ = std::fs::remove_file(&temp);
        }
        persisted?;
        Ok(())
    }
    /// The closure is trusted integration code. Its result must be safe output,
    /// never an echo of the credential; credential material is not serializable.
    pub fn execute_redacted(
        &self,
        agent: AgentId,
        capability: &str,
        name: &str,
        request_id: &str,
        f: impl FnOnce(&str) -> Result<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        uuid::Uuid::parse_str(request_id)?;
        let manifest = self.load_manifest()?;
        let grant = manifest
            .credentials
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Unknown credential reference"))?;
        if !grant.agents.contains(&agent) || !grant.capabilities.iter().any(|c| c == capability) {
            bail!("Credential reference is not granted for this agent capability");
        }
        if grant
            .expires_at
            .is_some_and(|expiry| expiry <= chrono::Utc::now().timestamp())
        {
            bail!("Credential session expired")
        }
        let value = self.store.decrypt(&grant.encrypted_value)?;
        let audit_path = self.manifest_path.with_extension("audit.jsonl");
        check_private_file(&audit_path)?;
        let mut options = std::fs::OpenOptions::new();
        options.append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut audit = options.open(audit_path)?;
        let event = serde_json::json!({"request_id":request_id,"agent":agent,"capability":capability,"credential_reference":name,"event":"credential_use_admitted","at":chrono::Utc::now().to_rfc3339()});
        audit.write_all(format!("{event}\n").as_bytes())?;
        audit.sync_data()?;
        let mut result =
            f(&value).map_err(|_| anyhow::anyhow!("Credential-backed integration failed"))?;
        redact(&mut result, &value);
        Ok(result)
    }
    fn load_manifest(&self) -> Result<CredentialManifest> {
        check_private_file(&self.manifest_path)?;
        if self
            .manifest_path
            .metadata()
            .is_ok_and(|metadata| metadata.len() > 1_048_576)
        {
            bail!("Credential manifest exceeds limit");
        }
        check_private_file(&self.manifest_path.parent().unwrap().join(".secret_key"))?;
        let manifest: CredentialManifest = match std::fs::read(&self.manifest_path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                CredentialManifest::default()
            }
            Err(error) => return Err(error.into()),
        };
        if manifest.credentials.iter().any(|(name, grant)| {
            name != &grant.name
                || !valid_name(name)
                || !SecretStore::is_secure_encrypted(&grant.encrypted_value)
        }) {
            bail!("Invalid encrypted credential manifest");
        }
        if !manifest.credentials.is_empty()
            && !self
                .manifest_path
                .parent()
                .unwrap()
                .join(".secret_key")
                .is_file()
        {
            bail!("Credential key is missing; restore the original key");
        }
        Ok(manifest)
    }
    pub fn references_for(&self, agent: AgentId, capability: &str) -> Result<Vec<String>> {
        Ok(self
            .load_manifest()?
            .credentials
            .values()
            .filter(|g| {
                g.agents.contains(&agent)
                    && g.capabilities.iter().any(|c| c == capability)
                    && g.expires_at
                        .is_none_or(|expiry| expiry > chrono::Utc::now().timestamp())
            })
            .map(|g| g.name.clone())
            .collect())
    }
}
struct RegistrationLock {
    path: PathBuf,
}
impl RegistrationLock {
    fn acquire(path: &Path) -> Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path).map_err(|_| {
            anyhow::anyhow!(
                "Credential registration is active or needs administrator crash recovery"
            )
        })?;
        Ok(Self {
            path: path.to_owned(),
        })
    }
}
impl Drop for RegistrationLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
fn check_private_file(path: &Path) -> Result<()> {
    match path.symlink_metadata() {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_file() {
                bail!("Credential file must not be a symlink or special file");
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o077 != 0 {
                    bail!("Credential file requires mode 0600");
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
pub(crate) fn redact(value: &mut serde_json::Value, secret: &str) {
    use base64::Engine;
    let encodings = [
        secret.to_owned(),
        base64::engine::general_purpose::STANDARD.encode(secret),
        base64::engine::general_purpose::STANDARD_NO_PAD.encode(secret),
        base64::engine::general_purpose::URL_SAFE.encode(secret),
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret),
        hex::encode(secret.as_bytes()),
        hex::encode_upper(secret.as_bytes()),
    ];
    redact_encodings(value, &encodings);
}
fn redact_encodings(value: &mut serde_json::Value, encodings: &[String]) {
    match value {
        serde_json::Value::String(text) => {
            for secret in encodings {
                *text = text.replace(secret, "[REDACTED]");
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_encodings(item, encodings);
            }
        }
        serde_json::Value::Object(object) => {
            let old = std::mem::take(object);
            for (key, mut item) in old {
                redact_encodings(&mut item, encodings);
                let key = encodings
                    .iter()
                    .fold(key, |key, secret| key.replace(secret, "[REDACTED]"));
                let normalized_key = key.replace(['_', '-'], "").to_ascii_lowercase();
                if [
                    "authorization",
                    "password",
                    "token",
                    "secret",
                    "apikey",
                    "accesstoken",
                    "sessiontoken",
                    "refreshtoken",
                    "totp",
                    "totpcode",
                ]
                .contains(&normalized_key.as_str())
                {
                    item = serde_json::Value::String("[REDACTED]".into());
                }
                object.insert(key, item);
            }
        }
        _ => {}
    }
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 80
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    #[test]
    fn known_encoded_secret_echoes_are_redacted() {
        use base64::Engine;
        let secret = "fake-sensitive-credential";
        let mut value = serde_json::json!({"base64":base64::engine::general_purpose::STANDARD.encode(secret),"hex":hex::encode(secret.as_bytes())});
        redact(&mut value, secret);
        assert_eq!(
            value,
            serde_json::json!({"base64":"[REDACTED]","hex":"[REDACTED]"})
        );
    }
    #[test]
    fn padded_unpadded_and_uppercase_encodings_are_redacted_in_nested_keys() {
        use base64::Engine;
        let secret = "fixture-secret-\u{ffff}x";
        for encoded in [
            base64::engine::general_purpose::STANDARD.encode(secret),
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(secret),
            base64::engine::general_purpose::URL_SAFE.encode(secret),
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(secret),
            hex::encode(secret.as_bytes()),
            hex::encode_upper(secret.as_bytes()),
        ] {
            let mut value =
                serde_json::json!({"items":[{encoded.clone():format!("echo:{encoded}")} ]});
            redact(&mut value, secret);
            assert!(!value.to_string().contains(&encoded));
            assert!(value["items"][0].get("[REDACTED]").is_some());
        }
    }
    #[test]
    fn expired_session_credentials_cannot_be_injected() {
        let d = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut broker = SecretBroker::open(d.path(), true).unwrap();
        assert!(broker
            .register_expiring(
                "session",
                vec![AgentId::Janet],
                vec!["n8n.write.session".into()],
                "fake-token",
                0
            )
            .is_err());
    }
    #[test]
    fn revoked_and_expired_bearers_are_inaccessible_to_existing_brokers() {
        let d = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut admin = SecretBroker::open(d.path(), true).unwrap();
        admin
            .register_expiring(
                "session",
                vec![AgentId::Janet],
                vec!["n8n.write.session".into()],
                "fake-token",
                chrono::Utc::now().timestamp() + 60,
            )
            .unwrap();
        let reader = SecretBroker::open(d.path(), true).unwrap();
        assert_eq!(
            reader
                .references_for(AgentId::Janet, "n8n.write.session")
                .unwrap(),
            vec!["session"]
        );
        admin.revoke_reference("session").unwrap();
        assert!(reader
            .references_for(AgentId::Janet, "n8n.write.session")
            .unwrap()
            .is_empty());
        assert!(reader
            .execute_redacted(
                AgentId::Janet,
                "n8n.write.session",
                "session",
                &uuid::Uuid::new_v4().to_string(),
                |_| panic!("Revoked token was injected")
            )
            .is_err());
    }
    #[test]
    fn only_scoped_callback_gets_credential() {
        let d = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut b = SecretBroker::open(d.path(), true).unwrap();
        b.register(
            "pricing_api",
            vec![AgentId::Janet],
            vec!["pricing.read".into()],
            "secret-value",
        )
        .unwrap();
        assert_eq!(
            b.references_for(AgentId::Janet, "pricing.read").unwrap(),
            vec!["pricing_api"]
        );
        assert!(b
            .execute_redacted(
                AgentId::Harley,
                "pricing.read",
                "pricing_api",
                "00000000-0000-0000-0000-000000000001",
                |_| Ok(serde_json::Value::Null)
            )
            .is_err());
        assert_eq!(
            b.execute_redacted(AgentId::Janet, "pricing.read", "pricing_api", "00000000-0000-0000-0000-000000000002", |s| Ok(
                serde_json::json!({"echo":s,"nested":[format!("Bearer {s}")],"token":"another-token"})
            ))
            .unwrap(),
            serde_json::json!({"echo":"[REDACTED]","nested":["Bearer [REDACTED]"],"token":"[REDACTED]"})
        );
        let manifest = std::fs::read_to_string(d.path().join("credential-manifest.json")).unwrap();
        assert!(!manifest.contains("secret-value"));
        let audit =
            std::fs::read_to_string(d.path().join("credential-manifest.audit.jsonl")).unwrap();
        assert!(audit.contains("pricing_api"));
        assert!(!audit.contains("secret-value"));
    }
    #[test]
    fn rotation_and_scope_changes_are_seen_by_existing_instances() {
        let d = TempDir::new().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let mut admin = SecretBroker::open(d.path(), true).unwrap();
        admin
            .register(
                "pricing_api",
                vec![AgentId::Janet],
                vec!["pricing.read".into()],
                "old-value",
            )
            .unwrap();
        let reader = SecretBroker::open(d.path(), true).unwrap();
        admin
            .register(
                "pricing_api",
                vec![AgentId::Harley],
                vec!["pricing.read".into()],
                "new-value",
            )
            .unwrap();
        assert!(reader
            .references_for(AgentId::Janet, "pricing.read")
            .unwrap()
            .is_empty());
        assert!(reader
            .execute_redacted(
                AgentId::Janet,
                "pricing.read",
                "pricing_api",
                "00000000-0000-0000-0000-000000000001",
                |_| Ok(serde_json::Value::Null)
            )
            .is_err());
        assert_eq!(
            reader
                .execute_redacted(
                    AgentId::Harley,
                    "pricing.read",
                    "pricing_api",
                    "00000000-0000-0000-0000-000000000002",
                    |secret| {
                        assert_eq!(secret, "new-value");
                        Ok(serde_json::json!({"value":secret,"sessionToken":"another-token"}))
                    }
                )
                .unwrap(),
            serde_json::json!({"value":"[REDACTED]","sessionToken":"[REDACTED]"})
        );
    }
}
