# Tarefas e evidências — SPEC-001

Esta lista acompanha a fundação. A existência do código não substitui os cenários manuais de rede.

| Tarefa | Requisitos | Estado | Evidência |
| --- | --- | --- | --- |
| Modelos e validação | RF01, RF05, RNF03 | Implementado na alpha | `src/models.rs` |
| Persistência local | RF01, RNF03 | Implementado na alpha | `src/storage.rs` |
| Identidades e Keychain | RF02, RF08 | Implementado na alpha | `src/vault.rs` |
| Argumentos e PTY | RF03, RF04, RF05, RNF04 | Implementado na alpha | `src/connection.rs`, `src/terminal.rs` |
| Adaptadores de arquivos | RF04, RF06, RF07 | Implementado na alpha | `src/transfer.rs` |
| Interface e demonstração | RF01–RF09, RNF01, RNF02 | Implementado na alpha | `src/app.rs`, `src/main.rs` |
| Documentação e padrões | RF10, RNF05 | Implementado | `README.md`, `docs/`, `.github/`, `AGENTS.md`, `CLAUDE.md` |
| Skills locais | RF10 | Implementado | `.agents/skills/`, `.claude/skills/` |
| Identidade e screenshot | RF10 | Implementado na alpha | `assets/brand/`, `docs/images/` |
| Verificações automatizadas | RNF05 | Validado | `scripts/check.sh`: fmt, Clippy e 17 testes passaram; 2 fixtures opt-in |
| Build e abertura macOS | RNF01, RF09 | Validado em arm64 | Build release; `.app` com assinatura ad hoc abriu; cadastro e bloqueio de conexão em demo verificados |
| Integração SFTP isolada | RF04, RF06 | Validado em loopback | Teste opt-in com `sshd` temporário; 1 passou |
| Integração de túneis isolada | RF05 | Validado em loopback | `tests/tunnels.rs`: SOCKS5, local, remoto e limpeza de listeners; 1 passou |
| Matriz externa de rede/Keychain | RF03–RF08 | Pendente em laboratório | `docs/testing.md` M04–M16; fixtures locais não cobrem todos os cenários |

## Registro de validação

Em 2026-11-18, no macOS 26.6.2 arm64, foi executado:

```bash
cargo test --locked --lib transfer::tests::sftp_roundtrip_against_isolated_local_sshd -- --ignored
```

Resultado: 1 teste passou, 0 falhas. A fixture cobre confiança do servidor, autenticação com chave, listagem, upload, download e conflito remoto em um daemon SSH real de loopback. Usa apenas arquivos temporários; não lê/escreve credenciais ou `known_hosts` pessoais.

A interface foi aberta em modo demo e produziu `docs/images/app-preview.png` por `--screenshot`; a captura e os assets de marca foram inspecionados visualmente. Ambos os skills passaram em `quick_validate.py`, e os scripts passaram em `bash -n`. A suíte completa passou com 17 testes, sem falhas, com formatação e Clippy limpos. As duas fixtures opt-in foram executadas separadamente e passaram: SFTP e SOCKS5/encaminhamentos local e remoto.

O pacote `dist/OpenSSH Manager.app` foi gerado em release para Apple Silicon, com `plutil` e `codesign --verify --deep --strict` aprovados. Na interface do pacote foram verificados primeira abertura, entrada em demonstração, cadastro de host com múltiplas tags e bloqueio de conexão de rede em demo. O build local usa assinatura ad hoc, sem notarização.

A matriz de servidores externos, autenticação interativa completa e Keychain real continua pendente. Não marcar esses cenários como concluídos apenas pelas fixtures locais.
