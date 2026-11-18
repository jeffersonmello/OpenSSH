//! SSH identities are private files (0600); passwords use the native Keychain.
use crate::models::{new_id, Identity};
use crate::storage::{secure_directory, Storage};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use zeroize::Zeroizing;

#[cfg(target_os = "macos")]
const SERVICE: &str = "io.github.jeffersonmello.openssh-manager";

#[cfg(target_os = "macos")]
pub fn set_password(host_id: &str, password: &str) -> Result<()> {
    keyring::Entry::new(SERVICE, host_id)?
        .set_password(password)
        .context("Não foi possível salvar a senha no Keychain")
}

#[cfg(target_os = "macos")]
pub fn get_password(host_id: &str) -> Result<String> {
    keyring::Entry::new(SERVICE, host_id)?
        .get_password()
        .context("Não foi possível ler a senha do Keychain")
}

#[cfg(target_os = "macos")]
pub fn delete_password(host_id: &str) -> Result<()> {
    match keyring::Entry::new(SERVICE, host_id)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(error).context("Não foi possível remover a senha do Keychain"),
    }
}

// Never silently use keyring's in-memory mock backend outside the supported OS.
#[cfg(not(target_os = "macos"))]
pub fn set_password(_: &str, _: &str) -> Result<()> {
    bail!("Keychain disponível apenas no macOS nesta versão")
}
#[cfg(not(target_os = "macos"))]
pub fn get_password(_: &str) -> Result<String> {
    bail!("Keychain disponível apenas no macOS nesta versão")
}
#[cfg(not(target_os = "macos"))]
pub fn delete_password(_: &str) -> Result<()> {
    bail!("Keychain disponível apenas no macOS nesta versão")
}

/// Passphrases have their own account namespace, distinct from host passwords.
pub fn set_identity_passphrase(identity_id: &str, passphrase: &str) -> Result<()> {
    set_password(&format!("identity:{identity_id}"), passphrase)
}

pub fn get_identity_passphrase(identity_id: &str) -> Result<String> {
    get_password(&format!("identity:{identity_id}"))
}

pub fn delete_identity_passphrase(identity_id: &str) -> Result<()> {
    delete_password(&format!("identity:{identity_id}"))
}

/// Validate against a bounded, regular file in this app's managed key store.
/// No key is created, decrypted on disk, or passed to another process.
pub fn validate_identity_passphrase(identity: &Identity, passphrase: &str) -> Result<()> {
    validate_identity_passphrase_in(&Storage::discover()?, identity, passphrase)
}

