# Instruções para agentes — OpenSSH Manager

## Contexto

Aplicativo desktop open source em Rust, inicialmente para macOS. UI egui/eframe; terminal e túneis via SSH do sistema; transferências SFTP via ssh2 e FTP via suppaftp. É independente do projeto OpenSSH do OpenBSD.

Leia a especificação ligada à tarefa em `specs/`, e consulte `docs/architecture.md` antes de alterar limites entre módulos. O estado inicial está em `specs/001-native-workspace/`.

## Fluxo de trabalho

- Para mudança de comportamento, mantenha requisito, implementação e evidência de validação ligados. Uma correção pequena pode atualizar a spec existente.
- Mantenha documentação e changelog compatíveis com o comportamento entregue. Não anuncie cenários de rede que não foram exercitados.
- Use `rg` para localizar símbolos e arquivos. Preserve alterações do usuário.
- Valide com `./scripts/check.sh`; para UI ou transporte, execute também os cenários relevantes em `docs/testing.md`.
- Não publique releases, envie mensagens, faça push ou use credenciais de produção sem autorização da tarefa.

## Invariantes do projeto

- Não desative verificação de chave de host nem aceite silenciosamente uma mudança de fingerprint.
- Construa processos com argumentos separados. Não interpole host, usuário, porta ou caminhos em comandos de shell.
- Senhas e chaves privadas não pertencem ao JSON, logs, screenshots, testes ou issues.
- Chaves privadas locais exigem permissões restritas. Keychain nesta versão guarda senhas de transferência; não descreva-o como armazenamento de chaves privadas.
- A rede deve ficar fora do frame egui. Gerencie encerramento e reaproveitamento dos recursos dos processos.
- Túneis usam loopback. Mudanças de exposição ou forwarding de agente exigem design explícito dentro do escopo solicitado.
- `--demo` permanece sem rede e sem persistência de dados ou segredos.
- Preserve `LICENSE` e a autoria original.

## Skills locais

- `.agents/skills/openssh-development/SKILL.md`: mudanças de implementação do produto.
- `.agents/skills/ssh-security-review/SKILL.md`: revisões de transporte, credenciais e confiança.

As mesmas skills são acessíveis por `.claude/skills/`. Evite duplicar políticas entre ferramentas; esta é a referência comum, e `CLAUDE.md` apenas encaminha para ela.

## Convenções

Código e identificadores em inglês. Documentação principal e mensagens de interface em português quando consistente com o contexto existente. Dados de demonstração usam domínios reservados e nomes fictícios. O produto se chama OpenSSH Manager; o binário é `openssh-manager`.
