use openssh_manager::{
    models::{Host, Workspace},
    storage::Storage,
    vault,
};
use std::fs;

#[test]
fn workspace_roundtrips_without_private_key_material_or_passwords() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    assert!(storage.load().unwrap().hosts.is_empty());
    let mut workspace = Workspace::default();
    workspace.hosts.push(Host {
        name: "Development".into(),
        address: "dev.example.com".into(),
        ..Host::default()
    });
    storage.save(&workspace).unwrap();
    let loaded = storage.load().unwrap();
    assert_eq!(loaded.hosts[0].address, "dev.example.com");
    assert_eq!(fs::read_dir(&storage.data_dir).unwrap().count(), 2);
    let json = fs::read_to_string(storage.workspace_path()).unwrap();
    assert!(!json.contains("PRIVATE KEY"));
    assert!(!json.contains("password"));
}

#[test]
fn invalid_and_newer_workspaces_are_never_overwritten() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    for original in [
        "{ broken",
        r#"{"schema_version":99,"hosts":[],"identities":[],"tunnels":[],"snippets":[]}"#,
    ] {
        fs::write(storage.workspace_path(), original).unwrap();
        assert!(storage.load().is_err());
        assert!(storage.save(&Workspace::default()).is_err());
        assert_eq!(
            fs::read_to_string(storage.workspace_path()).unwrap(),
            original
        );
    }
}

#[cfg(unix)]
#[test]
fn metadata_and_keys_directory_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    storage.save(&Workspace::default()).unwrap();
    assert_eq!(
        fs::metadata(&storage.data_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&storage.keys_dir)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(storage.workspace_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[cfg(unix)]
#[test]
fn workspace_symlink_cannot_overwrite_another_file() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let target = temp.path().join("important");
    fs::write(&target, b"preserve").unwrap();
    symlink(&target, storage.workspace_path()).unwrap();
    assert!(storage.save(&Workspace::default()).is_err());
    assert_eq!(fs::read(target).unwrap(), b"preserve");
}

#[test]
fn invalid_import_is_removed_without_changing_original() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let source = temp.path().join("invalid-key");
    fs::write(&source, b"this is not a key").unwrap();
    assert!(vault::import_key(&storage, "test", &source).is_err());
    assert_eq!(fs::read(&source).unwrap(), b"this is not a key");
    assert_eq!(fs::read_dir(&storage.keys_dir).unwrap().count(), 0);
}

#[test]
fn generated_ed25519_key_can_be_imported_with_matching_fingerprint() {
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let generated = vault::generate_key(&storage, "Generated test key").unwrap();
    assert_eq!(generated.algorithm, "ssh-ed25519");
    assert!(generated.fingerprint.starts_with("SHA256:"));
    let imported = vault::import_key(
        &storage,
        "Imported test key",
        std::path::Path::new(&generated.path),
    )
    .unwrap();
    assert_eq!(generated.public_key, imported.public_key);
    assert_eq!(generated.fingerprint, imported.fingerprint);
    assert_ne!(generated.path, imported.path);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&imported.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn encrypted_keys_roundtrip_without_persisting_passphrases_or_decrypted_material() {
    use ssh_key::PrivateKey;
    use zeroize::Zeroizing;
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let passphrase = Zeroizing::new("ephemeral test passphrase — not a real credential".to_owned());
    let generated =
        vault::generate_key_with_passphrase(&storage, "Protected identity", Some(&passphrase))
            .unwrap();
    assert!(generated.encrypted);
    assert_eq!(generated.algorithm_label(), "Ed25519");
    let original = Zeroizing::new(fs::read_to_string(&generated.path).unwrap());
    let parsed = PrivateKey::from_openssh(original.as_str()).unwrap();
    assert!(parsed.is_encrypted());
    let decrypted = parsed.decrypt(passphrase.as_str()).unwrap();
    assert_eq!(
        decrypted.public_key().to_openssh().unwrap(),
        generated.public_key
    );

    let imported = vault::import_key_text(
        &storage,
        "Pasted protected key",
        &original,
        Some(&passphrase),
    )
    .unwrap();
    assert!(imported.encrypted);
    assert_eq!(generated.fingerprint, imported.fingerprint);
    assert_eq!(
        fs::read(&generated.path).unwrap(),
        fs::read(&imported.path).unwrap(),
        "Original encrypted bytes must be preserved"
    );
    let imported_file = vault::import_key_with_passphrase(
        &storage,
        "File protected key",
        std::path::Path::new(&generated.path),
        Some(&passphrase),
    )
    .unwrap();
    assert_eq!(imported_file.fingerprint, generated.fingerprint);

    let mut workspace = Workspace::default();
    workspace.identities.push(imported.clone());
    workspace.hosts.push(Host {
        name: "Secure server".into(),
        address: "server.example.com".into(),
        ..Host::default()
    });
    workspace
        .bind_identity(&imported.id, &[workspace.hosts[0].id.clone()])
        .unwrap();
    storage.save(&workspace).unwrap();
    let json = fs::read_to_string(storage.workspace_path()).unwrap();
    assert!(!json.contains(passphrase.as_str()));
    assert!(!json.contains("PRIVATE KEY"));
    let loaded = storage.load().unwrap();
    assert_eq!(
        loaded.hosts[0].identity_id.as_deref(),
        Some(imported.id.as_str())
    );
    assert!(loaded.identities[0].encrypted);

    // OpenSSH can inspect the encrypted file's public metadata without a secret
    // argv or environment variable; generation/decryption itself uses no CLI.
    let output = std::process::Command::new("ssh-keygen")
        .args(["-l", "-f"])
        .arg(&generated.path)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains(&generated.fingerprint));
}

