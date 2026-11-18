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
