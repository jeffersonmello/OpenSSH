# OpenSSH Manager

Use `AGENTS.md` como referência comum e a especificação relacionada em `specs/`. O produto é um aplicativo Rust nativo para macOS com egui/eframe.

Preserve a separação entre `models`, `storage`, `vault`, `connection`, `terminal`, `transfer` e `app`. Nunca coloque segredos em JSON/logs nem desative verificação de host. Execute processos com argumentos separados, mantenha rede fora do frame e preserve o modo demo sem rede/persistência.

Valide com `./scripts/check.sh` e os cenários relevantes em `docs/testing.md`. Atualize documentação de comportamento e declare testes de rede não executados.
