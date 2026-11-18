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
