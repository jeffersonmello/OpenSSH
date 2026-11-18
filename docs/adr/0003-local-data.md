# ADR-0003 — Dados locais e segredos separados

- Estado: aceita
- Data: 2026-11-18

## Contexto

O aplicativo precisa persistir hosts, grupos, preferências e referências a identidades sem criar conta ou serviço remoto. Credenciais exigem proteção diferente de metadados.

## Decisão

Usar `Workspace` JSON com schema versionado e gravação atômica em diretório local da plataforma. Guardar chaves privadas em arquivos com permissões restritas e senhas de transferência no Keychain do macOS. Não persistir segredos no JSON.

## Consequências

O usuário controla os dados e backups. Metadados não são criptografados, e chaves privadas geradas sem passphrase dependem da proteção local. Schema inválido ou desconhecido deve falhar sem sobrescrever o arquivo. Sincronização e exportação protegida exigirão design próprio.
