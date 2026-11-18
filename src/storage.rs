//! Versioned metadata storage. Secrets never belong in workspace.json.
use crate::models::Workspace;
use anyhow::{bail, Context, Result};
use directories::ProjectDirs;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Storage {
    pub data_dir: PathBuf,
    pub keys_dir: PathBuf,
}

impl Storage {
    pub fn discover() -> Result<Self> {
        let project = ProjectDirs::from("io", "github.jeffersonmello", "OpenSSH Manager")
            .context("Não foi possível localizar o diretório de dados do usuário")?;
        Self::new(project.data_local_dir().to_path_buf())
    }

    pub fn new(data_dir: PathBuf) -> Result<Self> {
        secure_directory(&data_dir)?;
        let keys_dir = data_dir.join("keys");
        secure_directory(&keys_dir)?;
        Ok(Self { data_dir, keys_dir })
    }

    pub fn workspace_path(&self) -> PathBuf {
        self.data_dir.join("workspace.json")
    }

    pub fn load(&self) -> Result<Workspace> {
        let path = self.workspace_path();
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Workspace::default()),
            Err(e) => return Err(e.into()),
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                bail!("O arquivo de workspace deve ser um arquivo regular, sem link simbólico")
            }
            Ok(_) => {}
        }
        let bytes = fs::read(&path).context("Não foi possível ler o workspace")?;
        let workspace: Workspace = serde_json::from_slice(&bytes)
            .context("Workspace inválido; o arquivo original foi preservado")?;
        validate_version(&workspace)?;
        Ok(workspace)
    }

    pub fn save(&self, workspace: &Workspace) -> Result<()> {
        validate_version(workspace)?;
        // An invalid or newer on-disk workspace must never be overwritten after
        // a caller has fallen back to an empty workspace in memory.
        self.load()?;
        let bytes = serde_json::to_vec_pretty(workspace)?;
        let temporary = self
            .data_dir
            .join(format!(".workspace-{}.tmp", crate::models::new_id()));
        let result = (|| -> Result<()> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&temporary)
                .context("Não foi possível criar o arquivo temporário")?;
            file.write_all(&bytes)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            fs::rename(&temporary, self.workspace_path())
                .context("Falha ao gravar o workspace de forma atômica")?;
            // Persist the directory entry as well as the file contents.
            #[cfg(unix)]
            File::open(&self.data_dir)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn validate_version(workspace: &Workspace) -> Result<()> {
    if workspace.schema_version != 1 {
        bail!(
            "Versão de workspace não suportada: {}. Esperado: 1. O arquivo foi preservado.",
            workspace.schema_version
        );
    }
    Ok(())
}

pub(crate) fn secure_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).context("Não foi possível criar o diretório de dados")?;
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("O diretório de dados deve ser um diretório real, sem link simbólico");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
