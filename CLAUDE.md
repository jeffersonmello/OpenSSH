# Claude Code — OpenSSH Manager

Leia e siga [AGENTS.md](AGENTS.md), a referência comum de implementação e segurança deste repositório.

- Especificação inicial: `specs/001-native-workspace/spec.md`.
- Arquitetura: `docs/architecture.md`.
- Verificações: `./scripts/check.sh` e cenários em `docs/testing.md`.
- Skills: `.claude/skills/openssh-development/` e `.claude/skills/ssh-security-review/`, que apontam para as definições canônicas em `.agents/skills/`.

Não mantenha uma segunda cópia divergente das regras de autenticação, persistência ou confiança. Ao entregar uma mudança, registre o comportamento, sua validação real e limites restantes.
