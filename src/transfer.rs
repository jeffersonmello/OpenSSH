//! Blocking transfer operations. Call from a worker, never the GUI event loop.
use crate::models::{Host, Identity, Protocol};
use crate::vault;
use anyhow::{bail, Context, Result};
use directories::BaseDirs;
use ssh2::{CheckResult, KnownHostFileKind, OpenFlags, OpenType, Session};
use std::fs::{self, File, OpenOptions};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use suppaftp::{types::FileType, FtpError, FtpStream};
use zeroize::Zeroizing;

const TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub struct RemoteEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub enum TransferAction {
    List { remote: String },
    Download { remote: String, local: PathBuf },
    Upload { local: PathBuf, remote: String },
}

pub fn execute(
    host: &Host,
    identity: Option<&Identity>,
    action: TransferAction,
) -> Result<Vec<RemoteEntry>> {
    host.validate().map_err(anyhow::Error::msg)?;
    let remote = match &action {
        TransferAction::List { remote }
        | TransferAction::Download { remote, .. }
        | TransferAction::Upload { remote, .. } => remote,
    };
    if remote.chars().any(char::is_control) {
        bail!("O caminho remoto contém caracteres de controle");
    }
    match host.protocol {
        Protocol::Ftp => ftp(host, action),
        Protocol::Ssh | Protocol::Sftp => sftp(host, identity, action),
    }
}

fn connect(host: &Host) -> Result<TcpStream> {
    let addresses: Vec<_> = (host.address.as_str(), host.port)
        .to_socket_addrs()
        .context("Não foi possível resolver o endereço do servidor")?
        .collect();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, TIMEOUT) {
            Ok(stream) => {
                stream.set_read_timeout(Some(TIMEOUT))?;
                stream.set_write_timeout(Some(TIMEOUT))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error
        .unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                "Nenhum endereço encontrado",
            )
        })
        .into())
}

fn sftp(
    host: &Host,
    identity: Option<&Identity>,
    action: TransferAction,
) -> Result<Vec<RemoteEntry>> {
    let base = BaseDirs::new().context("Diretório pessoal indisponível")?;
    let known_hosts = base.home_dir().join(".ssh/known_hosts");
    sftp_with_known_hosts(host, identity, action, &known_hosts)
}

fn sftp_with_known_hosts(
    host: &Host,
    identity: Option<&Identity>,
    action: TransferAction,
    known_hosts: &Path,
) -> Result<Vec<RemoteEntry>> {
    sftp_with_known_hosts_and_passphrase(host, identity, action, known_hosts, None)
}

