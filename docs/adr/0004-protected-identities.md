# ADR-0004 — Identidades protegidas e vínculos estáveis

- Estado: aceita
- Data: 2026-11-18
- Relacionada: [SPEC-002](../../specs/002-key-workspace/spec.md)

## Contexto

O editor precisa aceitar chaves OpenSSH protegidas e permitir que uma identidade seja usada por várias conexões. O caminho inicial baseado em `ssh-keygen` com passphrase vazia não satisfaz esse fluxo. Passar passphrase não vazia como argumento exporia o segredo na lista de processos.

## Decisão

Usar `ssh-key`/RustCrypto para validar chaves OpenSSH e sua passphrase em memória. Preservar o material privado criptografado em arquivo restrito. Oferecer armazenamento opcional da passphrase no Keychain, em entrada por identidade, para autenticação SFTP.

Manter `Host.identity_id` como referência e preservar `Identity.id` ao editar nome ou substituir material. Metadado `encrypted` usa default `false`, permitindo abrir o schema `1` existente. Vínculos só se aplicam a SSH/SFTP e cada host possui no máximo uma identidade.

## Consequências

O usuário pode cadastrar uma chave protegida sem remover sua proteção em disco. Quem substitui uma identidade compartilhada altera as próximas autenticações de todos os hosts vinculados; o editor deve mostrar esse alcance. O cliente SSH continua solicitando passphrase de forma interativa, independentemente de ela estar salva para o SFTP.

Passphrase e material descriptografado existem temporariamente em memória e exigem buffers de vida curta e limpeza quando aplicável. Keychain não armazena a chave privada. A confirmação de fingerprint do servidor e demais limites de confiança permanecem inalterados.
