use eframe::egui::{self, Align, Color32, FontId, RichText, Stroke, Vec2};
use openssh_manager::{
    models::*,
    storage::Storage,
    terminal::TerminalSession,
    transfer::{self, RemoteEntry, TransferAction},
    vault,
};
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
use zeroize::Zeroize;

const BG: Color32 = Color32::from_rgb(14, 20, 28);
const SIDE: Color32 = Color32::from_rgb(18, 26, 36);
const CARD: Color32 = Color32::from_rgb(23, 33, 45);
const LINE: Color32 = Color32::from_rgb(39, 53, 68);
const TEXT: Color32 = Color32::from_rgb(228, 236, 242);
const MUTED: Color32 = Color32::from_rgb(133, 151, 168);
const MINT: Color32 = Color32::from_rgb(100, 228, 189);
const AMBER: Color32 = Color32::from_rgb(239, 188, 112);
const RED: Color32 = Color32::from_rgb(244, 136, 136);

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Hosts,
    Keys,
    Tunnels,
    Files,
    Snippets,
    KnownHosts,
    Settings,
    Terminal,
}
#[derive(Clone, Copy)]
enum Icon {
    Server,
    Key,
    Route,
    Folder,
    Code,
    Shield,
    Settings,
    Terminal,
    Search,
    Star,
}
enum AppEvent {
    Identity(Result<Identity, String>),
    Transfer {
        host: Host,
        path: String,
        listing: bool,
        result: Result<Vec<RemoteEntry>, String>,
    },
    Credential(Result<(), String>),
}
struct SessionTab {
    id: String,
    title: String,
    tunnel_id: Option<String>,
    session: TerminalSession,
}

pub struct OpenSshApp {
    workspace: Workspace,
    storage: Option<Storage>,
    writable: bool,
    demo: bool,
    page: Page,
    query: String,
    group: Option<String>,
    favorites: bool,
    selected: Option<String>,
    host_editor: Option<Host>,
    host_password: String,
    host_tags: String,
    host_editor_id: String,
    tunnel_editor: Option<Tunnel>,
    key_name: String,
    snippet_editor: Option<Snippet>,
    notice: Option<(String, bool)>,
    sender: Sender<AppEvent>,
    receiver: Receiver<AppEvent>,
    busy: bool,
    sessions: Vec<SessionTab>,
    active_session: usize,
    file_host: String,
    remote_path: String,
    local_path: String,
    entries: Vec<RemoteEntry>,
    ftp_consent: bool,
    confirm_delete: Option<(String, String)>,
    logo: Option<egui::TextureHandle>,
    screenshot: Option<PathBuf>,
    started: Instant,
    screenshot_requested: bool,
}