fn sftp_with_known_hosts_and_passphrase(
    host: &Host,
    identity: Option<&Identity>,
    action: TransferAction,
    known_hosts: &Path,
    supplied_passphrase: Option<&str>,
) -> Result<Vec<RemoteEntry>> {
    let mut session = Session::new()?;
    session.set_tcp_stream(connect(host)?);
    session.set_timeout(TIMEOUT.as_millis() as u32);
    session.handshake().context("Falha no handshake SSH")?;
    verify_host(&session, host, known_hosts)?;
    if let Some(identity) = identity {
        let passphrase = if identity.encrypted {
            Some(Zeroizing::new(match supplied_passphrase {
                Some(passphrase) => passphrase.to_owned(),
                None => vault::get_identity_passphrase(&identity.id)
                    .context("Esta chave exige passphrase. Salve-a explicitamente no Keychain pelo editor de chaves ou use o ssh-agent sem selecionar uma identidade gerenciada.")?,
            }))
        } else {
            None
        };
        session.userauth_pubkey_file(&host.username, None, Path::new(&identity.path), passphrase.as_ref().map(|value| value.as_str()))
            .context("Não foi possível autenticar com a chave. Confira a chave pública no servidor e a passphrase salva, ou use o ssh-agent.")?;
    } else if session.userauth_agent(&host.username).is_err() {
        let password = Zeroizing::new(
            vault::get_password(&host.id)
                .context("Sem chave no ssh-agent ou senha salva no Keychain para este host")?,
        );
        session
            .userauth_password(&host.username, &password)
            .context("Senha SSH recusada")?;
    }
    if !session.authenticated() {
        bail!("O servidor recusou a autenticação SSH");
    }
    let sftp = session
        .sftp()
        .context("O servidor não disponibilizou SFTP")?;
    match action {
        TransferAction::List { remote } => {
            let mut entries = Vec::new();
            for (path, stat) in sftp
                .readdir(Path::new(&remote))
                .context("Não foi possível listar o diretório remoto")?
            {
                let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if !safe_entry_name(name) {
                    continue;
                }
                entries.push(RemoteEntry {
                    name: name.into(),
                    path: remote_join(&remote, name),
                    is_dir: stat.is_dir(),
                    size: stat.size.unwrap_or(0),
                });
            }
            sort_entries(&mut entries);
            Ok(entries)
        }
        TransferAction::Download { remote, local } => {
            let mut input = sftp
                .open(Path::new(&remote))
                .context("Não foi possível abrir o arquivo remoto")?;
            download(&local, |output| {
                io::copy(&mut input, output)?;
                Ok(())
            })?;
            Ok(vec![])
        }
        TransferAction::Upload { local, remote } => {
            let mut input = File::open(local).context("Não foi possível abrir o arquivo local")?;
            let mut output = sftp.open_mode(Path::new(&remote), OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::EXCLUSIVE, 0o600, OpenType::File)
                .context("Não foi possível criar o destino remoto. Arquivos existentes não são substituídos via SFTP.")?;
            io::copy(&mut input, &mut output)
                .context("Falha ao enviar o arquivo; pode existir um arquivo remoto incompleto")?;
            output.fsync().ok(); // fsync extension is optional on SFTP servers.
            output.close()?;
            Ok(vec![])
        }
    }
}

fn verify_host(session: &Session, host: &Host, path: &Path) -> Result<()> {
    let (key, _) = session
        .host_key()
        .context("O servidor não enviou sua chave SSH")?;
    verify_known_key(session, host, path, key)
}

fn verify_known_key(session: &Session, host: &Host, path: &Path, key: &[u8]) -> Result<()> {
    let mut known = session.known_hosts()?;
    known.read_file(path, KnownHostFileKind::OpenSSH)
        .context("Abra primeiro uma sessão SSH e confira o fingerprint para registrar este servidor em ~/.ssh/known_hosts")?;
    match known.check_port(&host.address, host.port, key) {
        CheckResult::Match => Ok(()),
        CheckResult::NotFound => bail!("Servidor ainda não confiável para SFTP. Abra uma sessão SSH e confira o fingerprint antes de aceitar a chave."),
        CheckResult::Mismatch => bail!("A chave do servidor mudou. A transferência foi bloqueada; confirme a mudança com o administrador."),
        CheckResult::Failure => bail!("Não foi possível verificar a chave do servidor; transferência bloqueada."),
    }
}

fn ftp(host: &Host, action: TransferAction) -> Result<Vec<RemoteEntry>> {
    let password = Zeroizing::new(
        vault::get_password(&host.id)
            .context("Salve a senha FTP deste host no Keychain antes de conectar")?,
    );
    ftp_with_password(host, action, &password)
}

