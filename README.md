<p align="center">
  <img src="assets/brand/logo.png" width="112" alt="Logo OpenSSH Manager" />
</p>

<h1 align="center">OpenSSH Manager</h1>
<p align="center">Suas conexões. Suas chaves. Seu ambiente.</p>
<p align="center">Um workspace desktop open source para SSH, túneis e transferência de arquivos, escrito em Rust.</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-2021-B7410E?logo=rust&logoColor=white" /></a>
  <a href="https://github.com/emilk/egui"><img alt="egui / eframe" src="https://img.shields.io/badge/UI-egui%20%2F%20eframe-64D8B1" /></a>
  <a href="https://www.apple.com/macos/"><img alt="macOS first" src="https://img.shields.io/badge/macOS-first-111827?logo=apple&logoColor=white" /></a>
  <a href="LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue" /></a>
  <a href="specs/001-native-workspace/spec.md"><img alt="Status alpha" src="https://img.shields.io/badge/status-alpha-F5B85B" /></a>
</p>

![Banner do projeto](assets/brand/banner.png)

> **Alpha 0.1.0.** Projeto em desenvolvimento, sem auditoria independente de segurança. Comece com hosts de teste. Este projeto é independente e não é afiliado ao projeto OpenSSH do OpenBSD, nem ao Termius.

## Visão do projeto

O OpenSSH Manager reúne conexões, identidades, terminais, encaminhamentos de portas e arquivos em um aplicativo nativo. O foco inicial é macOS, com interface e lógica em Rust, sem Electron ou servidor de aplicação. Os fluxos de organização de hosts são inspirados em ferramentas como Termius; a identidade visual e a implementação são próprias.

![Interface real do aplicativo com dados fictícios](docs/images/app-preview.png)

A captura usa o modo de demonstração. Endereços `example.com` e informações de exemplo não representam ambientes reais. O logo e o banner são imagens geradas por IA; os prompts e a origem ficam em [Identidade visual](docs/branding.md).

## Recursos e limites

| Área | Alpha inicial | Próximos passos |
| --- | --- | --- |
| Hosts | Cadastro, edição, grupos, tags, favoritos e busca local | Importação de `~/.ssh/config`, operações em lote |
| Terminal SSH | Sessão integrada por PTY usando o cliente SSH do sistema | Seleção avançada, busca, maior compatibilidade de terminal |
| Identidades | Importação e geração de Ed25519; metadados e fingerprint | Passphrases na geração, certificados e integração com hardware |
| Senhas | Armazenamento no Keychain do macOS para transferências | Fluxos unificados de autenticação |
| Túneis | SOCKS5, encaminhamento local e remoto via SSH | Monitoramento, reconexão e dependências entre túneis |
| Arquivos | Listagem, upload e download de arquivos via SFTP e FTP | Fila com progresso, cancelamento e operações recursivas |
| Snippets | Cadastro, edição e cópia de comandos; execução manual | Organização e integração com sessões |
| Segurança do servidor | SSH confirma novas chaves; SFTP exige `known_hosts` válido | Gerenciador visual de confiança |
| Dados | Workspace JSON local, gravação atômica e permissões restritas | Migrações de schema e exportação protegida |
| Plataformas | macOS como alvo inicial | Validação e empacotamento de Linux e Windows |

FTP comum transmite credenciais e arquivos sem criptografia. Seu uso exige confirmação explícita no aplicativo. Prefira SFTP sempre que possível. FTPS ainda não é oferecido.

A alpha não oferece sincronização em nuvem, colaboração, Touch ID, FIDO2, reconexão automática ou equivalência completa com terminais maduros. A importação aceita chaves sem passphrase; para usar chaves protegidas, carregue-as no `ssh-agent` e selecione autenticação pelo agente. Chaves privadas ficam em arquivos locais com permissão `0600`; **não** são armazenadas no Keychain. Os metadados do workspace não são criptografados.

## Executar no macOS

### Pré-requisitos

