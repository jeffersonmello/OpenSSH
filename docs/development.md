# Desenvolvimento

## Ambiente

O alvo inicial é macOS. Instale as ferramentas de desenvolvimento Apple e o Rust estável usando os instaladores oficiais. O arquivo `rust-toolchain.toml` solicita `rustfmt` e `clippy`; `Cargo.lock` fixa o grafo de dependências utilizado pela aplicação.

```bash
rustup show
cargo run --locked
cargo run --locked -- --demo
```

Use `cargo run --release --locked` para avaliar interação e consumo em uma build otimizada. A primeira compilação inclui dependências nativas de gráficos e libssh2/OpenSSL e pode demorar.

## Verificações

```bash
./scripts/check.sh
```

Equivale a:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
```

A CI repete essas verificações em macOS e compila a aplicação. Resultados de CI só podem ser anunciados depois da execução correspondente. `cargo audit` pode ser usado localmente quando instalado, mas não é pré-requisito deste bootstrap.

## Build de distribuição

```bash
./scripts/bundle-macos.sh
```

O script compila release, cria `dist/OpenSSH Manager.app`, inclui ícone, metadados e licença e aplica assinatura ad hoc. Para outro target:

```bash
rustup target add x86_64-apple-darwin
./scripts/bundle-macos.sh x86_64-apple-darwin
```

O script exige macOS, Xcode Command Line Tools e um target compatível. Não combina arquiteturas em um universal binary. Builds assinadas com Developer ID, notarização e instalador DMG exigem uma etapa futura com identidade de distribuição do mantenedor.

O workflow de release cria pacotes por arquitetura e os guarda como artifacts do workflow. Não publica automaticamente uma release pública. Para publicar, revise a versão, o changelog, os artifacts e os resultados de validação e use o fluxo de distribuição do mantenedor.

## Dados locais

Não utilize seus únicos exemplares de uma chave para testar importação. Faça backup dos dados locais antes de mudanças de schema. Não exclua o diretório do aplicativo para contornar erro de leitura; preserve o arquivo para diagnóstico e recuperação.

Use apenas servidores de laboratório e nomes fictícios em demonstrações, testes e documentação. O modo `--demo` não deve iniciar processos de conexão nem gravar dados ou credenciais.

## Fluxo de implementação

1. Leia `AGENTS.md` e a especificação relacionada.
2. Registre critérios de aceite observáveis e o impacto nos dados.
3. Implemente a menor alteração completa.
4. Faça testes direcionados às invariantes alteradas.
5. Valide a interface ou o protocolo afetado e anote o que não foi exercitado.
6. Atualize spec, tarefas, documentação e changelog se houver mudança visível.

Use [o template](../specs/templates/feature.md) para novos recursos. Evite atualizar o status de uma tarefa somente porque existe um botão correspondente: o fluxo precisa alcançar seu resultado real.
