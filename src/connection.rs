//! Construction of SSH argv. Host data is never interpolated into a shell.

use crate::models::{Host, Identity, Protocol, Tunnel, TunnelKind};
use anyhow::{bail, ensure, Result};

pub fn ssh_executable() -> &'static str {
    if cfg!(target_os = "macos") {
        "/usr/bin/ssh"
    } else {
        "ssh"
    }
}

fn base_arguments(host: &Host, identity: Option<&Identity>) -> Result<Vec<String>> {
    host.validate().map_err(anyhow::Error::msg)?;
    ensure!(
        host.protocol != Protocol::Ftp,
        "Um host FTP não oferece uma sessão SSH."
    );
    // Config isolation prevents Host/Match rules, ProxyCommand and RemoteCommand
    // in a user's SSH config from unexpectedly changing an app-managed connection.
    // The normal known_hosts file and SSH_AUTH_SOCK are deliberately retained.
    let mut args: Vec<String> = [
        "-F",
        "/dev/null",
        "-o",
        "StrictHostKeyChecking=ask",
        "-o",
        "ForwardAgent=no",
        "-o",
        "ClearAllForwardings=yes",
        "-o",
        "ConnectTimeout=15",
        "-o",
        "ServerAliveInterval=30",
        "-o",
        "ServerAliveCountMax=3",
        "-o",
        "PermitLocalCommand=no",
        "-p",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    args.push(host.port.to_string());
    args.extend(["-l".into(), host.username.clone()]);
    if let Some(identity) = identity {
        ensure!(
            !identity.path.is_empty() && !identity.path.contains('\0'),
            "Caminho de identidade inválido."
        );
        args.extend([
            "-i".into(),
            identity.path.clone(),
            "-o".into(),
            "IdentitiesOnly=yes".into(),
        ]);
    }
    Ok(args)
}

pub fn ssh_arguments(host: &Host, identity: Option<&Identity>) -> Result<Vec<String>> {
    let mut args = base_arguments(host, identity)?;
    args.extend(["-tt".into(), "--".into(), host.address.clone()]);
    Ok(args)
}

pub fn tunnel_arguments(
    host: &Host,
    identity: Option<&Identity>,
    tunnel: &Tunnel,
) -> Result<Vec<String>> {
    ensure!(
        tunnel.host_id == host.id,
        "O túnel não pertence ao host selecionado."
    );
    ensure!(
        tunnel.bind_port > 0,
        "A porta de escuta deve ser maior que zero."
    );
    let mut args = base_arguments(host, identity)?;
    // ClearAllForwardings would also erase these explicitly supplied forwards.
    // With -F /dev/null there are no inherited forwarding rules to clear.
    if let Some(index) = args.iter().position(|arg| arg == "ClearAllForwardings=yes") {
        args.drain(index - 1..=index);
    }
    args.extend(["-N".into(), "-o".into(), "ExitOnForwardFailure=yes".into()]);
    let binding = format!("127.0.0.1:{}", tunnel.bind_port);
    match tunnel.kind {
        TunnelKind::Dynamic => args.extend(["-D".into(), binding]),
        TunnelKind::Local | TunnelKind::Remote => {
            let destination = tunnel.destination_host.trim();
            if destination.is_empty()
                || destination.starts_with('-')
                || !destination
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))
            {
                bail!("Destino do túnel inválido. Use um hostname ou IP.");
            }
            ensure!(
                tunnel.destination_port > 0,
                "A porta de destino deve ser maior que zero."
            );
            let destination = if destination.contains(':') && !destination.starts_with('[') {
                format!("[{destination}]")
            } else {
                destination.to_owned()
            };
            let flag = if tunnel.kind == TunnelKind::Local {
                "-L"
            } else {
                "-R"
            };
            args.extend([
                flag.into(),
                format!("{binding}:{destination}:{}", tunnel.destination_port),
            ]);
        }
    }
    args.extend(["--".into(), host.address.clone()]);
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> Host {
        Host {
            name: "test".into(),
            address: "server.example.com".into(),
            username: "deploy".into(),
            ..Host::default()
        }
    }

    #[test]
    fn ssh_uses_explicit_trust_and_keeps_target_a_single_argument() {
        let args = ssh_arguments(&host(), None).unwrap();
        assert!(args.contains(&"StrictHostKeyChecking=ask".into()));
        assert!(args.contains(&"ForwardAgent=no".into()));
        assert_eq!(&args[args.len() - 2..], &["--", "server.example.com"]);
        assert!(!args.iter().any(|arg| arg == "StrictHostKeyChecking=no"));
    }

    #[test]
    fn host_cannot_inject_ssh_options_or_shell_commands() {
        for value in [
            "-oProxyCommand=evil",
            "server\n-oProxyCommand=evil",
            "user@server",
            "ssh://server",
        ] {
            let mut target = host();
            target.address = value.into();
            assert!(ssh_arguments(&target, None).is_err());
        }
        // Shell metacharacters remain one argv entry; no shell ever receives it.
        let mut target = host();
        target.address = "server;touch-owned".into();
        assert_eq!(
            ssh_arguments(&target, None).unwrap().last().unwrap(),
            "server;touch-owned"
        );
    }

    #[test]
    fn forwards_are_loopback_bound_and_fail_closed() {
        let target = host();
        let mut tunnel = Tunnel {
            host_id: target.id.clone(),
            ..Tunnel::default()
        };
        let args = tunnel_arguments(&target, None, &tunnel).unwrap();
        assert!(args.contains(&"127.0.0.1:1080".into()));
        assert!(args.contains(&"ExitOnForwardFailure=yes".into()));
        assert!(!args.contains(&"ClearAllForwardings=yes".into()));
        tunnel.kind = TunnelKind::Local;
        tunnel.destination_host = "::1".into();
        let args = tunnel_arguments(&target, None, &tunnel).unwrap();
        assert!(args.contains(&"127.0.0.1:1080:[::1]:5432".into()));
        tunnel.destination_host = "server:22 -oProxyCommand=evil".into();
        assert!(tunnel_arguments(&target, None, &tunnel).is_err());
        tunnel.kind = TunnelKind::Dynamic;
        tunnel.bind_port = 0;
        assert!(tunnel_arguments(&target, None, &tunnel).is_err());
    }
}
