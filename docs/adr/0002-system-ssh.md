# ADR-0002 — SSH do sistema para terminal e túneis

- Estado: aceita
- Data: 2026-11-18

## Contexto

SSH inclui autenticação interativa, `ssh-agent`, confiança em hosts e encaminhamentos. Reimplementar esses fluxos em um novo cliente amplia a superfície de erro. Transferências precisam de operações estruturadas de arquivo.

## Decisão

Executar o cliente SSH do sistema em um PTY para sessões e túneis. Construir argumentos separados e validar entradas; não invocar shell para concatenar comandos. Renderizar o terminal com parser VT100. SFTP usa `ssh2` com verificação estrita de `known_hosts` antes da autenticação.

## Consequências

A autenticação interativa acontece no terminal. As credenciais do Keychain para transferência não são injetadas no processo SSH. SFTP é uma conexão distinta e pode ter compatibilidade de algoritmos diferente do cliente do sistema. A aplicação precisa gerenciar corretamente resize, entrada de teclado, saída e encerramento dos processos.