#[test]
fn incorrect_or_missing_passphrase_never_creates_a_private_destination() {
    use zeroize::Zeroizing;
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let generated = vault::generate_key_with_passphrase(
        &storage,
        "Encrypted",
        Some("ephemeral fixture secret"),
    )
    .unwrap();
    let source = Zeroizing::new(fs::read_to_string(&generated.path).unwrap());
    let before = fs::read_dir(&storage.keys_dir).unwrap().count();
    for candidate in [None, Some("incorrect fixture phrase")] {
        let error =
            vault::import_key_text(&storage, "Invalid import", &source, candidate).unwrap_err();
        assert!(!format!("{error:#}").contains("fixture secret"));
        assert!(!format!("{error:#}").contains("PRIVATE KEY"));
        assert_eq!(fs::read_dir(&storage.keys_dir).unwrap().count(), before);
        assert_eq!(
            fs::read_to_string(&generated.path).unwrap(),
            source.as_str()
        );
    }
}

#[test]
fn identity_binding_validates_atomically_and_preserves_other_assignments() {
    use openssh_manager::models::Protocol;
    let mut workspace = Workspace::demo();
    let production = workspace.identities[0].id.clone();
    let other = workspace.identities[1].id.clone();
    let old_hosts = workspace.hosts.clone();
    let selected = vec![workspace.hosts[1].id.clone(), workspace.hosts[3].id.clone()];
    workspace.bind_identity(&production, &selected).unwrap();
    assert_eq!(workspace.hosts[0].identity_id, None);
    assert_eq!(
        workspace.hosts[1].identity_id.as_deref(),
        Some(production.as_str())
    );
    assert_eq!(
        workspace.hosts[3].identity_id.as_deref(),
        Some(production.as_str())
    );
    assert_eq!(
        workspace.hosts[2].identity_id.as_deref(),
        Some(other.as_str())
    );
    assert_eq!(workspace.hosts[5].identity_id, old_hosts[5].identity_id);

    let before = workspace.hosts.clone();
    assert!(workspace
        .bind_identity(
            &production,
            &[workspace.hosts[0].id.clone(), "unknown-host".into()]
        )
        .is_err());
    assert_eq!(workspace.hosts, before);
    assert!(workspace.bind_identity("unknown-identity", &[]).is_err());
    assert_eq!(workspace.hosts, before);
    let ftp = Host {
        name: "Legacy FTP".into(),
        address: "ftp.example.com".into(),
        protocol: Protocol::Ftp,
        port: 21,
        ..Host::default()
    };
    let ftp_id = ftp.id.clone();
    workspace.hosts.push(ftp);
    let before = workspace.hosts.clone();
    assert!(workspace
        .bind_identity(&production, &[workspace.hosts[0].id.clone(), ftp_id])
        .is_err());
    assert_eq!(workspace.hosts, before);
}

#[test]
fn old_identity_metadata_defaults_to_unencrypted_without_schema_migration() {
    let identity = serde_json::json!({"id":"old-id","name":"Legacy","path":"/tmp/reference-only","public_key":"public metadata","fingerprint":"SHA256:reference","algorithm":"ssh-rsa"});
    let parsed: openssh_manager::models::Identity = serde_json::from_value(identity).unwrap();
    assert!(!parsed.encrypted);
    assert_eq!(parsed.algorithm_label(), "RSA");
    let demo = Workspace::demo();
    assert_eq!(demo.identities.len(), 3);
    assert!(demo
        .identities
        .iter()
        .all(|identity| identity.path.is_empty()));
    assert!(demo.hosts.iter().all(|host| host.identity_id.is_some()));
}

#[test]
fn legacy_unencrypted_pem_import_remains_supported() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("legacy-rsa");
    let output = std::process::Command::new("ssh-keygen")
        .args(["-q", "-t", "rsa", "-b", "2048", "-m", "PEM", "-N", "", "-f"])
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success());
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let imported = vault::import_key(&storage, "Legacy RSA", &source).unwrap();
    assert_eq!(imported.algorithm_label(), "RSA");
    assert!(!imported.encrypted);
    assert_eq!(
        fs::read(&source).unwrap(),
        fs::read(&imported.path).unwrap()
    );
}

#[test]
fn pasted_private_key_armor_is_normalized_for_system_ssh_compatibility() {
    use zeroize::Zeroizing;
    let temp = tempfile::tempdir().unwrap();
    let storage = Storage::new(temp.path().join("data")).unwrap();
    let generated = vault::generate_key(&storage, "Canonical").unwrap();
    let canonical = Zeroizing::new(fs::read_to_string(&generated.path).unwrap());
    for pasted in [
        Zeroizing::new(format!("\n  {}  \n", canonical.trim())),
        Zeroizing::new(canonical.trim_end().to_owned()),
    ] {
        let imported = vault::import_key_text(&storage, "Pasted key", &pasted, None).unwrap();
        let output = std::process::Command::new("ssh-keygen")
            .args(["-y", "-P", "", "-f"])
            .arg(&imported.path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "System SSH must read normalized armor"
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            imported.public_key
        );
        assert_eq!(imported.fingerprint, generated.fingerprint);
    }
}
