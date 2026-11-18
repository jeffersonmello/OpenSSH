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

pub fn import_key(storage: &Storage, name: &str, source: &Path) -> Result<Identity> {
    validate_name(name)?;
    let metadata =
        fs::symlink_metadata(source).context("Não foi possível abrir a chave de origem")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1_048_576 {
        bail!("Selecione um arquivo regular de chave privada de até 1 MiB");
    }
    secure_directory(&storage.keys_dir)?;
    let id = new_id();
    let path = storage.keys_dir.join(&id);
    let result = (|| -> Result<Identity> {
        let mut input = fs::File::open(source)?;
        let mut bytes = Zeroizing::new(Vec::new());
        Read::by_ref(&mut input)
            .take(1_048_577)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 1_048_576 {
            bail!("A chave excede 1 MiB");
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&path)?;
        output.write_all(&bytes)?;
        output.sync_all()?;
        identity_from_file(id.clone(), name, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result
}

pub fn generate_key(storage: &Storage, name: &str) -> Result<Identity> {
    validate_name(name)?;
    secure_directory(&storage.keys_dir)?;
    let id = new_id();
    let path = storage.keys_dir.join(&id);
    // A fresh UUID filename isolates the identity from the user-facing label.
    if path.exists() {
        bail!("O arquivo da identidade já existe");
    }
    let result = (|| -> Result<Identity> {
        let output = Command::new(ssh_keygen())
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "openssh-manager"])
            .arg("-f")
            .arg(&path)
            .stdin(Stdio::null())
            .output()
            .context("Não foi possível executar ssh-keygen")?;
        if !output.status.success() {
            bail!("ssh-keygen não conseguiu gerar a chave Ed25519");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
            fs::set_permissions(
                path.with_extension("pub"),
                fs::Permissions::from_mode(0o600),
            )?;
        }
        identity_from_file(id.clone(), name, &path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("pub"));
    }
    result
}

fn validate_name(name: &str) -> Result<()> {
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        bail!("Informe um nome válido para a identidade");
    }
    Ok(())
}

fn identity_from_file(id: String, name: &str, path: &Path) -> Result<Identity> {
    // Empty passphrase makes encrypted imports fail promptly, without opening a
    // prompt or placing a nonempty secret in argv. The original remains intact.
    let output = Command::new(ssh_keygen())
        .args(["-y", "-P", ""])
        .arg("-f")
        .arg(path)
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        bail!("Chave inválida ou protegida por passphrase. Nesta versão, importe uma chave sem passphrase ou use seu ssh-agent diretamente.");
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