fn validate_identity_passphrase_in(
    storage: &Storage,
    identity: &Identity,
    passphrase: &str,
) -> Result<()> {
    let path = Path::new(&identity.path);
    let metadata =
        fs::symlink_metadata(path).context("Não foi possível abrir a chave gerenciada")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_KEY_BYTES as u64
    {
        bail!("A identidade deve apontar para um arquivo privado regular de até 1 MiB");
    }
    let managed = storage.keys_dir.canonicalize()?;
    let canonical = path.canonicalize()?;
    if canonical.parent() != Some(managed.as_path()) {
        bail!("O arquivo da identidade não pertence ao diretório de chaves gerenciadas");
    }
    let mut bytes = Zeroizing::new(Vec::new());
    fs::File::open(path)?
        .take(MAX_KEY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_KEY_BYTES {
        bail!("A chave excede 1 MiB");
    }
    let key = ssh_key::PrivateKey::from_openssh(&*bytes)
        .map_err(|_| anyhow::anyhow!("Chave OpenSSH gerenciada inválida"))?;
    if !key.is_encrypted() {
        bail!("Esta chave não é protegida por passphrase");
    }
    validate_encryption_cost(&key)?;
    drop(
        key.decrypt(passphrase)
            .map_err(|_| anyhow::anyhow!("Passphrase incorreta ou chave protegida inválida"))?,
    );
    Ok(())
}

pub fn import_key(storage: &Storage, name: &str, source: &Path) -> Result<Identity> {
    import_key_with_passphrase(storage, name, source, None)
}

pub fn import_key_with_passphrase(
    storage: &Storage,
    name: &str,
    source: &Path,
    passphrase: Option<&str>,
) -> Result<Identity> {
    validate_name(name)?;
    let metadata =
        fs::symlink_metadata(source).context("Não foi possível abrir a chave de origem")?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_KEY_BYTES as u64
    {
        bail!("Selecione um arquivo regular de chave privada de até 1 MiB");
    }
    let mut input = fs::File::open(source)?;
    let mut bytes = Zeroizing::new(Vec::new());
    Read::by_ref(&mut input)
        .take(MAX_KEY_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    import_bytes(storage, name, &bytes, passphrase)
}

pub fn import_key_text(
    storage: &Storage,
    name: &str,
    text: &str,
    passphrase: Option<&str>,
) -> Result<Identity> {
    import_bytes(storage, name, text.as_bytes(), passphrase)
}

pub fn generate_key(storage: &Storage, name: &str) -> Result<Identity> {
    generate_key_with_passphrase(storage, name, None)
}

pub fn generate_key_with_passphrase(
    storage: &Storage,
    name: &str,
    passphrase: Option<&str>,
) -> Result<Identity> {
    use ssh_key::{rand_core::OsRng, Algorithm, LineEnding, PrivateKey};
    validate_name(name)?;
    let private = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)
        .context("Não foi possível gerar a chave Ed25519")?;
    let private = match passphrase.filter(|value| !value.is_empty()) {
        Some(value) => private
            .encrypt(&mut OsRng, value)
            .context("Não foi possível proteger a chave com a passphrase")?,
        None => private,
    };
    let encoded = private.to_openssh(LineEnding::LF)?;
    let id = new_id();
    let identity = identity_from_key(id, name, &private)?;
    persist_key(storage, identity, encoded.as_bytes())
}

const MAX_KEY_BYTES: usize = 1_048_576;

fn import_bytes(
    storage: &Storage,
    name: &str,
    bytes: &[u8],
    passphrase: Option<&str>,
) -> Result<Identity> {
    validate_name(name)?;
    if bytes.len() > MAX_KEY_BYTES {
        bail!("A chave excede 1 MiB");
    }
    let text = std::str::from_utf8(bytes)
        .context("A chave precisa estar em formato de texto OpenSSH ou PEM")?;
    match ssh_key::PrivateKey::from_openssh(text.trim()) {
        Ok(private) => {
            if private.is_encrypted() {
                // Bound user-supplied KDF work before decrypting an imported file.
                // Generation uses ssh-key's default bcrypt work factor (16).
                validate_encryption_cost(&private)?;
                let passphrase = passphrase
                    .filter(|value| !value.is_empty())
                    .context("Informe a passphrase para validar esta chave protegida")?;
                // Parse/decrypt first. No private destination exists on failure.
                // The decrypted key is zeroized by ssh-key on drop; persist only
                // the original encrypted input, never this decrypted value.
                drop(private.decrypt(passphrase).map_err(|_| {
                    anyhow::anyhow!("Passphrase incorreta ou chave protegida inválida")
                })?);
            }
            let identity = identity_from_key(new_id(), name, &private)?;
            // Normalize only PEM armor whitespace for the system SSH client:
            // it rejects leading blank lines and a missing final LF. The
            // encrypted key payload/cipher/KDF are preserved unchanged.
            let mut normalized = Zeroizing::new(text.trim().replace("\r\n", "\n"));
            normalized.push('\n');
            persist_key(storage, identity, normalized.as_bytes())
        }
        Err(_) => {
            // Preserve support for unencrypted PEM identities accepted by the
            // system client. No nonempty passphrase ever enters CLI arguments.
            let legacy = [
                "-----BEGIN RSA PRIVATE KEY-----",
                "-----BEGIN EC PRIVATE KEY-----",
                "-----BEGIN DSA PRIVATE KEY-----",
                "-----BEGIN PRIVATE KEY-----",
            ]
            .iter()
            .any(|header| text.trim_start().starts_with(header));
            if !legacy
                || text.contains("ENCRYPTED")
                || passphrase.is_some_and(|value| !value.is_empty())
            {
                bail!("Chave inválida ou formato não suportado. Use OpenSSH para importar uma chave protegida por passphrase.");
            }
            import_legacy(storage, name, bytes)
        }
    }
}

fn validate_encryption_cost(key: &ssh_key::PrivateKey) -> Result<()> {
    if let ssh_key::Kdf::Bcrypt { rounds, .. } = key.kdf() {
        if *rounds > 1024 {
            bail!("A chave usa mais de 1024 rodadas bcrypt; reduza o custo em uma ferramenta confiável antes de importar.");
        }
    }
    Ok(())
}

fn identity_from_key(id: String, name: &str, key: &ssh_key::PrivateKey) -> Result<Identity> {
    Ok(Identity {
        id,
        name: name.trim().into(),
        path: String::new(),
        public_key: key.public_key().to_openssh()?,
        fingerprint: key.fingerprint(ssh_key::HashAlg::Sha256).to_string(),
        algorithm: key.algorithm().as_str().to_owned(),
        encrypted: key.is_encrypted(),
    })
}

fn persist_key(storage: &Storage, mut identity: Identity, bytes: &[u8]) -> Result<Identity> {
    secure_directory(&storage.keys_dir)?;
    let path = storage.keys_dir.join(&identity.id);
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    // Create-new success is tracked so an error can never remove someone
    // else's existing file in the extraordinarily unlikely UUID collision.
    let mut output = options
        .open(&path)
        .context("Não foi possível criar o arquivo privado da identidade")?;
    let result = output.write_all(bytes).and_then(|_| output.sync_all());
    if let Err(error) = result {
        drop(output);
        let _ = fs::remove_file(&path);
        return Err(error.into());
    }
    identity.path = path.to_string_lossy().into_owned();
    Ok(identity)
}

fn import_legacy(storage: &Storage, name: &str, bytes: &[u8]) -> Result<Identity> {
    let identity = Identity {
        id: new_id(),
        name: name.trim().into(),
        path: String::new(),
        public_key: String::new(),
        fingerprint: String::new(),
        algorithm: String::new(),
        encrypted: false,
    };
    let copied = persist_key(storage, identity, bytes)?;
    let result = identity_from_legacy_file(copied.id, name, Path::new(&copied.path));
    if result.is_err() {
        let _ = fs::remove_file(&copied.path);
    }
    result
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        bail!("Informe um nome válido para a identidade");
    }
    Ok(())
}

