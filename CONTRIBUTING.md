# Como contribuir

Obrigado por contribuir com o OpenSSH Manager. O projeto está em alpha e prioriza uma base compreensível, verificável e segura.

## Antes de começar

Leia [README](README.md), [AGENTS.md](AGENTS.md), a [arquitetura](docs/architecture.md) e a especificação da área afetada. Para mudanças grandes, abra uma proposta explicando o problema e o resultado esperado. Correções pequenas e documentação podem ir direto para um pull request.

## Preparar e validar

```bash
git clone https://github.com/jeffersonmello/OpenSSH.git
cd OpenSSH
cargo run --locked -- --demo
./scripts/check.sh
```

Trabalhe em uma branch descritiva. Use dados fictícios; não inclua endereços privados, senhas, tokens ou chaves reais. Testes de rede devem usar laboratório sob seu controle.

## Pull request

Explique o problema e o comportamento final, relacione a spec e informe os comandos e cenários executados. Declare limitações. Se alterar comportamento, atualize documentação e changelog; se alterar arquitetura ou schema, registre decisão e estratégia de migração.

Mantenha a alteração focada, código formatado e `Cargo.lock` compatível. Preserve a licença e avisos de autoria. Evite dependências novas sem uma necessidade concreta.

## Segurança e convivência

Vulnerabilidades seguem [SECURITY.md](SECURITY.md), sem divulgação pública de segredos. Toda participação segue o [Código de Conduta](CODE_OF_CONDUCT.md).

As contribuições são disponibilizadas sob a licença MIT do projeto.
