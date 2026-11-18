# Tarefas e evidências — SPEC-002

| Tarefa | Requisitos | Estado | Evidência |
| --- | --- | --- | --- |
| Estilo arredondado e grade | RF01, RF11 | Validado visualmente | `src/app.rs`, `src/app/keys.rs` |
| Editor de identidade | RF02–RF04, RF07 | Implementado e testado localmente | `src/app/keys.rs`, `src/vault.rs`, testes de rascunho e máscara |
| Identidades protegidas | RF03, RNF01, RNF02 | Validado localmente | Geração/importação arquivo/texto, passphrase incorreta, bytes criptografados e permissões |
| Ação de Keychain por identidade | RF10 | Implementado; Keychain real não exercitado | Ação após metadados salvos; atualização isolada valida a chave; nome/vínculos preservam entrada |
| Vínculos e seletor no host | RF05, RF06, RF08, RF09 | Implementado; domínio testado | Vínculos múltiplos, substituição, preservação de outros hosts e rejeição atômica de FTP |
| Compatibilidade de JSON | RNF03 | Validado | Identidades antigas recebem `encrypted: false`; schema permanece `1` |
| Demonstração de chaves | RF12 | Implementado; captura criada | `--demo --page keys`, `docs/images/keys-preview.png`; captura inspecionada |
| Documentação e especificação | Todos | Atualizada | README, docs, ADR-0004 e SPEC-002 |
| Verificações automatizadas | RNF01–RNF03 | Validado | `scripts/check.sh`: fmt/Clippy limpos; 27 testes passaram; 2 fixtures opt-in |
| SFTP real em loopback | RF03, RF10 / adaptador | Validado com segredo injetado apenas no fixture | `sshd` isolado, chave criptografada aceita, passphrase incorreta recusada; sem Keychain pessoal |
| Pacote macOS e QA interativa | RF11, RF12 | Validado em arm64 | Rebuild release, plutil/codesign aprovados; navegação e novo cadastro inspecionados no aplicativo |

## Registro de validação — 2026-11-18

Ambiente: macOS 26.6.2 arm64, árvore de trabalho da SPEC-002.

`./scripts/check.sh` passou com formatação e Clippy sem avisos e **27 testes**: 12 de biblioteca, 3 de interface e 12 de serviços. As duas fixtures de rede opt-in permanecem ignoradas na suíte padrão. A execução completa usou um ambiente com permissão de sockets de loopback; uma tentativa anterior em ambiente restrito foi bloqueada na abertura do listener.

Os testes headless do editor verificam a saída real de pintura e clipboard do campo mascarado, a impossibilidade de desfazer para recuperar segredo após revelar/limpar o campo e o rascunho de identidade existente com ID e vínculos preservados. São evidências automatizadas específicas; não equivalem a uma sessão de navegação interativa completa.

A integração SFTP isolada com `/usr/sbin/sshd` também passou separadamente, incluindo autenticação com chave criptografada e recusa de passphrase incorreta. O fixture usa segredos e arquivos temporários; **Keychain real não foi exercitado**.

A captura `docs/images/keys-preview.png` foi criada em modo demo. O painel existente mostra estado privado oculto, ação explícita de substituição, metadados públicos e vínculos. A captura foi inspecionada. O pacote release arm64 foi reconstruído com assinatura ad hoc; `plutil -lint` e `codesign --verify --deep --strict` passaram. Pela interface nativa, foram conferidos navegação, cantos arredondados e cadastro lateral com nome, conteúdo privado oculto, passphrase e seleção de conexões. Nenhum segredo real foi inserido e nenhum vínculo real foi alterado nesta inspeção. A confirmação do utilitário de ícones exigiu execução fora do sandbox do macOS.

## Limites

Autenticação externa completa, Keychain real e matriz de compatibilidade permanecem pendentes. As verificações da SPEC-001 continuam como evidências históricas da fundação, sem validar automaticamente os fluxos novos.
