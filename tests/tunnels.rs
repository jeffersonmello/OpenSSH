//! Opt-in routing checks against an entirely temporary, loopback-only SSH daemon.
#![cfg(target_os = "macos")]

use anyhow::{bail, ensure, Context, Result};
use openssh_manager::{
    connection,
    models::{Host, Identity, Tunnel, TunnelKind},
    storage::Storage,
    vault,
};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const PAYLOAD: &[u8] = b"isolated routing fixture";

struct Process(Child);

impl Process {
    fn check_running(&mut self) -> Result<()> {
        if let Some(status) = self.0.try_wait()? {
            let mut stderr = String::new();
            if let Some(mut stream) = self.0.stderr.take() {
                stream.read_to_string(&mut stderr)?;
            }
            bail!("Fixture process exited with {status}: {stderr}");
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if self.0.try_wait()?.is_none() {
            self.0.kill()?;
        }
        self.0.wait()?;
        Ok(())
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

struct Fixture {
    // Drop the daemon before the temporary keys/configuration directory.
    daemon: Process,
    host: Host,
    client: Identity,
    known_hosts: PathBuf,
    _temp: tempfile::TempDir,
}

impl Fixture {
    fn start() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let storage = Storage::new(temp.path().join("data"))?;
        let server = vault::generate_key(&storage, "ephemeral tunnel server")?;
        let client = vault::generate_key(&storage, "ephemeral tunnel client")?;
        let authorized = temp.path().join("authorized_keys");
        fs::write(&authorized, &client.public_key)?;
        let port = unused_port()?;
        let username = String::from_utf8(Command::new("/usr/bin/id").arg("-un").output()?.stdout)?
            .trim()
            .to_owned();
        let config = temp.path().join("sshd_config");
        fs::write(&config, format!(
            "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nPidFile {}\nAuthorizedKeysFile {}\nStrictModes no\nPubkeyAuthentication yes\nPasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\nUseDNS no\nAllowUsers {username}\nAllowTcpForwarding yes\nGatewayPorts no\nLogLevel ERROR\n",
            server.path, temp.path().join("sshd.pid").display(), authorized.display()
        ))?;
        let mut daemon = Process(
            Command::new("/usr/sbin/sshd")
                .args(["-D", "-e", "-f"])
                .arg(config)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()?,
        );
        let readiness = connect_when_ready(port, &mut daemon)?;
        drop(readiness);
        let known_hosts = temp.path().join("known_hosts");
        fs::write(
            &known_hosts,
            format!("[127.0.0.1]:{port} {}\n", server.public_key),
        )?;
        let host = Host {
            name: "isolated tunnel fixture".into(),
            address: "127.0.0.1".into(),
            username,
            port,
            ..Host::default()
        };
        Ok(Self {
            daemon,
            host,
            client,
            known_hosts,
            _temp: temp,
        })
    }

    fn tunnel(&self, kind: TunnelKind, bind_port: u16, target_port: u16) -> Result<Process> {
        let tunnel = Tunnel {
            name: "ephemeral routing test".into(),
            host_id: self.host.id.clone(),
            kind,
            bind_port,
            destination_host: "127.0.0.1".into(),
            destination_port: target_port,
            ..Tunnel::default()
        };
        // Exercise production argument construction, changing only test isolation
        // options before the destination. No user SSH configuration, identity
        // agent, personal trust file or system trust file participates.
        let mut args = connection::tunnel_arguments(&self.host, Some(&self.client), &tunnel)?;
        let destination = args
            .iter()
            .position(|arg| arg == "--")
            .context("Missing destination boundary")?;
        args.splice(
            destination..destination,
            [
                "-o".into(),
                format!("UserKnownHostsFile={}", self.known_hosts.display()),
                "-o".into(),
                "GlobalKnownHostsFile=/dev/null".into(),
                "-o".into(),
                "IdentityAgent=none".into(),
                "-o".into(),
                "BatchMode=yes".into(),
                "-o".into(),
                "LogLevel=ERROR".into(),
            ],
        );
        Ok(Process(
            Command::new(connection::ssh_executable())
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .spawn()?,
        ))
    }
}

struct Echo {
    port: u16,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Echo {
    fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(true)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                        let mut payload = vec![0; PAYLOAD.len()];
                        if stream.read_exact(&mut payload).is_ok() {
                            let _ = stream.write_all(&payload);
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            port,
            stop,
            worker: Some(worker),
        })
    }
}

impl Drop for Echo {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn unused_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn connect_when_ready(port: u16, process: &mut Process) -> Result<TcpStream> {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        process.check_running()?;
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            stream.set_write_timeout(Some(Duration::from_secs(3)))?;
            return Ok(stream);
        }
        ensure!(
            Instant::now() < deadline,
            "Loopback listener did not start on {port}"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

fn echo_roundtrip(stream: &mut TcpStream) -> Result<()> {
    stream.write_all(PAYLOAD)?;
    let mut received = vec![0; PAYLOAD.len()];
    stream.read_exact(&mut received)?;
    ensure!(received == PAYLOAD, "Forwarded payload changed");
    Ok(())
}

fn assert_listener_closed(port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_err() {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "Listener remained open after tunnel process stopped"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "requires /usr/sbin/sshd and an eligible local macOS account; entirely isolated loopback fixture"]
fn socks5_local_and_remote_forwarding_deliver_payloads_and_close_listeners() -> Result<()> {
    let mut fixture = Fixture::start()?;
    let target = Echo::start()?;
    for kind in [TunnelKind::Dynamic, TunnelKind::Local, TunnelKind::Remote] {
        let port = unused_port()?;
        let mut tunnel = fixture.tunnel(kind, port, target.port)?;
        {
            let mut stream = connect_when_ready(port, &mut tunnel)?;
            if kind == TunnelKind::Dynamic {
                stream.write_all(&[5, 1, 0])?;
                let mut greeting = [0; 2];
                stream.read_exact(&mut greeting)?;
                ensure!(greeting == [5, 0], "SOCKS5 negotiation failed");
                let [high, low] = target.port.to_be_bytes();
                stream.write_all(&[5, 1, 0, 1, 127, 0, 0, 1, high, low])?;
                let mut reply = [0; 4];
                stream.read_exact(&mut reply)?;
                ensure!(
                    reply[0] == 5 && reply[1] == 0,
                    "SOCKS5 connection refused: {reply:?}"
                );
                let length = match reply[3] {
                    1 => 6,
                    4 => 18,
                    3 => {
                        let mut length = [0];
                        stream.read_exact(&mut length)?;
                        usize::from(length[0]) + 2
                    }
                    _ => bail!("Unknown SOCKS5 reply address type"),
                };
                stream.read_exact(&mut vec![0; length])?;
            }
            echo_roundtrip(&mut stream)
                .with_context(|| format!("{} forwarding failed", kind.label()))?;
        }
        tunnel.stop()?;
        assert_listener_closed(port)?;
    }
    fixture.daemon.stop()?;
    Ok(())
}