fn ftp_with_password(
    host: &Host,
    action: TransferAction,
    password: &str,
) -> Result<Vec<RemoteEntry>> {
    if password.chars().any(char::is_control) {
        bail!("Senhas FTP não podem conter caracteres de controle");
    }
    // The caller must obtain explicit plaintext-FTP consent in the UI.
    // All sockets, including passive data sockets, have finite IO timeouts.
    let mut ftp =
        FtpStream::connect_with_stream(connect(host)?)?.passive_stream_builder(|address| {
            let stream =
                TcpStream::connect_timeout(&address, TIMEOUT).map_err(FtpError::ConnectionError)?;
            stream
                .set_read_timeout(Some(TIMEOUT))
                .map_err(FtpError::ConnectionError)?;
            stream
                .set_write_timeout(Some(TIMEOUT))
                .map_err(FtpError::ConnectionError)?;
            Ok(stream)
        });
    ftp.set_passive_nat_workaround(true);
    ftp.login(host.username.as_str(), password)
        .context("Autenticação FTP recusada")?;
    ftp.transfer_type(FileType::Binary)?;
    let result = match action {
        TransferAction::List { remote } => ftp_entries(&mut ftp, &remote),
        TransferAction::Download { remote, local } => {
            let mut input = ftp.retr_as_stream(&remote)?;
            download(&local, |output| {
                io::copy(&mut input, output)?;
                ftp.finalize_retr_stream(input)?;
                Ok(())
            })?;
            Ok(vec![])
        }
        TransferAction::Upload { local, remote } => {
            // FTP has no portable exclusive STOR. Require a successful parent
            // listing, since a SIZE error does not prove a file is absent.
            // Another client can still race the listing/STOR pair.
            let (parent, name) = remote
                .rsplit_once('/')
                .map(|(parent, name)| (if parent.is_empty() { "/" } else { parent }, name))
                .unwrap_or((".", remote.as_str()));
            if !safe_entry_name(name) {
                bail!("Nome de arquivo remoto inválido");
            }
            if ftp_entries(&mut ftp, parent)?
                .iter()
                .any(|entry| entry.name == name)
            {
                bail!("Já existe um arquivo remoto com este nome");
            }
            let mut input = File::open(local).context("Não foi possível abrir o arquivo local")?;
            ftp.put_file(&remote, &mut input)
                .context("Falha no upload FTP; o arquivo remoto pode estar incompleto")?;
            Ok(vec![])
        }
    };
    let _ = ftp.quit();
    result
}

fn ftp_entries(ftp: &mut FtpStream, remote: &str) -> Result<Vec<RemoteEntry>> {
    let parsed = match ftp.mlsd(Some(remote)) {
        Ok(lines) => parse_ftp_listing(&lines, true)?,
        Err(_) => parse_ftp_listing(&ftp.list(Some(remote))?, false)?,
    };
    let mut entries: Vec<_> = parsed
        .into_iter()
        .filter(|file| safe_entry_name(file.name()))
        .map(|file| RemoteEntry {
            name: file.name().into(),
            path: remote_join(remote, file.name()),
            is_dir: file.is_directory(),
            size: file.size() as u64,
        })
        .collect();
    sort_entries(&mut entries);
    Ok(entries)
}

fn download(destination: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<()> {
    // Reserve an entirely new destination; never truncate an existing file.
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options
        .open(destination)
        .context("Destino já existe ou não pode ser criado. Escolha um novo nome.")?;
    let result = write(&mut output).and_then(|_| output.sync_all().map_err(Into::into));
    if result.is_err() {
        drop(output);
        let _ = fs::remove_file(destination);
    }
    result
}

fn safe_entry_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\'))
}

fn remote_join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.into()
    } else {
        format!("{}/{}", directory.trim_end_matches('/'), name)
    }
}

fn parse_ftp_listing(lines: &[String], mlsd: bool) -> Result<Vec<suppaftp::list::File>> {
    lines
        .iter()
        .filter(|line| !line.is_empty() && (mlsd || !line.starts_with("total ")))
        .map(|line| {
            let parsed = if mlsd {
                suppaftp::list::File::from_mlsx_line(line)
            } else {
                suppaftp::list::File::try_from(line.as_str())
            };
            parsed.context("O servidor retornou um formato de listagem FTP não suportado")
        })
        .collect()
}

