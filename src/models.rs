use serde::{Deserialize, Serialize};

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Protocol {
    #[default]
    Ssh,
    Sftp,
    Ftp,
}
impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ssh => "SSH",
            Self::Sftp => "SFTP",
            Self::Ftp => "FTP",
        }
    }
    pub fn default_port(self) -> u16 {
        if self == Self::Ftp {
            21
        } else {
            22
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub address: String,
    pub username: String,
    pub port: u16,
    pub protocol: Protocol,
    pub group: String,
    pub tags: Vec<String>,
    pub identity_id: Option<String>,
    pub favorite: bool,
    pub notes: String,
}
impl Default for Host {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: String::new(),
            address: String::new(),
            username: "root".into(),
            port: 22,
            protocol: Protocol::Ssh,
            group: "Pessoal".into(),
            tags: vec![],
            identity_id: None,
            favorite: false,
            notes: String::new(),
        }
    }
}
impl Host {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("Informe o nome do host.".into());
        }
        if self.address.is_empty()
            || self.address.starts_with('-')
            || self
                .address
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '/' | '\\' | '\0' | '@'))
        {
            return Err("Endereço inválido. Use um hostname ou IP sem usuário ou esquema.".into());
        }
        if self.username.is_empty()
            || self.username.starts_with('-')
            || self
                .username
                .chars()
                .any(|c| c.is_whitespace() || c.is_control() || c == '@')
        {
            return Err("Usuário inválido.".into());
        }
        if self.port == 0 {
            return Err("A porta deve estar entre 1 e 65535.".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub path: String,
    pub public_key: String,
    pub fingerprint: String,
    pub algorithm: String,
    #[serde(default)]
    pub encrypted: bool,
}
impl Identity {
    pub fn algorithm_label(&self) -> &str {
        match self.algorithm.as_str() {
            "ssh-ed25519" => "Ed25519",
            "ssh-rsa" | "rsa-sha2-256" | "rsa-sha2-512" => "RSA",
            "ecdsa-sha2-nistp256" | "ecdsa-sha2-nistp384" | "ecdsa-sha2-nistp521" => "ECDSA",
            "ssh-dss" => "DSA",
            "sk-ssh-ed25519@openssh.com" => "Ed25519 SK",
            "sk-ecdsa-sha2-nistp256@openssh.com" => "ECDSA SK",
            other => other,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum TunnelKind {
    #[default]
    Dynamic,
    Local,
    Remote,
}
impl TunnelKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dynamic => "SOCKS5",
            Self::Local => "Local",
            Self::Remote => "Remoto",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub kind: TunnelKind,
    pub bind_port: u16,
    pub destination_host: String,
    pub destination_port: u16,
}
impl Default for Tunnel {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: String::new(),
            host_id: String::new(),
            kind: TunnelKind::Dynamic,
            bind_port: 1080,
            destination_host: "127.0.0.1".into(),
            destination_port: 5432,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub title: String,
    pub command: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub schema_version: u32,
    pub hosts: Vec<Host>,
    pub identities: Vec<Identity>,
    pub tunnels: Vec<Tunnel>,
    pub snippets: Vec<Snippet>,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            schema_version: 1,
            hosts: vec![],
            identities: vec![],
            tunnels: vec![],
            snippets: vec![],
        }
    }
}
impl Workspace {
    /// Replace this identity's host selection, leaving unrelated links intact.
    /// Validate the complete selection first so errors never partially mutate it.
    pub fn bind_identity(&mut self, identity_id: &str, host_ids: &[String]) -> Result<(), String> {
        if !self
            .identities
            .iter()
            .any(|identity| identity.id == identity_id)
        {
            return Err("Identidade não encontrada.".into());
        }
        for id in host_ids {
            let host = self
                .hosts
                .iter()
                .find(|host| &host.id == id)
                .ok_or_else(|| "Host não encontrado.".to_owned())?;
            if host.protocol == Protocol::Ftp {
                return Err("Hosts FTP não usam chaves SSH.".into());
            }
        }
        for host in &mut self.hosts {
            if host_ids.contains(&host.id) {
                host.identity_id = Some(identity_id.to_owned());
            } else if host.identity_id.as_deref() == Some(identity_id) {
                host.identity_id = None;
            }
        }
        Ok(())
    }

    pub fn demo() -> Self {
        let mut workspace = Self::default();
        for (name, address, group, tags, favorite) in [
            (
                "Production API",
                "api.example.com",
                "Production",
                vec!["debian", "api"],
                true,
            ),
            (
                "Primary database",
                "db.example.com",
                "Production",
                vec!["ubuntu", "postgres"],
                true,
            ),
            (
                "Edge gateway",
                "edge.example.com",
                "Infrastructure",
                vec!["alpine", "gateway"],
                false,
            ),
            (
                "Staging server",
                "staging.example.com",
                "Development",
                vec!["ubuntu", "staging"],
                false,
            ),
            (
                "Build runner",
                "ci.example.com",
                "Development",
                vec!["debian", "ci"],
                false,
            ),
            (
                "Storage server",
                "storage.example.com",
                "Infrastructure",
                vec!["sftp", "backups"],
                false,
            ),
        ] {
            workspace.hosts.push(Host {
                name: name.into(),
                address: address.into(),
                username: "deploy".into(),
                group: group.into(),
                tags: tags.into_iter().map(String::from).collect(),
                favorite,
                ..Host::default()
            });
        }
        for (name, algorithm, encrypted, hosts) in [
            ("production-deploy", "ssh-ed25519", true, vec![0, 1]),
            ("infrastructure", "ssh-rsa", true, vec![2, 5]),
            ("development", "ssh-ed25519", false, vec![3, 4]),
        ] {
            let id = new_id();
            workspace.identities.push(Identity {
                id: id.clone(),
                name: name.into(),
                path: String::new(),
                public_key: "Demonstração — nenhuma chave privada ou pública real está cadastrada."
                    .into(),
                fingerprint: "SHA256:DEMO • apenas referência visual".into(),
                algorithm: algorithm.into(),
                encrypted,
            });
            for index in hosts {
                workspace.hosts[index].identity_id = Some(id.clone());
            }
        }
        workspace.tunnels.push(Tunnel {
            name: "Private network".into(),
            host_id: workspace.hosts[2].id.clone(),
            ..Tunnel::default()
        });
        workspace.snippets = vec![
            Snippet {
                id: new_id(),
                title: "System overview".into(),
                command: "uptime && df -h && free -m".into(),
                description: "Visão geral de um servidor Linux".into(),
            },
            Snippet {
                id: new_id(),
                title: "Follow service logs".into(),
                command: "journalctl -u nginx -f".into(),
                description: "Acompanhar logs do nginx".into(),
            },
        ];
        workspace
    }
}
