use super::*;

pub(super) enum PassphraseAction {
    Keep,
    Save(zeroize::Zeroizing<String>),
    Clear,
}

#[cfg(test)]
mod tests;

pub(super) struct KeyDraft {
    id: Option<String>,
    name: String,
    private_key: String,
    passphrase: String,
    source: Option<PathBuf>,
    generate: bool,
    replace_material: bool,
    reveal_key: bool,
    reveal_passphrase: bool,
    remember_passphrase: bool,
    linked_hosts: Vec<String>,
}

impl Drop for KeyDraft {
    fn drop(&mut self) {
        self.private_key.zeroize();
        self.passphrase.zeroize();
    }
}

impl KeyDraft {
    fn new(generate: bool) -> Self {
        Self {
            id: None,
            name: String::new(),
            private_key: String::new(),
            passphrase: String::new(),
            source: None,
            generate,
            replace_material: true,
            reveal_key: false,
            reveal_passphrase: false,
            remember_passphrase: false,
            linked_hosts: vec![],
        }
    }

    pub(super) fn existing(identity: &Identity, workspace: &Workspace) -> Self {
        let mut draft = Self::new(false);
        draft.id = Some(identity.id.clone());
        draft.name = identity.name.clone();
        draft.replace_material = false;
        draft.linked_hosts = workspace
            .hosts
            .iter()
            .filter(|host| host.identity_id.as_ref() == Some(&identity.id))
            .map(|host| host.id.clone())
            .collect();
        draft
    }

    fn has_material(&self) -> bool {
        self.generate || self.source.is_some() || !self.private_key.trim().is_empty()
    }

    fn worker_copy(&self) -> Self {
        Self {
            id: self.id.clone(),
            name: self.name.clone(),
            private_key: self.private_key.clone(),
            passphrase: self.passphrase.clone(),
            source: self.source.clone(),
            generate: self.generate,
            replace_material: self.replace_material,
            reveal_key: false,
            reveal_passphrase: false,
            remember_passphrase: self.remember_passphrase,
            linked_hosts: self.linked_hosts.clone(),
        }
    }
}

/// Password masking disables clipboard export. egui also records raw text in
/// its undo state, which must be cleared even when a secret is revealed.
pub(super) fn secret_input(
    ui: &mut egui::Ui,
    id: &str,
    text: &mut String,
    multiline: bool,
    reveal: bool,
    hint: &str,
) -> egui::Response {
    let edit = if multiline {
        egui::TextEdit::multiline(text)
            .desired_rows(4)
            .font(FontId::monospace(11.5))
    } else {
        egui::TextEdit::singleline(text)
    };
    let mut output = edit
        .id_salt(id)
        .password(!reveal)
        .char_limit(if multiline { 1_048_576 } else { 4096 })
        .hint_text(hint)
        .desired_width(f32::INFINITY)
        .margin(Vec2::new(12.0, 10.0))
        .show(ui);
    output.state.clear_undoer();
    output.state.store(ui.ctx(), output.response.id);
    output.response
}

