# Changelog

As mudanças relevantes do produto são registradas aqui. O projeto segue a intenção de [Semantic Versioning](https://semver.org/), com possíveis mudanças de comportamento durante a fase `0.x`.

## [Unreleased]

## [0.1.0] — 2026-11-18

### Fundação alpha

- Aplicativo Rust nativo com workspace visual para hosts, identidades e túneis.
- Terminal SSH integrado por PTY e cliente do sistema.
- Configuração de SOCKS5 e encaminhamentos locais e remotos.
- Adaptadores de transferência SFTP e FTP e credenciais de transferência no Keychain.
- Modo de demonstração com dados fictícios.
- Identidade visual, documentação, especificação inicial, instruções Codex/Claude e skills locais.
- CI e scripts de verificação e empacotamento macOS.

### Limites

Sem sincronização, FTPS, assinatura Developer ID, notarização ou paridade completa de terminal. SFTP, FTP e túneis foram validados em loopback; a matriz de servidores externos e o Keychain real continuam pendentes. Veja `docs/testing.md`.
