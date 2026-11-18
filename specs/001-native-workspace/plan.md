# Plano — SPEC-001

## Camadas

1. `models`: schema e validação.
2. `storage` e `vault`: persistência, identidades e credenciais.
3. `connection`, `terminal` e `transfer`: adaptadores.
4. `app`: navegação, formulários, terminal e workers.
5. Documentação, skills e automação do repositório.

## Decisões

- UI Rust com egui/eframe: [ADR-0001](../../docs/adr/0001-native-rust.md).
- PTY + SSH do sistema para terminal/túneis; ssh2 para SFTP: [ADR-0002](../../docs/adr/0002-system-ssh.md).
- JSON local, arquivos de chave restritos e Keychain: [ADR-0003](../../docs/adr/0003-local-data.md).
- FTP comum fica explícito como transporte sem criptografia.
- Modo demo usa `example.com`, mantém alterações em memória e não conecta.

## Estratégia de validação

Testes automatizados para construção de argumentos, modelos e dados locais. Build e abertura da interface para validar integração nativa. Cenários de rede usam hosts descartáveis sob controle do mantenedor e devem ser registrados separadamente da validação unitária.

## Riscos

Diferenças entre cliente OpenSSH e libssh2, compatibilidade parcial VT100, autenticação interativa do Keychain, callbacks de interface e encerramento de processos. Cada risco tem cenários correspondentes em [testing.md](../../docs/testing.md).

## Entrega

Código, `Cargo.lock`, assets finais, documentação, templates GitHub, CI e script `.app`. A publicação externa e a distribuição notarizada ficam fora deste bootstrap.