impl OpenSshApp {
    pub fn new(cc: &eframe::CreationContext<'_>, demo: bool, screenshot: Option<PathBuf>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut style = (*ctx.style()).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = SIDE;
        style.visuals.extreme_bg_color = BG;
        style.visuals.faint_bg_color = CARD;
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.selection.bg_fill = Color32::from_rgb(37, 80, 72);
        style.visuals.selection.stroke = Stroke::new(1.0_f32, MINT);
        style.visuals.widgets.inactive.bg_fill = CARD;
        style.visuals.widgets.inactive.weak_bg_fill = CARD;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, LINE);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(34, 51, 62);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(34, 51, 62);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, MINT.gamma_multiply(0.6));
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(38, 68, 65);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(38, 68, 65);
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(14.0, 9.0);
        style.spacing.interact_size.y = 34.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, FontId::proportional(28.0));
        ctx.set_style(style);
        let logo = image::load_from_memory(include_bytes!("../assets/brand/logo.png"))
            .ok()
            .map(|image| {
                let rgba = image.to_rgba8();
                ctx.load_texture(
                    "brand-logo",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    ),
                    Default::default(),
                )
            });
        let (sender, receiver) = mpsc::channel();
        let (storage, workspace, writable, notice) = if demo {
            (None, Workspace::demo(), true, None)
        } else {
            match Storage::discover() {
                Ok(storage) => match storage.load() {
                    Ok(workspace) => (Some(storage), workspace, true, None),
                    Err(error) => (Some(storage), Workspace::default(), false, Some((format!("Não foi possível ler seus dados. Gravação bloqueada para preservar o arquivo: {error}"), true))),
                },
                Err(error) => (None, Workspace::default(), false, Some((format!("Diretório de dados indisponível: {error}"), true))),
            }
        };
        let selected = workspace.hosts.first().map(|host| host.id.clone());
        let file_host = selected.clone().unwrap_or_default();
        Self {
            workspace,
            storage,
            writable,
            demo,
            page: Page::Hosts,
            query: String::new(),
            group: None,
            favorites: false,
            selected,
            host_editor: None,
            host_password: String::new(),
            host_tags: String::new(),
            host_editor_id: String::new(),
            tunnel_editor: None,
            key_name: "Minha chave".into(),
            snippet_editor: None,
            notice,
            sender,
            receiver,
            busy: false,
            sessions: vec![],
            active_session: 0,
            file_host,
            remote_path: "/".into(),
            local_path: std::env::var("HOME").unwrap_or_else(|_| ".".into()),
            entries: vec![],
            ftp_consent: false,
            confirm_delete: None,
            logo,
            screenshot,
            started: Instant::now(),
            screenshot_requested: false,
        }
    }

    fn persist(&mut self) -> bool {
        if self.demo {
            return true;
        }
        if !self.writable {
            self.notice = Some((
                "Gravação bloqueada. Corrija o arquivo de dados e reinicie o app.".into(),
                true,
            ));
            return false;
        }
        if let Some(storage) = &self.storage {
            if let Err(error) = storage.save(&self.workspace) {
                self.notice = Some((format!("Não foi possível salvar: {error}"), true));
                return false;
            }
            return true;
        }
        false
    }

    fn poll(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                AppEvent::Identity(result) => {
                    self.busy = false;
                    match result {
                        Ok(identity) => {
                            self.workspace.identities.push(identity);
                            if self.persist() {
                                self.notice =
                                    Some(("Chave adicionada ao cofre local.".into(), false));
                            }
                        }
                        Err(error) => self.notice = Some((error, true)),
                    }
                }
                AppEvent::Transfer {
                    host,
                    path,
                    listing,
                    result,
                } => {
                    self.busy = false;
                    match result {
                        Ok(entries) => {
                            if listing
                                && self.file_host == host.id
                                && self.remote_path == path
                                && self.workspace.hosts.iter().any(|current| current == &host)
                            {
                                self.entries = entries;
                            }
                            self.notice = Some((
                                if listing {
                                    "Diretório atualizado."
                                } else {
                                    "Transferência concluída."
                                }
                                .into(),
                                false,
                            ));
                        }
                        Err(error) => self.notice = Some((error, true)),
                    }
                }
                AppEvent::Credential(result) => {
                    self.notice = Some(match result {
                        Ok(()) => ("Senha salva no Chaves do macOS.".into(), false),
                        Err(error) => (error, true),
                    })
                }
            }
        }
    }

    fn sidebar(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("navigation")
            .exact_width(220.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDE).inner_margin(18.0))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if let Some(logo) = &self.logo {
                        ui.add(
                            egui::Image::new(logo)
                                .fit_to_exact_size(Vec2::splat(36.0))
                                .corner_radius(9),
                        );
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new("OpenSSH").size(21.0).strong());
                        ui.label(
                            RichText::new("CONNECTION MANAGER")
                                .size(8.5)
                                .color(MUTED)
                                .extra_letter_spacing(1.6),
                        );
                    });
                });
                ui.add_space(30.0);
                ui.label(
                    RichText::new("WORKSPACE")
                        .size(10.0)
                        .color(MUTED)
                        .extra_letter_spacing(1.8),
                );
                ui.add_space(8.0);
                for (page, icon, label) in [
                    (Page::Hosts, Icon::Server, "Conexões"),
                    (Page::Keys, Icon::Key, "Chaves SSH"),
                    (Page::Tunnels, Icon::Route, "Túneis e SOCKS"),
                    (Page::Files, Icon::Folder, "Arquivos"),
                    (Page::Snippets, Icon::Code, "Snippets"),
                ] {
                    if nav_item(ui, icon, label, self.page == page) {
                        self.page = page;
                        self.query.clear();
                    }
                }
                ui.add_space(24.0);
                ui.label(
                    RichText::new("SEGURANÇA")
                        .size(10.0)
                        .color(MUTED)
                        .extra_letter_spacing(1.8),
                );
                ui.add_space(8.0);
                if nav_item(
                    ui,
                    Icon::Shield,
                    "Servidores conhecidos",
                    self.page == Page::KnownHosts,
                ) {
                    self.page = Page::KnownHosts;
                }
                if nav_item(
                    ui,
                    Icon::Settings,
                    "Preferências",
                    self.page == Page::Settings,
                ) {
                    self.page = Page::Settings;
                }
                let gap = (ui.available_height() - 142.0).max(12.0);
                ui.add_space(gap);
                egui::Frame::new()
                    .fill(CARD)
                    .corner_radius(10)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            draw_icon(ui, Icon::Shield, MINT, 16.0);
                            ui.label(RichText::new("Local por natureza").size(12.0).strong());
                        });
                        ui.label(
                            RichText::new("Seus servidores. Suas chaves.\nSem conta, sem nuvem.")
                                .size(11.5)
                                .color(MUTED),
                        );
                    });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(MINT).size(9.0));
                    ui.label(RichText::new("v0.1.0 · alpha").size(11.0).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("MIT").size(10.0).color(MUTED));
                    });
                });
            });
    }

    fn topbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("topbar")
            .exact_height(58.0)
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(RichText::new("Workspace pessoal").size(13.0).color(MUTED));
                    ui.label(RichText::new("/").color(LINE));
                    ui.label(RichText::new(self.page_title()).size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        pill(
                            ui,
                            if self.demo {
                                "DEMONSTRAÇÃO"
                            } else {
                                "ARMAZENAMENTO LOCAL"
                            },
                            if self.demo { AMBER } else { MINT },
                        );
                    });
                });
            });
        if !self.sessions.is_empty() {
            egui::TopBottomPanel::top("sessions")
                .exact_height(48.0)
                .frame(
                    egui::Frame::new()
                        .fill(SIDE)
                        .inner_margin(egui::Margin::symmetric(20, 6)),
                )
                .show(ctx, |ui| {
                    let mut close = None;
                    ui.horizontal(|ui| {
                        for (index, tab) in self.sessions.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    self.page == Page::Terminal && self.active_session == index,
                                    format!(">_  {}", tab.title),
                                )
                                .clicked()
                            {
                                self.active_session = index;
                                self.page = Page::Terminal;
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("Encerrar sessão e processo")
                                .clicked()
                            {
                                close = Some(index);
                            }
                        }
                    });
                    if let Some(index) = close {
                        self.sessions.remove(index);
                        self.active_session = self
                            .active_session
                            .min(self.sessions.len().saturating_sub(1));
                        if self.sessions.is_empty() {
                            self.page = Page::Hosts;
                        }
                    }
                });
        }
    }

    fn page_title(&self) -> &'static str {
        match self.page {
            Page::Hosts => "Conexões",
            Page::Keys => "Chaves SSH",
            Page::Tunnels => "Túneis e SOCKS",
            Page::Files => "Arquivos",
            Page::Snippets => "Snippets",
            Page::KnownHosts => "Servidores conhecidos",
            Page::Settings => "Preferências",
            Page::Terminal => "Terminal",
        }
    }

    fn header(&self, ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.label(RichText::new(title).size(30.0).strong());
        ui.add_space(2.0);
        ui.label(RichText::new(subtitle).size(13.0).color(MUTED));
        ui.add_space(22.0);
    }

    fn hosts(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                self.header(
                    ui,
                    "Sua infraestrutura, conectada.",
                    "Um lugar para seus servidores, terminais e conexões seguras.",
                )
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                if primary(ui, "+  Nova conexão").clicked() {
                    self.host_editor = Some(Host::default());
                }
            });
        });
        ui.horizontal(|ui| {
            draw_icon(ui, Icon::Search, MUTED, 18.0);
            ui.add_sized(
                [(ui.available_width() - 310.0).max(120.0), 38.0],
                egui::TextEdit::singleline(&mut self.query)
                    .hint_text("Buscar por nome, endereço ou tag...")
                    .margin(Vec2::new(12.0, 10.0)),
            );
            if ui
                .selectable_label(self.favorites, "☆  Favoritos")
                .clicked()
            {
                self.favorites = !self.favorites;
            }
            if ui.button("Todos os grupos").clicked() {
                self.group = None;
            }
        });
        ui.add_space(26.0);
        let mut groups: Vec<String> = self
            .workspace
            .hosts
            .iter()
            .map(|host| host.group.clone())
            .collect();
        groups.sort();
        groups.dedup();
        if !groups.is_empty() {
            section_label(ui, "GRUPOS", &format!("{} workspaces", groups.len()));
            ui.add_space(10.0);
            let columns = groups.len().min(4);
            ui.columns(columns, |uis| {
                for (index, group) in groups.iter().enumerate() {
                    let count = self
                        .workspace
                        .hosts
                        .iter()
                        .filter(|host| &host.group == group)
                        .count();
                    let active = self.group.as_ref() == Some(group);
                    let response = egui::Frame::new()
                        .fill(if active {
                            Color32::from_rgb(28, 52, 49)
                        } else {
                            CARD
                        })
                        .stroke(Stroke::new(
                            1.0_f32,
                            if active {
                                MINT.gamma_multiply(0.6)
                            } else {
                                LINE
                            },
                        ))
                        .corner_radius(12)
                        .inner_margin(16.0)
                        .show(&mut uis[index % columns], |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                icon_tile(ui, Icon::Folder, MINT, 36.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(group).strong().size(13.0));
                                    ui.label(
                                        RichText::new(format!("{count} conexões"))
                                            .size(11.0)
                                            .color(MUTED),
                                    );
                                });
                            });
                        })
                        .response;
                    if response.interact(egui::Sense::click()).clicked() {
                        self.group = if active { None } else { Some(group.clone()) };
                    }
                }
            });
            ui.add_space(28.0);
        }
        let query = self.query.to_lowercase();
        let hosts: Vec<Host> = self
            .workspace
            .hosts
            .iter()
            .filter(|host| {
                (!self.favorites || host.favorite)
                    && self.group.as_ref().is_none_or(|group| &host.group == group)
                    && format!(
                        "{} {} {} {}",
                        host.name,
                        host.address,
                        host.group,
                        host.tags.join(" ")
                    )
                    .to_lowercase()
                    .contains(&query)
            })
            .cloned()
            .collect();
        section_label(ui, "CONEXÕES", &format!("{} servidores", hosts.len()));
        ui.add_space(12.0);
        if hosts.is_empty() {
            empty_state(
                ui,
                Icon::Server,
                "Seu próximo servidor começa aqui",
                "Adicione uma conexão SSH, SFTP ou FTP para organizar seu workspace.",
            );
            ui.horizontal(|ui| {
                if primary(ui, "+  Adicionar conexão").clicked() {
                    self.host_editor = Some(Host::default());
                }
                if !self.demo
                    && self.workspace.hosts.is_empty()
                    && self.sessions.is_empty()
                    && !self.busy
                    && ui.button("Explorar demonstração").clicked()
                {
                    self.demo = true;
                    self.workspace = Workspace::demo();
                    self.selected = self.workspace.hosts.first().map(|host| host.id.clone());
                }
            });
            return;
        }
        let cols = if ui.available_width() > 820.0 { 3 } else { 2 };
        egui::ScrollArea::vertical()
            .id_salt("host-grid")
            .show(ui, |ui| {
                for row in hosts.chunks(cols) {
                    ui.columns(cols, |uis| {
                        for (index, host) in row.iter().enumerate() {
                            let selected = self.selected.as_ref() == Some(&host.id);
                            let response = egui::Frame::new()
                                .fill(CARD)
                                .stroke(Stroke::new(
                                    1.0_f32,
                                    if selected {
                                        MINT.gamma_multiply(0.8)
                                    } else {
                                        LINE
                                    },
                                ))
                                .corner_radius(12)
                                .inner_margin(17.0)
                                .show(&mut uis[index], |ui| {
                                    ui.set_min_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        icon_tile(
                                            ui,
                                            if host.protocol == Protocol::Ftp {
                                                Icon::Folder
                                            } else {
                                                Icon::Server
                                            },
                                            if host.group == "Production" {
                                                MINT
                                            } else {
                                                Color32::from_rgb(146, 171, 239)
                                            },
                                            38.0,
                                        );
                                        ui.vertical(|ui| {
                                            ui.label(RichText::new(&host.name).strong().size(14.0));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} · {}",
                                                    host.protocol.label(),
                                                    host.username
                                                ))
                                                .size(11.0)
                                                .color(MUTED),
                                            );
                                        });
                                        ui.with_layout(
                                            egui::Layout::right_to_left(Align::Center),
                                            |ui| {
                                                if host.favorite {
                                                    draw_icon(ui, Icon::Star, AMBER, 14.0);
                                                }
                                            },
                                        );
                                    });
                                    ui.add_space(13.0);
                                    ui.label(
                                        RichText::new(format!("{}:{}", host.address, host.port))
                                            .monospace()
                                            .size(11.0)
                                            .color(MUTED),
                                    );
                                    ui.add_space(9.0);
                                    ui.horizontal(|ui| {
                                        for tag in host.tags.iter().take(3) {
                                            pill(ui, tag, MUTED);
                                        }
                                    });
                                })
                                .response;
                            let response = response.interact(egui::Sense::click());
                            if response.clicked() {
                                self.selected = Some(host.id.clone());
                            }
                            if response.double_clicked() {
                                self.connect(host.clone());
                            }
                            response.context_menu(|ui| {
                                if ui.button("Conectar").clicked() {
                                    self.connect(host.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Editar conexão").clicked() {
                                    self.host_editor = Some(host.clone());
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                    ui.add_space(14.0);
                }
                ui.add_space(22.0);
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 31, 37))
                    .stroke(Stroke::new(1.0_f32, LINE))
                    .corner_radius(12)
                    .inner_margin(18.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            draw_icon(ui, Icon::Terminal, MINT, 20.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Feito para quem vive no terminal.")
                                        .size(13.0)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(
                                        "Rust nativo · SSH do sistema · Dados sob seu controle",
                                    )
                                    .size(12.0)
                                    .color(MUTED),
                                );
                            });
                        });
                    });
            });
    }

    fn details(&mut self, ctx: &egui::Context) {
        let Some(host) = self
            .selected
            .as_ref()
            .and_then(|id| self.workspace.hosts.iter().find(|host| &host.id == id))
            .cloned()
        else {
            return;
        };
        egui::SidePanel::right("host-details")
            .exact_width(290.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(SIDE).inner_margin(22.0))
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("DETALHES DA CONEXÃO")
                            .size(10.0)
                            .color(MUTED)
                            .extra_letter_spacing(1.2),
                    );
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("×").clicked() {
                            self.selected = None;
                        }
                    });
                });
                ui.add_space(28.0);
                icon_tile(ui, Icon::Server, MINT, 52.0);
                ui.add_space(12.0);
                ui.label(RichText::new(&host.name).size(22.0).strong());
                ui.label(RichText::new(&host.group).size(12.0).color(MUTED));
                ui.add_space(22.0);
                detail(ui, "ENDEREÇO", &host.address);
                detail(ui, "USUÁRIO", &host.username);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| detail(ui, "PROTOCOLO", host.protocol.label()));
                    ui.add_space(40.0);
                    ui.vertical(|ui| detail(ui, "PORTA", &host.port.to_string()));
                });
                let identity = host.identity_id.as_ref().and_then(|id| {
                    self.workspace
                        .identities
                        .iter()
                        .find(|identity| &identity.id == id)
                });
                detail(
                    ui,
                    "AUTENTICAÇÃO",
                    identity
                        .map(|identity| identity.name.as_str())
                        .unwrap_or("SSH Agent / interativa"),
                );
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    for tag in &host.tags {
                        pill(ui, tag, MUTED);
                    }
                });
                if !host.notes.is_empty() {
                    ui.add_space(20.0);
                    detail(ui, "NOTAS", &host.notes);
                }
                ui.add_space(26.0);
                let label = if host.protocol == Protocol::Ssh {
                    ">_  Abrir terminal"
                } else {
                    "Abrir arquivos  →"
                };
                if primary_full(ui, label).clicked() {
                    self.connect(host.clone());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 36.0],
                        egui::Button::new("Editar conexão"),
                    )
                    .clicked()
                {
                    self.host_editor = Some(host.clone());
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 34.0],
                        egui::Button::new(if host.favorite {
                            "★  Remover dos favoritos"
                        } else {
                            "☆  Adicionar aos favoritos"
                        }),
                    )
                    .clicked()
                {
                    if let Some(saved) = self
                        .workspace
                        .hosts
                        .iter_mut()
                        .find(|item| item.id == host.id)
                    {
                        saved.favorite = !saved.favorite;
                    }
                    self.persist();
                }
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    draw_icon(ui, Icon::Shield, MUTED, 14.0);
                    ui.label(
                        RichText::new("Verificação de host habilitada")
                            .size(10.5)
                            .color(MUTED),
                    );
                });
            });
    }

    fn connect(&mut self, host: Host) {
        if self.demo {
            self.notice = Some(("Demonstração: conexões de rede desabilitadas. Reinicie sem --demo para usar seus servidores.".into(), false));
            return;
        }
        if host.protocol != Protocol::Ssh {
            self.file_host = host.id;
            self.entries.clear();
            self.remote_path = "/".into();
            self.ftp_consent = false;
            self.page = Page::Files;
            return;
        }
        let identity = host.identity_id.as_ref().and_then(|id| {
            self.workspace
                .identities
                .iter()
                .find(|identity| &identity.id == id)
        });
        match TerminalSession::ssh(&host, identity) {
            Ok(session) => {
                self.sessions.push(SessionTab {
                    id: new_id(),
                    title: host.name.clone(),
                    tunnel_id: None,
                    session,
                });
                self.active_session = self.sessions.len() - 1;
                self.page = Page::Terminal;
            }
            Err(error) => self.notice = Some((format!("Falha ao iniciar SSH: {error}"), true)),
        }
    }

    fn keys(&mut self, ui: &mut egui::Ui) {
        self.header(
            ui,
            "As chaves da sua infraestrutura.",
            "Identidades locais para autenticar suas conexões SSH e SFTP.",
        );
        ui.horizontal(|ui| {
            ui.add_sized(
                [240.0, 36.0],
                egui::TextEdit::singleline(&mut self.key_name).hint_text("Nome da identidade"),
            );
            if ui
                .add_enabled(
                    !self.busy && !self.demo && self.writable,
                    egui::Button::new("+  Gerar Ed25519")
                        .fill(MINT)
                        .stroke(Stroke::NONE),
                )
                .clicked()
            {
                self.key_job(None);
            }
            if ui
                .add_enabled(
                    !self.busy && !self.demo && self.writable,
                    egui::Button::new("Importar chave..."),
                )
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Escolher chave privada SSH")
                    .pick_file()
                {
                    self.key_job(Some(path));
                }
            }
        });
        ui.add_space(16.0);
        ui.label(RichText::new("As chaves privadas ficam em arquivos locais com permissão 0600. As chaves geradas nesta versão não têm passphrase.").size(12.0).color(AMBER));
        ui.add_space(24.0);
        if self.workspace.identities.is_empty() {
            empty_state(ui, Icon::Key, "Uma identidade. Muitas conexões.", "Gere uma chave Ed25519 ou importe uma chave privada existente. Nunca compartilhe sua chave privada.");
        }
        for identity in self.workspace.identities.clone() {
            egui::Frame::new()
                .fill(CARD)
                .stroke(Stroke::new(1.0_f32, LINE))
                .corner_radius(12)
                .inner_margin(18.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        icon_tile(ui, Icon::Key, MINT, 40.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&identity.name).strong());
                            ui.label(RichText::new(&identity.algorithm).color(MUTED).size(11.0));
                        });
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Remover referência").clicked() {
                                self.confirm_delete =
                                    Some(("identity".into(), identity.id.clone()));
                            }
                            if ui.button("Copiar chave pública").clicked() {
                                ui.ctx().copy_text(identity.public_key.clone());
                                self.notice = Some(("Chave pública copiada.".into(), false));
                            }
                        });
                    });
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new(&identity.fingerprint)
                            .monospace()
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.label(RichText::new(&identity.path).size(11.0).color(MUTED));
                });
            ui.add_space(12.0);
        }
    }

    fn key_job(&mut self, source: Option<PathBuf>) {
        let Some(storage) = self.storage.as_ref() else {
            return;
        };
        let keys_dir = storage.keys_dir.clone();
        let data_dir = storage.data_dir.clone();
        let name = self.key_name.trim().to_string();
        if name.is_empty() {
            self.notice = Some(("Informe um nome para a identidade.".into(), true));
            return;
        }
        self.busy = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let storage = Storage { data_dir, keys_dir };
            let result = match source {
                Some(source) => vault::import_key(&storage, &name, &source),
                None => vault::generate_key(&storage, &name),
            };
            let _ =
                sender.send(AppEvent::Identity(result.map_err(|error| {
                    format!("Não foi possível adicionar a chave: {error}")
                })));
        });
    }

    fn tunnels(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                self.header(
                    ui,
                    "Um caminho seguro até lá.",
                    "SOCKS5 dinâmico e encaminhamento local ou remoto sobre SSH.",
                )
            });
            ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| {
                if primary(ui, "+  Novo túnel").clicked() {
                    self.tunnel_editor = Some(Tunnel {
                        host_id: self
                            .workspace
                            .hosts
                            .first()
                            .map(|host| host.id.clone())
                            .unwrap_or_default(),
                        ..Tunnel::default()
                    });
                }
            });
        });
        if self.workspace.tunnels.is_empty() {
            empty_state(ui, Icon::Route, "Acesse sua rede privada com segurança", "Use um servidor SSH como ponte para serviços internos ou navegue por um proxy SOCKS5.");
        }
        for tunnel in self.workspace.tunnels.clone() {
            let host = self
                .workspace
                .hosts
                .iter()
                .find(|host| host.id == tunnel.host_id)
                .cloned();
            let running = self.sessions.iter_mut().position(|session| {
                session.tunnel_id.as_ref() == Some(&tunnel.id) && !session.session.is_finished()
            });
            egui::Frame::new().fill(CARD).stroke(Stroke::new(1.0_f32, LINE)).corner_radius(12).inner_margin(20.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    icon_tile(ui, Icon::Route, MINT, 42.0);
                    ui.vertical(|ui| { ui.label(RichText::new(&tunnel.name).strong()); ui.label(RichText::new(format!("{} · 127.0.0.1:{} · {}", tunnel.kind.label(), tunnel.bind_port, host.as_ref().map(|host| host.name.as_str()).unwrap_or("Host removido"))).size(12.0).color(MUTED)); });
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Excluir").clicked() { self.confirm_delete = Some(("tunnel".into(), tunnel.id.clone())); }
                        if ui.button("Editar").clicked() { self.tunnel_editor = Some(tunnel.clone()); }
                        if let Some(index) = running {
                            if ui.button("Abrir sessão").clicked() { self.active_session = index; self.page = Page::Terminal; }
                            if ui.button("Parar").clicked() { self.sessions.remove(index); }
                        } else if primary(ui, "Iniciar túnel").clicked() {
                            if self.demo { self.notice = Some(("Demonstração: túneis de rede desabilitados.".into(), false)); }
                            else if let Some(host) = &host {
                                let identity = host.identity_id.as_ref().and_then(|id| self.workspace.identities.iter().find(|key| &key.id == id));
                                match TerminalSession::tunnel(host, identity, &tunnel) {
                                    Ok(session) => { self.sessions.push(SessionTab { id: new_id(), title: format!("Túnel: {}", tunnel.name), tunnel_id: Some(tunnel.id.clone()), session }); self.active_session = self.sessions.len() - 1; self.page = Page::Terminal; self.notice = Some(("Confirme a autenticação no terminal. Processo aberto não confirma que a porta já está disponível.".into(), false)); }
                                    Err(error) => self.notice = Some((error.to_string(), true)),
                                }
                            }
                        }
                    });
                });
                if tunnel.kind == TunnelKind::Dynamic { ui.add_space(10.0); ui.label(RichText::new(format!("Cliente: socks5h://127.0.0.1:{} · DNS pelo túnel", tunnel.bind_port)).monospace().size(11.0).color(MUTED)); }
            });
            ui.add_space(14.0);
        }
    }

    fn files(&mut self, ui: &mut egui::Ui) {
        self.header(
            ui,
            "Arquivos, de um lado ao outro.",
            "Explore diretórios e transfira arquivos por SFTP ou FTP.",
        );
        let before = self.file_host.clone();
        ui.horizontal(|ui| {
            ui.label("Conexão");
            egui::ComboBox::from_id_salt("file-host")
                .width(280.0)
                .selected_text(
                    self.workspace
                        .hosts
                        .iter()
                        .find(|host| host.id == self.file_host)
                        .map(|host| host.name.as_str())
                        .unwrap_or("Selecione um host"),
                )
                .show_ui(ui, |ui| {
                    for host in &self.workspace.hosts {
                        ui.selectable_value(
                            &mut self.file_host,
                            host.id.clone(),
                            format!(
                                "{} ({})",
                                host.name,
                                if host.protocol == Protocol::Ftp {
                                    "FTP"
                                } else {
                                    "SFTP"
                                }
                            ),
                        );
                    }
                });
            if self.busy {
                ui.spinner();
            }
        });
        if before != self.file_host {
            self.entries.clear();
            self.remote_path = "/".into();
            self.ftp_consent = false;
        }
        let host = self
            .workspace
            .hosts
            .iter()
            .find(|host| host.id == self.file_host)
            .cloned();
        if let Some(host) = &host {
            if host.protocol != Protocol::Ftp && ui.button("Verificar servidor via SSH").clicked() {
                let mut target = host.clone();
                target.protocol = Protocol::Ssh;
                self.connect(target);
            }
        }
        if host
            .as_ref()
            .is_some_and(|host| host.protocol == Protocol::Ftp)
        {
            ui.add_space(10.0);
            ui.label(
                RichText::new("FTP comum transmite senha e arquivos sem criptografia.")
                    .color(AMBER),
            );
            ui.checkbox(
                &mut self.ftp_consent,
                "Entendo e autorizo FTP sem criptografia nesta conexão",
            );
        }
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("LOCAL").size(10.0).color(MUTED));
            ui.add_sized(
                [(ui.available_width() - 140.0).max(120.0), 34.0],
                egui::TextEdit::singleline(&mut self.local_path),
            );
            if ui.button("Escolher...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.local_path = path.display().to_string();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("REMOTO").size(10.0).color(MUTED));
            let changed = ui
                .add_sized(
                    [(ui.available_width() - 300.0).max(120.0), 34.0],
                    egui::TextEdit::singleline(&mut self.remote_path),
                )
                .changed();
            if changed {
                self.entries.clear();
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Atualizar"))
                .clicked()
            {
                self.file_job(TransferAction::List {
                    remote: self.remote_path.clone(),
                });
            }
            if ui
                .add_enabled(!self.busy, egui::Button::new("Enviar arquivo..."))
                .clicked()
            {
                if let Some(local) = rfd::FileDialog::new().pick_file() {
                    if let Some(name) = local.file_name().and_then(|name| name.to_str()) {
                        let remote = format!("{}/{}", self.remote_path.trim_end_matches('/'), name);
                        self.file_job(TransferAction::Upload { local, remote });
                    }
                }
            }
        });
        ui.add_space(18.0);
        if self.entries.is_empty() {
            empty_state(ui, Icon::Folder, "Pronto para explorar", "Selecione um host e atualize o diretório. SFTP exige um servidor já verificado no terminal SSH.");
        }
        let mut action = None;
        egui::ScrollArea::vertical()
            .id_salt("remote-files")
            .show(ui, |ui| {
                if ui
                    .add_enabled(!self.busy, egui::Button::new("↑  Diretório pai"))
                    .clicked()
                {
                    self.remote_path = PathBuf::from(&self.remote_path)
                        .parent()
                        .and_then(|parent| parent.to_str())
                        .filter(|parent| !parent.is_empty())
                        .unwrap_or("/")
                        .to_string();
                    action = Some(TransferAction::List {
                        remote: self.remote_path.clone(),
                    });
                }
                for entry in &self.entries {
                    ui.separator();
                    ui.horizontal(|ui| {
                        draw_icon(
                            ui,
                            if entry.is_dir {
                                Icon::Folder
                            } else {
                                Icon::Code
                            },
                            if entry.is_dir { MINT } else { MUTED },
                            18.0,
                        );
                        if ui
                            .add_enabled(!self.busy, egui::Button::new(&entry.name).selected(false))
                            .double_clicked()
                            && entry.is_dir
                        {
                            self.remote_path = entry.path.clone();
                            action = Some(TransferAction::List {
                                remote: entry.path.clone(),
                            });
                        }
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if entry.is_dir {
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("Abrir"))
                                    .clicked()
                                {
                                    self.remote_path = entry.path.clone();
                                    action = Some(TransferAction::List {
                                        remote: entry.path.clone(),
                                    });
                                }
                            } else {
                                if ui
                                    .add_enabled(!self.busy, egui::Button::new("Baixar"))
                                    .clicked()
                                {
                                    action = Some(TransferAction::Download {
                                        remote: entry.path.clone(),
                                        local: PathBuf::from(&self.local_path).join(&entry.name),
                                    });
                                }
                                ui.label(
                                    RichText::new(format_bytes(entry.size))
                                        .color(MUTED)
                                        .size(12.0),
                                );
                            }
                        });
                    });
                }
            });
        if let Some(action) = action {
            self.file_job(action);
        }
    }

    fn file_job(&mut self, action: TransferAction) {
        if self.busy {
            return;
        }
        if self.demo {
            self.notice = Some((
                "Demonstração: transferências de rede desabilitadas.".into(),
                false,
            ));
            return;
        }
        let Some(host) = self
            .workspace
            .hosts
            .iter()
            .find(|host| host.id == self.file_host)
            .cloned()
        else {
            self.notice = Some(("Selecione uma conexão.".into(), true));
            return;
        };
        if host.protocol == Protocol::Ftp && !self.ftp_consent {
            self.notice = Some(("Confirme o uso de FTP sem criptografia.".into(), true));
            return;
        }
        let identity = host
            .identity_id
            .as_ref()
            .and_then(|id| self.workspace.identities.iter().find(|key| &key.id == id))
            .cloned();
        let listing = matches!(action, TransferAction::List { .. });
        if listing {
            self.entries.clear();
        }
        let path = self.remote_path.clone();
        self.busy = true;
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = transfer::execute(&host, identity.as_ref(), action)
                .map_err(|error| format!("Transferência: {error:#}"));
            let _ = sender.send(AppEvent::Transfer {
                host,
                path,
                listing,
                result,
            });
        });
    }

    fn snippets(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| { ui.vertical(|ui| self.header(ui, "Menos repetição. Mais controle.", "Salve comandos recorrentes e copie quando precisar. A execução é sempre manual.")); ui.with_layout(egui::Layout::right_to_left(Align::Min), |ui| { if primary(ui, "+  Novo snippet").clicked() { self.snippet_editor = Some(Snippet { id: new_id(), title: String::new(), command: String::new(), description: String::new() }); } }); });
        if self.workspace.snippets.is_empty() {
            empty_state(ui, Icon::Code, "Seu repertório de comandos", "Organize comandos de manutenção, diagnóstico e deploy sem executar nada automaticamente.");
        }
        for snippet in self.workspace.snippets.clone() {
            egui::Frame::new()
                .fill(CARD)
                .corner_radius(12)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&snippet.title).strong());
                        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Excluir").clicked() {
                                self.confirm_delete = Some(("snippet".into(), snippet.id.clone()));
                            }
                            if ui.button("Editar").clicked() {
                                self.snippet_editor = Some(snippet.clone());
                            }
                            if ui.button("Copiar").clicked() {
                                ui.ctx().copy_text(snippet.command.clone());
                            }
                        });
                    });
                    ui.label(RichText::new(&snippet.description).color(MUTED).size(12.0));
                    ui.add_space(10.0);
                    ui.label(RichText::new(&snippet.command).monospace().color(MINT));
                });
            ui.add_space(12.0);
        }
    }

    fn known_hosts(&self, ui: &mut egui::Ui) {
        self.header(
            ui,
            "Confiança começa na identidade.",
            "As sessões SSH e SFTP compartilham o arquivo known_hosts do seu usuário.",
        );
        ui.label(RichText::new("Compare o fingerprint do servidor por um canal confiável antes de aceitar a primeira conexão no terminal. Chaves alteradas são recusadas.").color(MUTED));
        ui.add_space(20.0);
        let path = std::env::var("HOME").map(|home| PathBuf::from(home).join(".ssh/known_hosts"));
        let contents = if self.demo {
            "# Demonstração: nenhum arquivo pessoal é exibido.".into()
        } else {
            path.ok()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .unwrap_or_else(|| {
                    "# Nenhum arquivo known_hosts encontrado. Conecte a um host SSH primeiro."
                        .into()
                })
        };
        egui::ScrollArea::both().show(ui, |ui| {
            ui.label(RichText::new(contents).monospace().size(12.0).color(MUTED));
        });
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        self.header(
            ui,
            "Simples. Local. Seu.",
            "OpenSSH Manager · v0.1.0 alpha · Licença MIT",
        );
        detail(
            ui,
            "DADOS DO APLICATIVO",
            &self
                .storage
                .as_ref()
                .map(|storage| storage.data_dir.display().to_string())
                .unwrap_or_else(|| "Indisponível".into()),
        );
        detail(
            ui,
            "MOTOR SSH",
            "/usr/bin/ssh · autenticação interativa e SSH Agent",
        );
        detail(ui, "ARMAZENAMENTO", "Metadados JSON · chaves privadas em arquivos 0600 · senhas de transferência no Chaves do macOS");
        detail(
            ui,
            "PRIVACIDADE",
            "Sem conta, sincronização, telemetria ou registro do conteúdo dos terminais.",
        );
        ui.add_space(10.0);
        ui.hyperlink_to(
            "Código-fonte e documentação ↗",
            "https://github.com/jeffersonmello/OpenSSH",
        );
        ui.add_space(24.0);
        if self.demo {
            ui.label(RichText::new("A demonstração usa dados fictícios em memória. Reinicie o app sem --demo para trabalhar com seus dados.").color(AMBER));
        } else if ui.button("Abrir terminal local").clicked() {
            match TerminalSession::local_shell() {
                Ok(session) => {
                    self.sessions.push(SessionTab {
                        id: new_id(),
                        title: "Terminal local".into(),
                        tunnel_id: None,
                        session,
                    });
                    self.active_session = self.sessions.len() - 1;
                    self.page = Page::Terminal;
                }
                Err(error) => self.notice = Some((error.to_string(), true)),
            }
        }
        ui.add_space(20.0);
        ui.label(
            RichText::new(
                "Projeto independente, sem afiliação ao projeto OpenSSH do OpenBSD ou ao Termius.",
            )
            .size(12.0)
            .color(MUTED),
        );
    }

    fn terminal(&mut self, ui: &mut egui::Ui) {
        if self.sessions.is_empty() {
            self.page = Page::Hosts;
            return;
        }
        self.active_session = self.active_session.min(self.sessions.len() - 1);
        let tab = &mut self.sessions[self.active_session];
        ui.horizontal(|ui| {
            draw_icon(ui, Icon::Terminal, MINT, 18.0);
            ui.label(RichText::new(&tab.title).strong());
            if tab.session.is_finished() {
                pill(ui, "SESSÃO ENCERRADA", AMBER);
            }
            ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                if ui.button("Copiar tela").clicked() {
                    ui.ctx().copy_text(tab.session.screen_text());
                }
            });
        });
        ui.label(RichText::new("Clique no terminal para digitar · Ctrl+C interrompe · fechar a aba encerra o processo").size(11.0).color(MUTED));
        ui.add_space(12.0);
        ui.push_id(&tab.id, |ui| {
            tab.session.render(ui);
        });
    }

    fn host_modal(&mut self, ctx: &egui::Context) {
        let Some(mut host) = self.host_editor.take() else {
            return;
        };
        if self.host_editor_id != host.id {
            self.host_tags = host.tags.join(", ");
            self.host_editor_id = host.id.clone();
        }
        let mut keep = true;
        let mut save = false;
        let mut delete = false;
        egui::Window::new("Conexão")
            .id(egui::Id::new("host-editor"))
            .collapsible(false)
            .resizable(false)
            .default_width(480.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                field(ui, "Nome", &mut host.name);
                field(ui, "Endereço (hostname ou IP)", &mut host.address);
                ui.columns(2, |columns| {
                    field(&mut columns[0], "Usuário", &mut host.username);
                    columns[1].label(RichText::new("Porta").color(MUTED).size(12.0));
                    columns[1].add(egui::DragValue::new(&mut host.port).range(1..=65535));
                });
                ui.horizontal(|ui| {
                    let old = host.protocol;
                    egui::ComboBox::from_id_salt("host-protocol")
                        .selected_text(host.protocol.label())
                        .show_ui(ui, |ui| {
                            for protocol in [Protocol::Ssh, Protocol::Sftp, Protocol::Ftp] {
                                ui.selectable_value(&mut host.protocol, protocol, protocol.label());
                            }
                        });
                    if old != host.protocol {
                        host.port = host.protocol.default_port();
                    }
                    ui.checkbox(&mut host.favorite, "Favorito");
                });
                field(ui, "Grupo", &mut host.group);
                field(ui, "Tags (separadas por vírgula)", &mut self.host_tags);
                egui::ComboBox::from_id_salt("host-identity")
                    .width(350.0)
                    .selected_text(
                        host.identity_id
                            .as_ref()
                            .and_then(|id| {
                                self.workspace
                                    .identities
                                    .iter()
                                    .find(|identity| &identity.id == id)
                            })
                            .map(|identity| identity.name.as_str())
                            .unwrap_or("SSH Agent / autenticação interativa"),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut host.identity_id,
                            None,
                            "SSH Agent / autenticação interativa",
                        );
                        for identity in &self.workspace.identities {
                            ui.selectable_value(
                                &mut host.identity_id,
                                Some(identity.id.clone()),
                                &identity.name,
                            );
                        }
                    });
                ui.label(
                    RichText::new("Senha SFTP/FTP (opcional, salva no Chaves do macOS)")
                        .color(MUTED)
                        .size(12.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.host_password)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                if ui.button("Esquecer senha salva").clicked() && !self.demo {
                    let id = host.id.clone();
                    let sender = self.sender.clone();
                    let context = ctx.clone();
                    std::thread::spawn(move || {
                        let _ = sender.send(AppEvent::Credential(
                            vault::delete_password(&id).map_err(|error| error.to_string()),
                        ));
                        context.request_repaint();
                    });
                }
                field(ui, "Notas", &mut host.notes);
                if host.protocol == Protocol::Ftp {
                    ui.label(RichText::new("FTP transmite dados sem criptografia.").color(AMBER));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.writable || self.demo,
                            egui::Button::new(RichText::new("Salvar conexão").color(BG).strong())
                                .fill(MINT),
                        )
                        .clicked()
                    {
                        save = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        keep = false;
                    }
                    if self.workspace.hosts.iter().any(|item| item.id == host.id)
                        && ui.button(RichText::new("Excluir").color(RED)).clicked()
                    {
                        delete = true;
                    }
                });
            });
        if save {
            host.tags = self
                .host_tags
                .split(',')
                .map(str::trim)
                .filter(|tag| !tag.is_empty())
                .map(String::from)
                .collect();
            match host.validate() {
                Ok(()) => {
                    if host.group.trim().is_empty() {
                        host.group = "Pessoal".into();
                    }
                    if let Some(saved) = self
                        .workspace
                        .hosts
                        .iter_mut()
                        .find(|item| item.id == host.id)
                    {
                        *saved = host.clone();
                    } else {
                        self.workspace.hosts.push(host.clone());
                    }
                    self.selected = Some(host.id.clone());
                    if self.file_host == host.id {
                        self.entries.clear();
                        self.ftp_consent = false;
                    }
                    self.persist();
                    keep = false;
                    if !self.host_password.is_empty() && !self.demo {
                        let mut password = std::mem::take(&mut self.host_password);
                        let id = host.id.clone();
                        let sender = self.sender.clone();
                        let context = ctx.clone();
                        std::thread::spawn(move || {
                            let result = vault::set_password(&id, &password)
                                .map_err(|error| error.to_string());
                            password.zeroize();
                            let _ = sender.send(AppEvent::Credential(result));
                            context.request_repaint();
                        });
                    }
                }
                Err(error) => self.notice = Some((error, true)),
            }
        }
        if delete {
            self.confirm_delete = Some(("host".into(), host.id.clone()));
            keep = false;
        }
        if keep {
            self.host_editor = Some(host);
        } else {
            self.host_password.zeroize();
            self.host_password.clear();
            self.host_editor_id.clear();
        }
    }

    fn tunnel_modal(&mut self, ctx: &egui::Context) {
        let Some(mut tunnel) = self.tunnel_editor.take() else {
            return;
        };
        let mut keep = true;
        let mut save = false;
        egui::Window::new("Túnel SSH").collapsible(false).resizable(false).default_width(450.0).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            field(ui, "Nome do túnel", &mut tunnel.name);
            egui::ComboBox::from_id_salt("tunnel-host").width(350.0).selected_text(self.workspace.hosts.iter().find(|host| host.id == tunnel.host_id).map(|host| host.name.as_str()).unwrap_or("Selecione o servidor SSH")).show_ui(ui, |ui| { for host in self.workspace.hosts.iter().filter(|host| host.protocol != Protocol::Ftp) { ui.selectable_value(&mut tunnel.host_id, host.id.clone(), &host.name); } });
            ui.horizontal(|ui| { for kind in [TunnelKind::Dynamic, TunnelKind::Local, TunnelKind::Remote] { ui.selectable_value(&mut tunnel.kind, kind, kind.label()); } });
            ui.label("Porta de escuta (127.0.0.1)"); ui.add(egui::DragValue::new(&mut tunnel.bind_port).range(1..=65535));
            if tunnel.kind != TunnelKind::Dynamic { field(ui, "Host de destino", &mut tunnel.destination_host); ui.label("Porta de destino"); ui.add(egui::DragValue::new(&mut tunnel.destination_port).range(1..=65535)); }
            ui.label(RichText::new("O túnel permanece aberto enquanto sua aba estiver em execução. Túneis remotos dependem da política do servidor.").color(MUTED).size(12.0));
            ui.horizontal(|ui| { if primary(ui, "Salvar túnel").clicked() { save = true; }
if ui.button("Cancelar").clicked() { keep = false; } });
        });
        if save {
            if tunnel.name.trim().is_empty() || tunnel.host_id.is_empty() {
                self.notice = Some(("Informe o nome e selecione um servidor.".into(), true));
            } else if let Some(host) = self
                .workspace
                .hosts
                .iter()
                .find(|host| host.id == tunnel.host_id)
            {
                match openssh_manager::connection::tunnel_arguments(host, None, &tunnel) {
                    Ok(_) => {
                        if let Some(saved) = self
                            .workspace
                            .tunnels
                            .iter_mut()
                            .find(|item| item.id == tunnel.id)
                        {
                            *saved = tunnel.clone();
                        } else {
                            self.workspace.tunnels.push(tunnel.clone());
                        }
                        self.persist();
                        keep = false;
                    }
                    Err(error) => self.notice = Some((error.to_string(), true)),
                }
            }
        }
        if keep {
            self.tunnel_editor = Some(tunnel);
        }
    }

    fn snippet_modal(&mut self, ctx: &egui::Context) {
        let Some(mut snippet) = self.snippet_editor.take() else {
            return;
        };
        let mut keep = true;
        egui::Window::new("Snippet")
            .collapsible(false)
            .default_width(500.0)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                field(ui, "Título", &mut snippet.title);
                field(ui, "Descrição", &mut snippet.description);
                ui.add(
                    egui::TextEdit::multiline(&mut snippet.command)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(5),
                );
                ui.horizontal(|ui| {
                    if primary(ui, "Salvar snippet").clicked() {
                        if snippet.title.trim().is_empty() || snippet.command.trim().is_empty() {
                            self.notice = Some(("Preencha o título e o comando.".into(), true));
                        } else {
                            if let Some(saved) = self
                                .workspace
                                .snippets
                                .iter_mut()
                                .find(|item| item.id == snippet.id)
                            {
                                *saved = snippet.clone();
                            } else {
                                self.workspace.snippets.push(snippet.clone());
                            }
                            self.persist();
                            keep = false;
                        }
                    }
                    if ui.button("Cancelar").clicked() {
                        keep = false;
                    }
                });
            });
        if keep {
            self.snippet_editor = Some(snippet);
        }
    }

    fn delete_modal(&mut self, ctx: &egui::Context) {
        let Some((kind, id)) = self.confirm_delete.clone() else {
            return;
        };
        egui::Window::new("Confirmar exclusão").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.label(if kind == "identity" { "Remover a identidade e os vínculos? O arquivo privado será preservado no disco." } else { "Excluir este item do workspace?" });
            ui.horizontal(|ui| {
                if ui.button(RichText::new("Excluir").color(RED)).clicked() {
                    match kind.as_str() {
                        "host" => { self.workspace.hosts.retain(|host| host.id != id); self.workspace.tunnels.retain(|tunnel| tunnel.host_id != id); if self.selected.as_ref() == Some(&id) { self.selected = None; } }
                        "identity" => { self.workspace.identities.retain(|identity| identity.id != id); for host in &mut self.workspace.hosts { if host.identity_id.as_ref() == Some(&id) { host.identity_id = None; } } }
                        "tunnel" => { self.workspace.tunnels.retain(|tunnel| tunnel.id != id); self.sessions.retain(|tab| tab.tunnel_id.as_ref() != Some(&id)); }
                        "snippet" => self.workspace.snippets.retain(|snippet| snippet.id != id),
                        _ => (),
                    }
                    self.persist(); self.confirm_delete = None;
                }
                if ui.button("Cancelar").clicked() { self.confirm_delete = None; }
            });
        });
    }
}