fn sort_entries(entries: &mut [RemoteEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example_host_key() -> Vec<u8> {
        // Well-formed SSH public-key wire encoding; there is no private key.
        let mut key = vec![];
        key.extend(11_u32.to_be_bytes());
        key.extend(b"ssh-ed25519");
        key.extend(32_u32.to_be_bytes());
        key.extend([7_u8; 32]);
        key
    }

    #[test]
    fn sftp_known_hosts_rejects_unknown_changed_and_wrong_ports_and_accepts_hashes() {
        use base64::Engine;
        let temp = tempfile::tempdir().unwrap();
        let known = temp.path().join("known_hosts");
        let key = example_host_key();
        let host = Host {
            name: "Fixture".into(),
            address: "server.example.com".into(),
            port: 2222,
            ..Host::default()
        };
        let session = Session::new().unwrap();
        assert!(verify_known_key(&session, &host, &known, &key).is_err());
        fs::write(
            &known,
            format!(
                "[server.example.com]:2222 ssh-ed25519 {}\n",
                base64::engine::general_purpose::STANDARD.encode(&key)
            ),
        )
        .unwrap();
        verify_known_key(&session, &host, &known, &key).unwrap();
        let mut changed = key.clone();
        *changed.last_mut().unwrap() = 8;
        assert!(verify_known_key(&session, &host, &known, &changed)
            .unwrap_err()
            .to_string()
            .contains("mudou"));
        let wrong_port = Host {
            port: 22,
            ..host.clone()
        };
        assert!(verify_known_key(&session, &wrong_port, &known, &key).is_err());
        let unknown = Host {
            address: "unknown.example.com".into(),
            ..host.clone()
        };
        assert!(verify_known_key(&session, &unknown, &known, &key).is_err());
        let output = std::process::Command::new("ssh-keygen")
            .args(["-H", "-f"])
            .arg(&known)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(fs::read_to_string(&known).unwrap().starts_with("|1|"));
        verify_known_key(&session, &host, &known, &key).unwrap();
        assert!(verify_known_key(&session, &host, &known, &changed).is_err());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires /usr/sbin/sshd and a local account eligible for SSH; uses only temporary keys and known_hosts"]
    fn sftp_roundtrip_against_isolated_local_sshd() {
        use crate::storage::Storage;
        use std::io::Read;
        use std::net::TcpListener;
        use std::process::{Child, Command, Stdio};
        use std::time::Instant;
        struct Daemon(Child);
        impl Drop for Daemon {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
        let temp = tempfile::tempdir().unwrap();
        let storage = Storage::new(temp.path().join("data")).unwrap();
        let server = vault::generate_key(&storage, "ephemeral server").unwrap();
        let client = vault::generate_key(&storage, "ephemeral client").unwrap();
        let protected = vault::generate_key_with_passphrase(
            &storage,
            "ephemeral protected client",
            Some("ephemeral sftp fixture phrase"),
        )
        .unwrap();
        let authorized = temp.path().join("authorized_keys");
        fs::write(
            &authorized,
            format!("{}\n{}\n", client.public_key, protected.public_key),
        )
        .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let username = String::from_utf8(Command::new("id").arg("-un").output().unwrap().stdout)
            .unwrap()
            .trim()
            .to_owned();
        let config = temp.path().join("sshd_config");
        fs::write(&config, format!("Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nStrictModes no\nPubkeyAuthentication yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nUseDNS no\nAllowUsers {username}\nSubsystem sftp internal-sftp\nLogLevel ERROR\n", server.path, temp.path().join("sshd.pid").display(), authorized.display())).unwrap();
        let mut daemon = Daemon(
            Command::new("/usr/sbin/sshd")
                .args(["-D", "-e", "-f"])
                .arg(config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = daemon.0.try_wait().unwrap() {
                let mut message = String::new();
                daemon
                    .0
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut message)
                    .unwrap();
                panic!("Temporary sshd exited with {status}: {message}");
            }
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                break;
            }
            assert!(Instant::now() < deadline, "Temporary sshd did not start");
            std::thread::sleep(Duration::from_millis(20));
        }
        let host = Host {
            name: "isolated sftp fixture".into(),
            address: "127.0.0.1".into(),
            username,
            port,
            protocol: Protocol::Sftp,
            ..Host::default()
        };
        let known = temp.path().join("known_hosts");
        let list_action = || TransferAction::List {
            remote: temp.path().to_string_lossy().into_owned(),
        };
        fs::write(&known, "").unwrap();
        let unknown =
            sftp_with_known_hosts(&host, Some(&client), list_action(), &known).unwrap_err();
        assert!(unknown.to_string().contains("não confiável"));
        fs::write(
            &known,
            format!("[127.0.0.1]:{port} {}\n", client.public_key),
        )
        .unwrap();
        let changed =
            sftp_with_known_hosts(&host, Some(&client), list_action(), &known).unwrap_err();
        assert!(changed.to_string().contains("mudou"));
        fs::write(
            &known,
            format!("[127.0.0.1]:{port} {}\n", server.public_key),
        )
        .unwrap();
        let protected_entries = sftp_with_known_hosts_and_passphrase(
            &host,
            Some(&protected),
            list_action(),
            &known,
            Some("ephemeral sftp fixture phrase"),
        )
        .unwrap();
        assert!(!protected_entries.is_empty());
        let bad_passphrase = sftp_with_known_hosts_and_passphrase(
            &host,
            Some(&protected),
            list_action(),
            &known,
            Some("wrong fixture phrase"),
        )
        .unwrap_err();
        assert!(!format!("{bad_passphrase:#}").contains("fixture phrase"));
        let local = temp.path().join("upload.txt");
        fs::write(&local, b"SFTP fixture content").unwrap();
        let remote = temp
            .path()
            .join("remote.txt")
            .to_string_lossy()
            .into_owned();
        sftp_with_known_hosts(
            &host,
            Some(&client),
            TransferAction::Upload {
                local,
                remote: remote.clone(),
            },
            &known,
        )
        .unwrap();
        let entries = sftp_with_known_hosts(
            &host,
            Some(&client),
            TransferAction::List {
                remote: temp.path().to_string_lossy().into_owned(),
            },
            &known,
        )
        .unwrap();
        assert!(entries
            .iter()
            .any(|entry| entry.name == "remote.txt" && entry.size == 20));
        let downloaded = temp.path().join("downloaded.txt");
        sftp_with_known_hosts(
            &host,
            Some(&client),
            TransferAction::Download {
                remote: remote.clone(),
                local: downloaded.clone(),
            },
            &known,
        )
        .unwrap();
        assert_eq!(fs::read(downloaded).unwrap(), b"SFTP fixture content");
        let conflict = sftp_with_known_hosts(
            &host,
            Some(&client),
            TransferAction::Upload {
                local: PathBuf::from(&remote),
                remote,
            },
            &known,
        );
        assert!(
            conflict.is_err(),
            "SFTP must not overwrite an existing remote file"
        );
    }

    #[test]
    fn rejects_remote_entries_that_escape_the_selected_directory() {
        for name in [
            "..",
            ".",
            "../secret",
            "/etc/passwd",
            "dir\\file",
            "name\r\nDELE file",
        ] {
            assert!(!safe_entry_name(name), "{name:?}");
        }
        assert!(safe_entry_name("my document.txt"));
    }

    #[test]
    fn download_preserves_existing_files_and_cleans_up_failures() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("file");
        fs::write(&path, b"original").unwrap();
        assert!(download(&path, |_| Ok(())).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        let incomplete = temp.path().join("incomplete");
        assert!(download(&incomplete, |_| bail!("connection lost")).is_err());
        assert!(!incomplete.exists());
    }

    // A local protocol fixture exercises real FTP control/data sockets without
    // contacting a public service or touching the user's Keychain.
    fn ftp_fixture() -> (Host, std::thread::JoinHandle<Vec<u8>>) {
        use std::io::{BufRead, BufReader, Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let worker = std::thread::spawn(move || {
            let (mut control, _) = listener.accept().unwrap();
            control
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            writeln!(control, "220 fixture ready\r").unwrap();
            let mut reader = BufReader::new(control.try_clone().unwrap());
            let mut passive = None;
            let mut uploaded = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap() == 0 {
                    break;
                }
                let command = line.split_whitespace().next().unwrap_or("");
                match command {
                    "USER" => write!(control, "331 Password required\r\n").unwrap(),
                    "PASS" => write!(control, "230 Logged in\r\n").unwrap(),
                    "TYPE" => write!(control, "200 Type set\r\n").unwrap(),
                    "PASV" => {
                        let data = TcpListener::bind("127.0.0.1:0").unwrap();
                        let data_port = data.local_addr().unwrap().port();
                        write!(
                            control,
                            "227 Entering Passive Mode (127,0,0,1,{},{})\r\n",
                            data_port / 256,
                            data_port % 256
                        )
                        .unwrap();
                        passive = Some(data);
                    }
                    "MLSD" | "RETR" | "STOR" => {
                        write!(control, "150 Opening data connection\r\n").unwrap();
                        let (mut data, _) = passive.take().unwrap().accept().unwrap();
                        data.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                        match command {
                            "MLSD" => write!(data, "type=file;size=5;modify=20260101000000; hello.txt\r\ntype=dir;size=0;modify=20260101000000; folder\r\n").unwrap(),
                            "RETR" => data.write_all(b"hello").unwrap(),
                            _ => { data.read_to_end(&mut uploaded).unwrap(); }
                        }
                        drop(data);
                        write!(control, "226 Transfer complete\r\n").unwrap();
                    }
                    "SIZE" => write!(control, "550 File not found\r\n").unwrap(),
                    "QUIT" => {
                        write!(control, "221 Goodbye\r\n").unwrap();
                        break;
                    }
                    _ => panic!("Unexpected command: {command}"),
                }
            }
            uploaded
        });
        (
            Host {
                name: "Loopback fixture".into(),
                address: "127.0.0.1".into(),
                username: "fixture".into(),
                port,
                protocol: Protocol::Ftp,
                ..Host::default()
            },
            worker,
        )
    }

    #[test]
    fn ftp_lists_downloads_and_uploads_over_real_loopback_sockets() {
        let (host, worker) = ftp_fixture();
        let entries = ftp_with_password(
            &host,
            TransferAction::List { remote: "/".into() },
            "fixture-password",
        )
        .unwrap();
        worker.join().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "folder");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].path, "/hello.txt");
        assert_eq!(entries[1].size, 5);

        let temp = tempfile::tempdir().unwrap();
        let local = temp.path().join("download.txt");
        let (host, worker) = ftp_fixture();
        ftp_with_password(
            &host,
            TransferAction::Download {
                remote: "/hello.txt".into(),
                local: local.clone(),
            },
            "fixture-password",
        )
        .unwrap();
        worker.join().unwrap();
        assert_eq!(fs::read(&local).unwrap(), b"hello");

        let (host, worker) = ftp_fixture();
        ftp_with_password(
            &host,
            TransferAction::Upload {
                local,
                remote: "/upload.txt".into(),
            },
            "fixture-password",
        )
        .unwrap();
        assert_eq!(worker.join().unwrap(), b"hello");
    }

    #[test]
    fn ftp_upload_refuses_existing_names_when_size_cannot_prove_absence() {
        let temp = tempfile::tempdir().unwrap();
        let local = temp.path().join("upload.txt");
        fs::write(&local, b"do not overwrite").unwrap();
        let (host, worker) = ftp_fixture();
        let result = ftp_with_password(
            &host,
            TransferAction::Upload {
                local,
                remote: "/hello.txt".into(),
            },
            "fixture-password",
        );
        assert!(result.unwrap_err().to_string().contains("Já existe"));
        assert!(worker.join().unwrap().is_empty(), "STOR must not be sent");
    }
}