impl OpenSshApp {
    pub(super) fn keys(&mut self, ui: &mut egui::Ui) {
        if self.host_editor.is_some() && ui.button("Voltar ao cadastro da conexão").clicked() {
            self.page = Page::Hosts;
        }
        self.header(
            ui,
            "Suas chaves. Suas conexões.",
            "Organize identidades e reutilize a mesma chave em vários servidores.",
        );
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!self.busy, |ui| {
                if primary(ui, "+  Nova chave").clicked() {
                    self.key_draft = Some(KeyDraft::new(false));
                }
                if ui.button("Gerar Ed25519").clicked() {
                    self.key_draft = Some(KeyDraft::new(true));
                }
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                pill(
                    ui,
                    &format!("{} identidades", self.workspace.identities.len()),
                    MUTED,
                );
            });
        });
        ui.add_space(20.0);
        ui.horizontal(|ui| {
            draw_icon(ui, Icon::Search, MUTED, 18.0);
            ui.add_sized(
                [ui.available_width(), 38.0],
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Buscar chave por nome, tipo ou fingerprint")
                    .margin(Vec2::new(12.0, 10.0)),
            );
        });
        ui.add_space(26.0);
        section_label(ui, "IDENTIDADES", "COFRE LOCAL");
        ui.add_space(12.0);
        let query = self.query.to_lowercase();
        let identities: Vec<_> = self
            .workspace
            .identities
            .iter()
            .filter(|identity| {
                format!(
                    "{} {} {}",
                    identity.name,
                    identity.algorithm_label(),
                    identity.fingerprint
                )
                .to_lowercase()
                .contains(&query)
            })
            .cloned()
            .collect();
        if identities.is_empty() {
            empty_state(
                ui,
                Icon::Key,
                "Uma chave abre muitos caminhos.",
                if self.workspace.identities.is_empty() {
                    "Cadastre uma chave privada ou gere uma identidade Ed25519. Depois, escolha as conexões que vão utilizá-la."
                } else {
                    "Nenhuma identidade corresponde à sua busca."
                },
            );
        }
        let columns = if ui.available_width() >= 780.0 {
            3
        } else if ui.available_width() >= 510.0 {
            2
        } else {
            1
        };
        egui::ScrollArea::vertical().id_salt("key-grid").show(ui, |ui| {
            for row in identities.chunks(columns) {
                ui.columns(columns, |uis| {
                    for (index, identity) in row.iter().enumerate() {
                        let selected = self.key_draft.as_ref().and_then(|draft| draft.id.as_ref()) == Some(&identity.id);
                        let count = self.workspace.hosts.iter().filter(|host| host.identity_id.as_ref() == Some(&identity.id)).count();
                        let frame = egui::Frame::new().fill(if selected { Color32::from_rgb(26, 47, 47) } else { CARD }).stroke(Stroke::new(1.0_f32, if selected { MINT.gamma_multiply(0.8) } else { LINE })).corner_radius(18).inner_margin(16.0).show(&mut uis[index], |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                icon_tile(ui, Icon::Key, MINT, 38.0);
                                ui.vertical(|ui| {
                                    ui.add(egui::Label::new(RichText::new(&identity.name).size(13.0).strong()).truncate());
                                    ui.label(RichText::new(format!("{} · {count} conexões", identity.algorithm_label())).size(10.5).color(MUTED));
                                });
                            });
                            ui.add_space(14.0);
                            pill(ui, if identity.encrypted { "COM PASSPHRASE" } else { "SEM PASSPHRASE" }, if identity.encrypted { MINT } else { MUTED });
                        }).response.interact(egui::Sense::click());
                        if !self.busy && frame.clicked() { self.key_draft = Some(KeyDraft::existing(identity, &self.workspace)); }
                    }
                });
                ui.add_space(14.0);
            }
            ui.add_space(18.0);
            egui::Frame::new().fill(Color32::from_rgb(18, 31, 37)).corner_radius(18).inner_margin(18.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Shield, MINT, 20.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Identidades compartilhadas, segredos protegidos.").size(13.0).strong());
                        ui.label(RichText::new("Chaves privadas em arquivos 0600. Conteúdo oculto por padrão.").size(11.0).color(MUTED));
                    });
                });
            });
        });
    }

    pub(super) fn key_panel(&mut self, ctx: &egui::Context) {
        if self.key_draft.is_none() {
            self.key_draft = self
                .workspace
                .identities
                .first()
                .map(|identity| KeyDraft::existing(identity, &self.workspace));
        }
        let width = if ctx.screen_rect().width() < 1200.0 {
            340.0
        } else {
            380.0
        };
        egui::SidePanel::right("key-editor").exact_width(width).resizable(false).frame(egui::Frame::new().fill(SIDE).inner_margin(22.0)).show(ctx, |ui| {
            let Some(mut draft) = self.key_draft.take() else {
                ui.add_space(30.0);
                icon_tile(ui, Icon::Key, MINT, 52.0);
                ui.add_space(18.0);
                ui.label(RichText::new("Seu cofre de identidades").size(21.0).strong());
                ui.label(RichText::new("Crie uma chave ou selecione uma identidade para editar os vínculos.").color(MUTED));
                return;
            };
            let identity = draft.id.as_ref().and_then(|id| self.workspace.identities.iter().find(|identity| &identity.id == id)).cloned();
            let mut save = false;
            let mut cancel = false;
            let mut remove = false;
            let mut forget_passphrase = false;
            ui.add_space(6.0);
            ui.label(RichText::new(if identity.is_some() { "EDITAR IDENTIDADE" } else if draft.generate { "GERAR IDENTIDADE" } else { "NOVA IDENTIDADE" }).size(10.0).color(MUTED).extra_letter_spacing(1.4));
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                icon_tile(ui, Icon::Key, MINT, 40.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(identity.as_ref().map(|identity| identity.name.as_str()).unwrap_or("Nova chave SSH")).size(19.0).strong());
                    ui.label(RichText::new(identity.as_ref().map(|identity| identity.algorithm_label()).unwrap_or(if draft.generate { "Ed25519" } else { "Importar identidade" })).size(11.0).color(MUTED));
                });
            });
            ui.add_space(16.0);
            let scroll_height = (ui.available_height() - 122.0).max(180.0);
            egui::ScrollArea::vertical().id_salt("key-form").max_height(scroll_height).show(ui, |ui| {
                ui.add_enabled_ui(!self.busy, |ui| {
                    field(ui, "Nome da identidade", &mut draft.name);
                    ui.add_space(4.0);
                    if identity.is_some() && !draft.replace_material {
                        egui::Frame::new().fill(CARD).corner_radius(14).inner_margin(12.0).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                draw_icon(ui, Icon::Shield, MINT, 17.0);
                                ui.label(RichText::new("Chave privada armazenada").size(12.0));
                            });
                            ui.label(RichText::new("O conteúdo salvo permanece oculto.").size(10.5).color(MUTED));
                            if ui.small_button("Substituir chave privada...").clicked() { draft.replace_material = true; }
                        });
                    } else if !draft.generate {
                        if identity.is_some() { ui.label(RichText::new("Os hosts vinculados passarão a usar a nova chave. O arquivo anterior será preservado.").size(10.5).color(AMBER)); }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if identity.is_some() { "Substituir chave privada (opcional)" } else { "Chave privada" }).size(12.0).color(MUTED));
                        });
                        if let Some(source) = &draft.source {
                            let name = source.file_name().unwrap_or_default().to_string_lossy().into_owned();
                            let mut clear = false;
                            egui::Frame::new().fill(CARD).corner_radius(12).inner_margin(10.0).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    draw_icon(ui, Icon::Folder, MINT, 16.0);
                                    ui.add(egui::Label::new(name).truncate());
                                    if ui.small_button("×").clicked() { clear = true; }
                                });
                            });
                            if clear { draft.source = None; }
                        } else {
                            secret_input(ui, "identity-private-key", &mut draft.private_key, true, draft.reveal_key, if identity.is_some() { "Chave atual preservada. Cole outra apenas para substituir." } else { "Cole sua chave privada aqui ou importe um arquivo." });
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut draft.reveal_key, "Mostrar conteúdo");
                                if ui.button("Importar arquivo...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().set_title("Selecionar chave privada SSH").pick_file() {
                                        draft.private_key.zeroize();
                                        draft.source = Some(path);
                                    }
                                }
                            });
                        }
                    } else {
                        pill(ui, "ED25519 · GERAÇÃO LOCAL", MINT);
                    }
                    let protected = identity.as_ref().is_some_and(|identity| identity.encrypted);
                    if draft.has_material() || identity.is_none() || protected {
                        let mut passphrase_fields = |ui: &mut egui::Ui| {
                            ui.label(RichText::new(if draft.generate { "Passphrase da nova chave (opcional)" } else { "Passphrase da chave (se protegida)" }).size(12.0).color(MUTED));
                            secret_input(ui, "identity-passphrase", &mut draft.passphrase, false, draft.reveal_passphrase, "Passphrase");
                            ui.checkbox(&mut draft.reveal_passphrase, "Mostrar passphrase");
                            ui.add_enabled_ui(!draft.passphrase.is_empty(), |ui| { ui.checkbox(&mut draft.remember_passphrase, "Guardar no Chaves para SFTP"); });
                            ui.label(RichText::new("O terminal SSH solicita a passphrase na própria sessão.").size(10.5).color(MUTED));
                            if protected && ui.add_enabled(!self.demo, egui::Button::new("Esquecer passphrase no Chaves")).clicked() { forget_passphrase = true; }
                        };
                        if identity.is_some() && !draft.replace_material {
                            egui::CollapsingHeader::new("Passphrase e autenticação SFTP").id_salt(("identity-passphrase-section", draft.id.as_deref())).show(ui, &mut passphrase_fields);
                        } else { passphrase_fields(ui); }
                    }
                    if protected { pill(ui, "CHAVE PROTEGIDA POR PASSPHRASE", MINT); }
                    if let Some(identity) = &identity {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Chave pública").size(12.0).color(MUTED));
                            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| { if ui.small_button("Copiar").clicked() { ui.ctx().copy_text(identity.public_key.clone()); } });
                        });
                        let mut public_key = identity.public_key.clone();
                        ui.add(egui::TextEdit::multiline(&mut public_key).interactive(false).font(FontId::monospace(10.5)).desired_rows(3).desired_width(f32::INFINITY).margin(Vec2::new(10.0, 10.0)));
                        ui.label(RichText::new(&identity.fingerprint).monospace().size(10.0).color(MUTED));
                        if draft.has_material() { ui.label(RichText::new("A chave pública será recalculada ao salvar a substituição.").size(10.5).color(AMBER)); }
                    } else {
                        ui.label(RichText::new("Chave pública e fingerprint são calculados ao salvar.").size(10.5).color(MUTED));
                    }
                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        draw_icon(ui, Icon::Server, MINT, 17.0);
                        ui.label(RichText::new("Conexões vinculadas").size(13.0).strong());
                        pill(ui, &draft.linked_hosts.len().to_string(), MINT);
                    });
                    ui.label(RichText::new("Selecione os hosts que usarão esta identidade.").size(11.0).color(MUTED));
                    let mut hosts = self.workspace.hosts.iter().filter(|host| host.protocol != Protocol::Ftp).collect::<Vec<_>>();
                    hosts.sort_by(|a, b| {
                        let linked = |host: &Host| draft.id.as_ref().is_some_and(|id| host.identity_id.as_ref() == Some(id));
                        (!linked(a), &a.name).cmp(&(!linked(b), &b.name))
                    });
                    if hosts.is_empty() { ui.label(RichText::new("Cadastre uma conexão SSH ou SFTP para criar vínculos.").size(11.0).color(MUTED)); }
                    for host in hosts {
                        let mut checked = draft.linked_hosts.contains(&host.id);
                        if ui.push_id(&host.id, |ui| ui.checkbox(&mut checked, &host.name)).inner.changed() {
                            if checked { draft.linked_hosts.push(host.id.clone()); } else { draft.linked_hosts.retain(|id| id != &host.id); }
                        }
                        if let Some(other) = host.identity_id.as_ref().filter(|id| Some(id.as_str()) != draft.id.as_deref()) {
                            let name = self.workspace.identities.iter().find(|key| &key.id == other).map(|key| key.name.as_str()).unwrap_or("outra identidade");
                            ui.label(RichText::new(if checked { format!("Ao salvar, substituirá: {name}") } else { format!("Atual: {name}") }).size(10.0).color(if checked { AMBER } else { MUTED }));
                        }
                    }
                });
            });
            ui.add_space(12.0);
            ui.add_enabled_ui(!self.busy && (self.writable || self.demo), |ui| {
                if primary_full(ui, if draft.generate { "Gerar e salvar chave" } else { "Salvar identidade" }).clicked() { save = true; }
                ui.horizontal(|ui| {
                    if ui.button("Cancelar").clicked() { cancel = true; }
                    if identity.is_some() && ui.button(RichText::new("Remover referência").color(RED)).clicked() { remove = true; }
                });
            });
            if self.busy { ui.label(RichText::new("Validando a identidade...").size(11.0).color(MINT)); }
            if remove { self.confirm_delete = draft.id.as_ref().map(|id| ("identity".into(), id.clone())); }
            if forget_passphrase {
                if let Some(id) = &draft.id {
                    let id = id.clone(); let sender = self.sender.clone(); let context = ctx.clone();
                    std::thread::spawn(move || { let result = vault::delete_identity_passphrase(&id).map_err(|_| "Não foi possível esquecer a passphrase no Chaves.".into()); let _ = sender.send(AppEvent::Credential(result, "Passphrase removida do Chaves do macOS.")); context.request_repaint(); });
                }
            }
            if save {
                self.save_key_draft(draft);
            } else if !cancel {
                self.key_draft = Some(draft);
            }
        });
    }

    fn save_key_draft(&mut self, mut draft: KeyDraft) {
        if draft.name.trim().is_empty() || draft.name.chars().any(char::is_control) {
            self.notice = Some(("Informe um nome válido para a identidade.".into(), true));
            self.key_draft = Some(draft);
            return;
        }
        let current = draft
            .id
            .as_ref()
            .and_then(|id| {
                self.workspace
                    .identities
                    .iter()
                    .find(|identity| &identity.id == id)
            })
            .cloned();
        let update_passphrase = !draft.passphrase.is_empty()
            && current.as_ref().is_some_and(|identity| identity.encrypted)
            && !draft.has_material();
        if update_passphrase && !draft.remember_passphrase {
            self.notice = Some(("Marque Guardar no Chaves para atualizar a passphrase de SFTP. Para editar apenas nome e vínculos, deixe o campo vazio.".into(), true));
            self.key_draft = Some(draft);
            return;
        }
        if !draft.has_material() && !update_passphrase {
            if let Some(mut identity) = current {
                identity.name = draft.name.trim().into();
                if !self.commit_identity(identity, &draft.linked_hosts, None) {
                    self.key_draft = Some(draft);
                }
            } else {
                self.notice = Some((
                    "Cole uma chave privada, importe um arquivo ou escolha Gerar Ed25519.".into(),
                    true,
                ));
                self.key_draft = Some(draft);
            }
            return;
        }
        if self.demo {
            self.notice = Some(("Demonstração: importação e geração de chaves privadas desabilitadas. Os vínculos das identidades de exemplo podem ser editados em memória.".into(), false));
            draft.private_key.zeroize();
            draft.passphrase.zeroize();
            self.key_draft = Some(draft);
            return;
        }
        let Some(storage) = self.storage.clone().filter(|_| self.writable) else {
            self.notice = Some((
                "Armazenamento indisponível para salvar a identidade.".into(),
                true,
            ));
            self.key_draft = Some(draft);
            return;
        };
        self.busy = true;
        let sender = self.sender.clone();
        let request = draft.worker_copy();
        self.key_draft = Some(draft);
        std::thread::spawn(move || {
            let draft = request;
            let passphrase = if draft.passphrase.is_empty() {
                None
            } else {
                Some(draft.passphrase.as_str())
            };
            let result = if update_passphrase {
                let mut identity = current.expect("validated existing identity");
                identity.name = draft.name.trim().into();
                vault::validate_identity_passphrase(&identity, &draft.passphrase).map(|()| identity)
            } else if draft.generate {
                vault::generate_key_with_passphrase(&storage, &draft.name, passphrase)
            } else if let Some(source) = &draft.source {
                vault::import_key_with_passphrase(&storage, &draft.name, source, passphrase)
            } else {
                vault::import_key_text(&storage, &draft.name, &draft.private_key, passphrase)
            };
            let passphrase_action = if result.as_ref().is_ok_and(|identity| identity.encrypted)
                && draft.remember_passphrase
                && !draft.passphrase.is_empty()
            {
                PassphraseAction::Save(zeroize::Zeroizing::new(draft.passphrase.clone()))
            } else if result.is_ok() && draft.id.is_some() && draft.has_material() {
                PassphraseAction::Clear
            } else {
                PassphraseAction::Keep
            };
            let result =
                result.map_err(|error| format!("Não foi possível salvar a identidade: {error}"));
            let _ = sender.send(AppEvent::Identity {
                result,
                previous_id: draft.id.clone(),
                linked_hosts: draft.linked_hosts.clone(),
                passphrase_action,
            });
        });
    }

    pub(super) fn apply_passphrase_action(&mut self, id: String, action: PassphraseAction) {
        if self.demo || matches!(action, PassphraseAction::Keep) {
            return;
        }
        self.busy = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = match action {
                PassphraseAction::Save(passphrase) => vault::set_identity_passphrase(&id, &passphrase)
                    .map_err(|_| "A identidade foi salva, mas a passphrase não pôde ser guardada no Chaves. Informe-a novamente no editor para usar SFTP.".into()),
                PassphraseAction::Clear => vault::delete_identity_passphrase(&id)
                    .map_err(|_| "A identidade foi salva, mas uma passphrase antiga não pôde ser removida do Chaves. Use Esquecer passphrase no editor.".into()),
                PassphraseAction::Keep => Ok(()),
            };
            let _ = sender.send(AppEvent::IdentityCredential(result));
        });
    }

    pub(super) fn commit_identity(
        &mut self,
        identity: Identity,
        hosts: &[String],
        warning: Option<String>,
    ) -> bool {
        let previous = self.workspace.clone();
        if let Some(saved) = self
            .workspace
            .identities
            .iter_mut()
            .find(|saved| saved.id == identity.id)
        {
            *saved = identity.clone();
        } else {
            self.workspace.identities.push(identity.clone());
        }
        if let Err(error) = self.workspace.bind_identity(&identity.id, hosts) {
            self.workspace = previous;
            self.notice = Some((error, true));
            return false;
        }
        if self.persist() {
            self.entries.clear();
            self.key_draft = Some(KeyDraft::existing(&identity, &self.workspace));
            self.notice = Some(match warning {
                Some(warning) => (warning, true),
                None => (
                    format!("Identidade salva · {} conexões vinculadas.", hosts.len()),
                    false,
                ),
            });
            true
        } else {
            self.workspace = previous;
            false
        }
    }
}