impl eframe::App for OpenSshApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        self.topbar(ctx);
        self.sidebar(ctx);
        if self.page == Page::Hosts && ctx.screen_rect().width() >= 1250.0 {
            self.details(ctx);
        }
        egui::TopBottomPanel::bottom("status")
            .exact_height(if self.notice.is_some() { 48.0 } else { 31.0 })
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(24, 6)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if let Some((message, error)) = &self.notice {
                        let message = message.clone();
                        let error = *error;
                        if ui.small_button("×").clicked() {
                            self.notice = None;
                        }
                        ui.label(RichText::new(message).size(11.0).color(if error {
                            RED
                        } else {
                            MINT
                        }));
                    } else {
                        ui.label(RichText::new("●").size(8.0).color(MINT));
                        ui.label(
                            RichText::new(if self.demo {
                                "Demonstração · dados fictícios · sem acesso à rede"
                            } else {
                                "Workspace local · pronto para conectar"
                            })
                            .size(10.5)
                            .color(MUTED),
                        );
                    }
                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        if self.busy {
                            ui.spinner();
                            ui.label(RichText::new("Processando...").size(10.0).color(MUTED));
                        } else {
                            ui.label(
                                RichText::new("BUILT WITH RUST")
                                    .size(9.0)
                                    .color(MUTED)
                                    .extra_letter_spacing(1.5),
                            );
                        }
                    });
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(28.0))
            .show(ctx, |ui| match self.page {
                Page::Hosts => self.hosts(ui),
                Page::Keys => self.keys(ui),
                Page::Tunnels => self.tunnels(ui),
                Page::Files => self.files(ui),
                Page::Snippets => self.snippets(ui),
                Page::KnownHosts => self.known_hosts(ui),
                Page::Settings => self.settings(ui),
                Page::Terminal => self.terminal(ui),
            });
        self.host_modal(ctx);
        self.tunnel_modal(ctx);
        self.snippet_modal(ctx);
        self.delete_modal(ctx);
        if self.busy || self.screenshot.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        if let Some(path) = &self.screenshot {
            if !self.screenshot_requested && self.started.elapsed() > Duration::from_secs(2) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.screenshot_requested = true;
            }
            let screenshot = ctx.input(|input| {
                input.events.iter().find_map(|event| {
                    if let egui::Event::Screenshot { image, .. } = event {
                        Some(image.clone())
                    } else {
                        None
                    }
                })
            });
            if let Some(screenshot) = screenshot {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let rgba: Vec<u8> = screenshot
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                match image::save_buffer(
                    path,
                    &rgba,
                    screenshot.width() as u32,
                    screenshot.height() as u32,
                    image::ColorType::Rgba8,
                ) {
                    Ok(()) => eprintln!("Screenshot saved: {}", path.display()),
                    Err(error) => eprintln!("Screenshot failed: {error}"),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl Drop for OpenSshApp {
    fn drop(&mut self) {
        self.host_password.zeroize();
    }
}

fn primary(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(text).color(BG).strong())
            .fill(MINT)
            .stroke(Stroke::NONE)
            .corner_radius(8),
    )
}
fn primary_full(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 40.0],
        egui::Button::new(RichText::new(text).color(BG).strong())
            .fill(MINT)
            .stroke(Stroke::NONE)
            .corner_radius(8),
    )
}
fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.10))
        .corner_radius(5)
        .inner_margin(egui::Margin::symmetric(7, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(9.5).color(color));
        });
}
fn detail(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(
        RichText::new(label)
            .size(9.0)
            .color(MUTED)
            .extra_letter_spacing(1.3),
    );
    ui.add_space(1.0);
    ui.label(RichText::new(value).size(13.0));
    ui.add_space(16.0);
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(RichText::new(label).color(MUTED).size(12.0));
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .margin(Vec2::new(10.0, 9.0)),
    );
}
fn section_label(ui: &mut egui::Ui, title: &str, count: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(10.0)
                .color(MUTED)
                .extra_letter_spacing(1.5),
        );
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(count).size(10.0).color(MUTED));
        });
    });
}
fn empty_state(ui: &mut egui::Ui, icon: Icon, title: &str, description: &str) {
    ui.add_space(35.0);
    icon_tile(ui, icon, MINT, 52.0);
    ui.add_space(18.0);
    ui.label(RichText::new(title).size(22.0).strong());
    ui.label(RichText::new(description).size(13.0).color(MUTED));
    ui.add_space(24.0);
}
fn icon_tile(ui: &mut egui::Ui, icon: Icon, color: Color32, size: f32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.11))
        .corner_radius(10)
        .inner_margin((size - 20.0) / 2.0)
        .show(ui, |ui| {
            draw_icon(ui, icon, color, 20.0);
        });
}
fn nav_item(ui: &mut egui::Ui, icon: Icon, label: &str, selected: bool) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 42.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            8,
            if selected {
                Color32::from_rgb(29, 53, 51)
            } else {
                CARD
            },
        );
    }
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.left_top() + Vec2::new(0.0, 12.0), Vec2::new(3.0, 18.0)),
            2,
            MINT,
        );
    }
    paint_icon(
        ui.painter(),
        rect.left_center() + Vec2::new(22.0, 0.0),
        icon,
        if selected { MINT } else { MUTED },
        18.0,
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(43.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        if selected { MINT } else { MUTED },
    );
    response.clicked()
}
fn draw_icon(ui: &mut egui::Ui, icon: Icon, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    paint_icon(ui.painter(), rect.center(), icon, color, size);
}
fn paint_icon(painter: &egui::Painter, center: egui::Pos2, icon: Icon, color: Color32, size: f32) {
    let s = size / 24.0;
    let p = |x: f32, y: f32| center + Vec2::new((x - 12.0) * s, (y - 12.0) * s);
    let stroke = Stroke::new(1.6 * s, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        painter.line_segment([p(a.0, a.1), p(b.0, b.1)], stroke);
    };
    let rect = |x: f32, y: f32, w: f32, h: f32| {
        painter.rect_stroke(
            egui::Rect::from_min_max(p(x, y), p(x + w, y + h)),
            2,
            stroke,
            egui::StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Server => {
            rect(3.0, 3.0, 18.0, 8.0);
            rect(3.0, 13.0, 18.0, 8.0);
            painter.circle_filled(p(7.0, 7.0), 1.0 * s, color);
            painter.circle_filled(p(7.0, 17.0), 1.0 * s, color);
            line((12.0, 7.0), (17.0, 7.0));
            line((12.0, 17.0), (17.0, 17.0));
        }
        Icon::Key => {
            painter.circle_stroke(p(8.0, 8.0), 4.0 * s, stroke);
            line((11.0, 11.0), (20.0, 20.0));
            line((16.0, 16.0), (19.0, 13.0));
            line((18.0, 18.0), (21.0, 15.0));
        }
        Icon::Folder => {
            line((3.0, 7.0), (3.0, 20.0));
            line((3.0, 20.0), (21.0, 20.0));
            line((21.0, 20.0), (21.0, 7.0));
            line((21.0, 7.0), (12.0, 7.0));
            line((12.0, 7.0), (9.0, 4.0));
            line((9.0, 4.0), (3.0, 4.0));
            line((3.0, 4.0), (3.0, 7.0));
        }
        Icon::Route => {
            painter.circle_stroke(p(5.0, 5.0), 2.0 * s, stroke);
            painter.circle_stroke(p(19.0, 19.0), 2.0 * s, stroke);
            line((7.0, 5.0), (16.0, 5.0));
            line((16.0, 5.0), (19.0, 8.0));
            line((19.0, 8.0), (5.0, 16.0));
            line((5.0, 16.0), (8.0, 19.0));
            line((8.0, 19.0), (17.0, 19.0));
        }
        Icon::Code => {
            line((8.0, 6.0), (3.0, 12.0));
            line((3.0, 12.0), (8.0, 18.0));
            line((16.0, 6.0), (21.0, 12.0));
            line((21.0, 12.0), (16.0, 18.0));
            line((14.0, 4.0), (10.0, 20.0));
        }
        Icon::Shield => {
            line((12.0, 2.0), (21.0, 6.0));
            line((21.0, 6.0), (19.0, 16.0));
            line((19.0, 16.0), (12.0, 22.0));
            line((12.0, 22.0), (5.0, 16.0));
            line((5.0, 16.0), (3.0, 6.0));
            line((3.0, 6.0), (12.0, 2.0));
            line((8.0, 12.0), (11.0, 15.0));
            line((11.0, 15.0), (16.0, 9.0));
        }
        Icon::Terminal => {
            rect(2.0, 3.0, 20.0, 18.0);
            line((6.0, 8.0), (10.0, 12.0));
            line((10.0, 12.0), (6.0, 16.0));
            line((13.0, 16.0), (18.0, 16.0));
        }
        Icon::Search => {
            painter.circle_stroke(p(10.0, 10.0), 6.0 * s, stroke);
            line((15.0, 15.0), (21.0, 21.0));
        }
        Icon::Settings => {
            painter.circle_stroke(p(12.0, 12.0), 4.0 * s, stroke);
            painter.circle_stroke(p(12.0, 12.0), 9.0 * s, stroke);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                line(
                    (12.0 + 8.0 * a.cos(), 12.0 + 8.0 * a.sin()),
                    (12.0 + 11.0 * a.cos(), 12.0 + 11.0 * a.sin()),
                );
            }
        }
        Icon::Star => {
            let points: Vec<_> = (0..10)
                .map(|i| {
                    let a = i as f32 * std::f32::consts::TAU / 10.0 - std::f32::consts::FRAC_PI_2;
                    let r = if i % 2 == 0 { 10.0 } else { 4.5 };
                    p(12.0 + r * a.cos(), 12.0 + r * a.sin())
                })
                .collect();
            painter.add(egui::Shape::closed_line(points, stroke));
        }
    }
}
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    }
}
