---
name: openssh-development
description: Implementar ou corrigir funcionalidades do aplicativo Rust OpenSSH Manager, ligando especificações, módulos e validação. Use para mudanças neste repositório, não para administrar servidores SSH externos.
---

# Desenvolvimento do OpenSSH Manager

Leia `AGENTS.md` e a spec ligada à demanda. Consulte `docs/architecture.md` quando alterar transporte, persistência ou coordenação da interface.

O produto usa egui/eframe; sessões e túneis usam o SSH do sistema por PTY, enquanto SFTP tem transporte ssh2 separado. Senhas de transferência usam Keychain; chaves privadas permanecem em arquivos restritos. Essas diferenças são relevantes para implementar autenticação e explicar o resultado ao usuário.

Para mudança visível, mantenha um critério de aceite em `specs/`. Uma correção pequena pode usar a spec existente; uma funcionalidade maior pode partir de `specs/templates/feature.md`. Não acrescente um processo de aprovação além do que a tarefa e seu ambiente já exigem.

Implemente no módulo responsável e mantenha rede fora do frame egui. Preserve `--demo` em memória, sem rede ou gravação de credenciais. Use dados fictícios em testes e capturas.

Execute `./scripts/check.sh` e os cenários afetados de `docs/testing.md`. Atualize as tarefas e a documentação com as evidências reais. Diferencie build/teste local de interoperabilidade com servidores e Keychain, que precisam de validação própria.
