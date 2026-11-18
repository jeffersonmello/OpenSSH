# Roadmap

O roadmap expressa prioridades, sem promessa de data. O status detalhado da entrega inicial fica em [tarefas](../specs/001-native-workspace/tasks.md).

## 0.1 — Fundação macOS

- Workspace local, organização de hosts e identidades.
- Terminal SSH integrado e ciclo de vida das sessões.
- Túneis SOCKS5, locais e remotos.
- Transferência básica de arquivos por SFTP e FTP.
- Especificações, instruções de agentes e padrões do repositório.

## 0.2 — Confiança e operação

- Fluxo visual de verificação de fingerprints e gestão de hosts conhecidos.
- Criação de chaves com passphrase sem segredos em argumentos.
- Cancelamento de transferências, progresso e fila.
- Importação de `~/.ssh/config` com confirmação de conflitos.
- Backup e exportação protegida do workspace.
- Cobertura de integração em laboratório para os protocolos.

## 0.3 — Terminal e produtividade

- Seleção, cópia, busca e histórico de terminal mais completos.
- Atalhos configuráveis, snippets mais avançados e múltiplos painéis.
- Reconexão opcional com políticas de retry explícitas.
- FTPS com validação de certificados.
- Operações de arquivos em lote e diretórios recursivos.

## Distribuição e plataformas

- Builds macOS assinadas e notarizadas, Intel e Apple Silicon.
- Avaliação de Linux e Windows baseada em testes reais.
- Revisão externa de segurança e medições de desempenho documentadas.
- Melhorias de acessibilidade e localização.

## Ideias que precisam de design próprio

Sincronização, compartilhamento de credenciais, Touch ID, FIDO2, certificados SSH, jump hosts e roteamento encadeado alteram os limites de confiança. Cada uma exige especificação e decisão arquitetural antes da implementação.
