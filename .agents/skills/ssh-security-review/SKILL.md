---
name: ssh-security-review
description: Revisar autenticação, confiança em hosts, argumentos de processos, chaves, transferências e túneis do OpenSSH Manager. Use para revisão ou mudança nesses limites de segurança do aplicativo.
---

# Revisão de segurança do transporte

Leia `docs/security.md` e localize o fluxo de entrada até o transporte. Verifique o comportamento real, com arquivo e linha, em vez de inferir proteção pelo nome de uma biblioteca.

Inspecione os limites que a mudança atravessa:

- Host, usuário, porta e caminho chegam ao processo como argumentos separados e validados; nenhum segredo vai para argv.
- SSH mantém verificação de host. SFTP verifica `known_hosts` antes de enviar credenciais e rejeita desconhecidos ou alterações.
- Senha fica no Keychain; chave privada em arquivo restrito. JSON, logs, erro e screenshot não carregam o segredo.
- Listeners ficam em loopback. Encerre e reaproveite recursos do processo quando a sessão terminar; considere a política `GatewayPorts` ao avaliar túnel remoto.
- Download não sobrescreve arquivo local existente. Entrada remota não deve escolher caminhos locais fora do destino solicitado.
- FTP é identificado como transporte em claro com confirmação explícita. Não confunda FTP, FTPS e SFTP.

Use testes direcionados quando puderem provar a invariante. Execute cenários de rede apenas em ambientes autorizados. Relate achados acionáveis com condição de disparo, impacto, localização e correção; registre limites de validação sem declarar uma auditoria completa.

Não remova proteção de confiança para contornar falhas. A tarefa autoriza corrigir o código do projeto, mas não concede autorização para acessar servidores, publicar material ou usar credenciais externas.
