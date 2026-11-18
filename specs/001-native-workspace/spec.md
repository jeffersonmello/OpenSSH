# SPEC-001 — Workspace nativo de conexões

- Estado: implementação alpha
- Criada em: 2026-11-18
- Produto: OpenSSH Manager
- Plataforma inicial: macOS

## Problema

Administrar servidores exige alternar entre terminal, arquivos de configuração, chaves e ferramentas de transferência. O usuário quer reunir esses fluxos em um aplicativo Rust open source, com organização visual e documentação suficiente para evolução por colaboradores e agentes.

## Escopo

Workspace local de hosts, identidades e túneis; terminal SSH integrado; SOCKS5, encaminhamento local/remoto; listagem e transferência básica SFTP/FTP; branding e estrutura pública do repositório.

## Requisitos funcionais

| ID | Requisito | Critério de aceite |
| --- | --- | --- |
| RF01 | Gerenciar hosts | Criar, editar, buscar e organizar hosts por grupo, tags e favoritos; persistir entre execuções |
| RF02 | Gerenciar identidades | Importar chave e gerar Ed25519; mostrar chave pública e fingerprint, sem expor conteúdo privado no JSON |
| RF03 | Abrir terminal SSH | Executar cliente SSH em PTY, renderizar saída e encaminhar entrada, resize e Ctrl+C |
| RF04 | Validar confiança | SSH mantém confirmação normal; SFTP rejeita host desconhecido ou alterado antes de autenticar |
| RF05 | Criar túneis | SOCKS5, local e remoto têm portas validadas e ciclo de vida da sessão |
| RF06 | Transferir por SFTP | Listar diretório e enviar/baixar um arquivo sem bloquear a janela |
| RF07 | Transferir por FTP | Exigir confirmação do transporte sem criptografia antes da operação |
| RF08 | Guardar senha de transferência | Usar Keychain por identificador do host, sem senha no workspace |
| RF09 | Explorar demonstração | `--demo` usa dados fictícios, memória e nenhuma conexão de rede |
| RF10 | Documentar e contribuir | README com imagens, quickstart, especificações, instruções Codex/Claude e padrões GitHub |

## Requisitos não funcionais

| ID | Requisito | Evidência esperada |
| --- | --- | --- |
| RNF01 | Rust nativo | Interface e serviços compilados em Rust, sem runtime web de aplicação |
| RNF02 | Responsividade | Rede e leitura de PTY fora do frame de renderização |
| RNF03 | Integridade local | Gravação atômica, schema validado, arquivos privados com permissões restritas |
| RNF04 | Processos seguros | Sem interpolação em shell, validação de host/usuário/porta, listeners de loopback |
| RNF05 | Manutenibilidade | Formatação, Clippy e testes automatizados; documentação do estado real |

## Não incluído

Sincronização, conta online, FTPS, servidor SSH, colaboração, auto-update, Touch ID/FIDO2, certificados SSH, terminal com paridade completa, assinatura Developer ID e notarização.

## Critérios de conclusão da fundação

- Build macOS e verificações automatizadas passam no ambiente utilizado.
- Interface abre e apresenta o workspace de demonstração com dados fictícios.
- Implementações de protocolos existem com testes locais relevantes; cenários de rede não executados ficam explícitos.
- Logo, banner e captura real da aplicação estão versionados e referenciados no README.
- Instruções dos agentes apontam para as mesmas especificações e invariantes de segurança.

## Rastreabilidade

Veja [tasks.md](tasks.md) para implementação e [docs/testing.md](../../docs/testing.md) para a matriz manual. Alterações futuras devem preservar os IDs ou declarar sua substituição.