fn identity_from_legacy_file(id: String, name: &str, path: &Path) -> Result<Identity> {
    // Empty passphrase makes encrypted imports fail promptly, without opening a
    // prompt or placing a nonempty secret in argv. The original remains intact.
    let output = Command::new(ssh_keygen())
        .args(["-y", "-P", ""])
        .arg("-f")
        .arg(path)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        bail!("Chave PEM inválida. Para importar uma chave protegida, converta-a para OpenSSH em uma ferramenta confiável.");
    }
    let public_key = String::from_utf8(output.stdout)?.trim().to_owned();
    let mut parts = public_key.split_whitespace();
    let algorithm = parts
        .next()
        .context("Algoritmo de chave ausente")?
        .to_owned();
    let encoded = parts.next().context("Chave pública ausente")?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(encoded)?;
    let fingerprint = format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(decoded)));
    Ok(Identity {
        id,
        name: name.trim().into(),
        path: path.to_string_lossy().into_owned(),
        public_key,
        fingerprint,
        algorithm,
        encrypted: false,
    })
}

fn ssh_keygen() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "/usr/bin/ssh-keygen"
    }
    #[cfg(not(target_os = "macos"))]
    {
        "ssh-keygen"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrase_validation_reads_only_managed_regular_files_without_writes() {
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::new(temp.path().join("data")).unwrap();
        let identity = generate_key_with_passphrase(
            &storage,
            "Protected fixture",
            Some("ephemeral validation fixture"),
        )
        .unwrap();
        let before = Zeroizing::new(fs::read(&identity.path).unwrap());
        validate_identity_passphrase_in(&storage, &identity, "ephemeral validation fixture")
            .unwrap();
        let wrong =
            validate_identity_passphrase_in(&storage, &identity, "incorrect fixture").unwrap_err();
        assert!(!format!("{wrong:#}").contains("validation fixture"));
        assert_eq!(fs::read(&identity.path).unwrap(), *before);
        assert_eq!(fs::read_dir(&storage.keys_dir).unwrap().count(), 1);

        let outside = temp.path().join("outside-key");
        fs::write(&outside, &*before).unwrap();
        let mut candidate = identity.clone();
        candidate.path = outside.to_string_lossy().into_owned();
        assert!(validate_identity_passphrase_in(
            &storage,
            &candidate,
            "ephemeral validation fixture"
        )
        .unwrap_err()
        .to_string()
        .contains("não pertence"));
        #[cfg(unix)]
        {
            let link = storage.keys_dir.join("symlink");
            std::os::unix::fs::symlink(&identity.path, &link).unwrap();
            candidate.path = link.to_string_lossy().into_owned();
            assert!(validate_identity_passphrase_in(
                &storage,
                &candidate,
                "ephemeral validation fixture"
            )
            .is_err());
        }
    }
}