- macOS com as ferramentas de desenvolvimento da Apple (`xcode-select --install` se necessário).
- Toolchain Rust estável por [rustup](https://rustup.rs/), com `rustfmt` e `clippy`.
- Cliente SSH do sistema (`/usr/bin/ssh` e `/usr/bin/ssh-keygen`).

```bash
git clone https://github.com/jeffersonmello/OpenSSH.git
cd OpenSSH
cargo run --locked
```

Para conhecer a interface sem criar conexões reais:

```bash
cargo run --locked -- --demo
```

O modo de demonstração usa um workspace em memória e bloqueia conexões de rede. Para usar seus próprios servidores, reinicie sem `--demo`.

### Primeira conexão

1. Cadastre nome, endereço, usuário e porta de um host de teste.
2. Escolha uma identidade importada ou gerada, ou use seu `ssh-agent`.
3. Abra o terminal SSH. Na primeira conexão, confira o fingerprint do servidor por um canal confiável antes de aceitar a chave.
4. Para SFTP, abra **Arquivos → Verificar servidor via SSH** e valide o fingerprint no terminal. A transferência rejeita chaves desconhecidas ou alteradas.
5. Cadastre um túnel e inicie a sessão correspondente. SOCKS usa um listener local; configure seu cliente para SOCKS5 em `127.0.0.1:<porta>` e resolução DNS pelo proxy, quando suportado.

Conexões gerenciadas ignoram `~/.ssh/config` (`-F /dev/null`), preservando `known_hosts` e `ssh-agent`. Informe o hostname ou IP real; aliases, `ProxyCommand` e regras de jump host desse arquivo ainda não são importados.

Senhas solicitadas pelo terminal são digitadas na própria sessão; a senha salva para transferências não é injetada no cliente SSH. Nunca cole chaves privadas em issues, screenshots ou arquivos do repositório.

Excluir uma identidade remove seu cadastro e vínculos, preservando o arquivo privado no disco. Senhas salvas podem ser removidas pelo botão **Esquecer senha salva** no editor do host; excluir o cadastro do host não apaga automaticamente sua entrada do Keychain.

### Gerar um aplicativo `.app`

```bash
./scripts/bundle-macos.sh
open "dist/OpenSSH Manager.app"
```

Para compilar outra arquitetura, instale o target e passe-o ao script:

```bash
rustup target add aarch64-apple-darwin
./scripts/bundle-macos.sh aarch64-apple-darwin
```

O pacote local recebe assinatura ad hoc, sem identidade Apple Developer ou notarização. Não há instalador publicado por esta alteração. Leia o [guia de desenvolvimento](docs/development.md) antes de distribuir builds.

## Desenvolvimento

```bash
./scripts/check.sh
```

O script executa formatação, Clippy e testes com `Cargo.lock`. Testes de rede, Keychain e interface têm cenários explícitos em [Validação](docs/testing.md); testes unitários não substituem a validação em servidores reais.

```text
src/
  app.rs          interface e coordenação das tarefas
  models.rs       entidades e schema do workspace
  storage.rs      persistência local
  vault.rs        identidades e Keychain
  connection.rs   argumentos seguros para o cliente SSH
  terminal.rs     PTY, parser VT100 e entrada de teclado
  transfer.rs     adaptadores SFTP e FTP
specs/            requisitos, plano, tarefas e critérios de aceite
docs/             arquitetura, decisões, segurança e operação
.agents/skills/   skills locais para Codex e Claude
.github/          CI, contribuição e templates
```

## Documentação

- [Produto e escopo](docs/product.md)
- [Arquitetura](docs/architecture.md) e [decisões técnicas](docs/adr/README.md)
- [Desenvolvimento e build](docs/development.md)
- [Testes e validação manual](docs/testing.md)
- [Modelo de segurança](docs/security.md) e [reporte de vulnerabilidades](SECURITY.md)
- [Especificação inicial](specs/001-native-workspace/spec.md), [plano](specs/001-native-workspace/plan.md) e [tarefas](specs/001-native-workspace/tasks.md)
- [Roadmap](docs/roadmap.md) e [histórico](CHANGELOG.md)
- [Identidade visual](docs/branding.md)

## Trabalhar com agentes

[AGENTS.md](AGENTS.md) orienta o Codex; [CLAUDE.md](CLAUDE.md) usa as mesmas regras de projeto para Claude Code. As skills canônicas ficam em `.agents/skills/` e são disponibilizadas ao Claude por links em `.claude/skills/`:

- `openssh-development`: implementar uma alteração ligando requisito, código e validação.
- `ssh-security-review`: revisar autenticação, confiança do servidor, processos, arquivos e túneis.

O fluxo é **especificar → planejar → implementar → validar → atualizar documentação**. Uma correção pequena pode atualizar a especificação existente; uma mudança de comportamento maior recebe uma nova pasta em `specs/`, a partir de [templates](specs/templates/feature.md).

## Contribuir

Issues, revisões, documentação e contribuições de código são bem-vindas. Consulte [CONTRIBUTING.md](CONTRIBUTING.md) e o [Código de Conduta](CODE_OF_CONDUCT.md). Antes de propor uma grande funcionalidade, descreva o problema e os critérios de aceite em uma issue.

## Licença e autoria

[MIT](LICENSE) · Copyright © 2026 Jefferson Mello Olynyki.

O nome do repositório é OpenSSH; o produto é identificado como **OpenSSH Manager** para deixar explícito seu papel de gerenciador independente. OpenSSH e outras marcas citadas pertencem aos respectivos titulares.
