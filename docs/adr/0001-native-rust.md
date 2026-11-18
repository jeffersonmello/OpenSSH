# ADR-0001 — Interface nativa em Rust

- Estado: aceita
- Data: 2026-11-18

## Contexto

O produto inicia no macOS e busca um aplicativo desktop de bom desempenho com Rust como linguagem principal. O projeto precisa de uma base simples para evoluir sem duas camadas de aplicação.

## Decisão

Usar `egui` com `eframe` e renderer Glow. Modelos e serviços ficam em módulos Rust separados da interface. Fixar o grafo resolvido no `Cargo.lock`.

## Alternativas consideradas

Tauri permitiria uma interface baseada em tecnologias web, porém acrescentaria frontend e bridge. AppKit direto oferece integração profunda, com maior custo de construção de componentes. egui permite iniciar com um único ecossistema Rust e layouts próprios.

## Consequências

Há um único build Rust para a aplicação. A aparência e a acessibilidade não equivalem automaticamente a componentes AppKit; atalhos, seleção de terminal, foco e integração de plataforma exigem validação explícita. Não há promessa quantitativa de desempenho antes de medições.
