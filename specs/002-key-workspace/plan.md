# Plano — SPEC-002

## Interface

Aplicar raios consistentes ao estilo egui e aos componentes próprios. A página de chaves usa grade de cartões e painel lateral de edição. Nome, entrada privada mascarada, passphrase opcional, importação de arquivo e referências públicas ficam no mesmo fluxo. Identidade existente mostra um bloco de estado e ação explícita para substituição; o arquivo privado não é carregado na interface. Campo privado vazio ao editar preserva o material atual. A seção de passphrase/SFTP fica recolhida ao editar uma chave existente e aberta para nova importação/geração.

Os vínculos são editados como seleção de hosts SSH/SFTP. O editor informa quando um host já pertence a outra identidade. O editor do host oferece escolha legível, com algoritmo/fingerprint e acesso à página de chaves.

## Domínio

A associação continua armazenada em `Host.identity_id`. Helpers de domínio validam identidade/hosts antes de aplicar mudanças, removem vínculos desmarcados desta identidade e rejeitam FTP. O ID da identidade é preservado em edição ou substituição de material.

Adicionar `Identity.encrypted` com default `false` na leitura do JSON. Manter `Workspace.schema_version = 1` e cobertura de arquivos antigos.

## Material privado e passphrase

Gerar Ed25519 com passphrase opcional e validar/descriptografar chaves OpenSSH em memória com `ssh-key`/RustCrypto. Preservar o conteúdo protegido no arquivo gerenciado. Não passar passphrase em argv de `ssh-keygen` ou `ssh`.

A opção de guardar passphrase usa uma entrada do Keychain por identidade, distinta da senha por host. Executar a ação de guardar/limpar somente depois da gravação de metadados bem-sucedida. Nome/vínculos preservam a credencial; substituição de material sem nova passphrase salva limpa a anterior. A atualização só da passphrase valida a chave existente, sem nova cópia privada. O adaptador SFTP consulta esse segredo para a chave selecionada. Cliente SSH e túneis mantêm autenticação interativa. Veja [ADR-0004](../../docs/adr/0004-protected-identities.md).

## Falhas e demonstração

Validar antes de substituir dados existentes. Passphrase incorreta ou arquivo inválido deixa o cadastro anterior intacto. O modo demo apresenta apenas metadados fictícios e não lê/grava chaves ou Keychain.

## Evidências

Executar testes de domínio/vault/schema e verificações de projeto. Abrir o editor real, verificar campos e vínculos em demo e gerar `docs/images/keys-preview.png` com `--demo --page keys --screenshot`. Atualizar resultados apenas depois da execução.
