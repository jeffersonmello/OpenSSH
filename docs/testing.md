# Validação

## Testes automatizados

Execute `./scripts/check.sh`. Os testes cobrem invariantes de comandos, persistência e operações locais. Uma fixture FTP usa sockets de loopback para exercitar listagem, upload e download sem serviço público. Não dependem de credenciais reais, Keychain desbloqueado ou servidores da internet.

### Integração SFTP isolada

No macOS, há um teste opt-in que inicia um `sshd` temporário em loopback com chaves, configuração e `known_hosts` temporários:

```bash
cargo test --locked --lib transfer::tests::sftp_roundtrip_against_isolated_local_sshd -- --ignored
```

O cenário verifica rejeição de host desconhecido e chave alterada, autenticação com chave, listagem, upload, download e recusa de sobrescrita remota. A evolução da SPEC-002 também exercita chave criptografada e recusa de passphrase incorreta com segredos temporários fornecidos diretamente ao fixture, sem Keychain. Exige `/usr/sbin/sshd` e uma conta local apta a autenticar por SSH. Não altera `~/.ssh/known_hosts`, Keychain ou configuração do serviço SSH do macOS; o daemon é encerrado ao fim. Fica ignorado na suíte padrão por depender dessas condições do ambiente.

### Integração de túneis isolada

```bash
cargo test --locked --test tunnels -- --ignored --nocapture
```

O teste inicia outro `sshd` temporário em loopback e usa os argumentos de túneis do aplicativo com um registro de confiança temporário. Verifica handshake SOCKS5, entrega de payload por SOCKS5, encaminhamento local e remoto e fechamento dos listeners ao encerrar o SSH. Chaves e processos de laboratório são descartados ao fim. Também fica ignorado por padrão por exigir um ambiente macOS apto a executar `sshd`.

### Evidências da fundação — 2026-11-18

Ambiente: macOS 26.6.2, Apple Silicon (`arm64`), árvore de trabalho da implementação inicial.

| Verificação | Resultado |
| --- | --- |
| Teste opt-in SFTP acima | 1 teste passou; 0 falhas |
| Teste opt-in de túneis acima | 1 teste passou; SOCKS5, local e remoto transportaram payloads; listeners encerrados |
| Geração de screenshot via `cargo run --locked -- --demo --screenshot docs/images/app-preview.png` | Aplicativo abriu e produziu captura real; inspeção visual realizada |
| Skills locais com `quick_validate.py` | Ambas válidas |
| Sintaxe de `scripts/check.sh` e `scripts/bundle-macos.sh` | `bash -n` passou |
| `scripts/bundle-macos.sh invalid-target` | Rejeitou o target inválido sem compilar |
| `./scripts/check.sh` | Formatação e Clippy sem avisos; 17 testes passaram; fixtures SFTP e túneis são opt-in |
| `./scripts/bundle-macos.sh` | Build release arm64 concluído; `.app` criado; `plutil` e verificação da assinatura ad hoc passaram |
| Abertura do `.app` por interface nativa | Janela inicial, demonstração e formulário inspecionados; host fictício cadastrado com tags `alpha` e `beta`; tentativa de conexão bloqueada em demo |

Ao registrar outra execução, informe sistema, arquitetura, commit ou estado da árvore, comando e resultado. Testes não executados devem ser marcados como pendentes. A fixture de loopback não substitui a matriz de compatibilidade com servidores e métodos de autenticação externos.

### Evidências do editor de chaves — SPEC-002

No mesmo ambiente macOS 26.6.2 arm64, `./scripts/check.sh` passou com **27 testes** (12 de biblioteca, 3 da interface e 12 de serviços), formatação e Clippy limpos. Duas fixtures opt-in ficam ignoradas por padrão. A execução com sockets de loopback autorizados passou; uma tentativa em ambiente restrito foi bloqueada ao abrir o listener.

Os testes de interface usam egui headless para conferir máscara na pintura, ausência de exportação do segredo para clipboard enquanto mascarado e limpeza do histórico de desfazer, inclusive após revelar e limpar. Também validam o rascunho de edição com ID/vínculos corretos e sem leitura do arquivo privado.

A integração SFTP isolada passou separadamente com chave criptografada e rejeição de passphrase incorreta. O arquivo `docs/images/keys-preview.png` é uma captura real da página demo. O `.app` release arm64 foi reconstruído e sua assinatura ad hoc verificada; navegação e formulário lateral foram inspecionados pela interface nativa, sem cadastrar segredos ou alterar vínculos reais. **Keychain real não foi testado.**

## Matriz manual de aceite

Use um servidor descartável sob seu controle. Nunca utilize os endereços de exemplo como servidores de teste reais.

| ID | Cenário | Resultado esperado |
| --- | --- | --- |
| M01 | Iniciar sem dados; criar, editar, buscar e reiniciar um host | Campos permanecem corretos e o JSON não contém senha ou chave privada |
| M02 | Abrir `--demo`, navegar e tentar conectar | Dados fictícios visíveis; rede e persistência bloqueadas |
| M03 | Importar chave própria e gerar Ed25519 | Fingerprint conferível; arquivo privado `0600`; origem preservada |
| M04 | Conectar por SSH a host novo | Prompt de confiança legível; conferir fingerprint antes de aceitar |
| M05 | Conectar com chave e com autenticação interativa | Terminal recebe entrada e saída sem bloquear a janela |
| M06 | Executar programa com ANSI, redimensionar e pressionar Ctrl+C | Cores, tamanho e sinal de interrupção coerentes; registrar incompatibilidades |
| M07 | Abrir duas sessões e fechar uma | Sessão fechada encerra seu processo; a outra continua |
| M08 | Criar SOCKS5 em porta livre | Tráfego do cliente usa o servidor; listener só em loopback |
| M09 | Iniciar túnel em porta ocupada | Erro real aparece na sessão; não exibir conexão ativa falsa |
| M10 | Validar encaminhamento local e remoto | Destino correto e listeners conferidos nos dois lados |
| M11 | SFTP com host desconhecido ou chave alterada | Operação rejeitada antes de autenticar |
| M12 | SFTP após validar o host em SSH | Listagem, upload e download de um arquivo de teste funcionam |
| M13 | Download para arquivo existente | Arquivo original não é sobrescrito |
| M14 | FTP sem e com confirmação de transporte aberto | Primeira operação bloqueada; após confirmação, conexão e transferência funcionam |
| M15 | Credencial de transferência no Keychain | Senha persiste no Keychain; JSON permanece sem o segredo |
| M16 | Host indisponível e credencial inválida | Erro compreensível; interface segue responsiva |
| M17 | JSON inválido ou schema desconhecido | Erro sem substituição silenciosa do workspace |
| M18 | Gerar `.app` e abrir no macOS | Ícone, nome e versão corretos; conexão local ainda funciona |

## Matriz do editor de chaves — SPEC-002

Use somente chaves descartáveis de laboratório. Screenshots do editor devem usar demo ou conteúdo privado vazio.

| ID | Cenário | Resultado esperado |
| --- | --- | --- |
| K01 | Criar identidade colando chave OpenSSH própria | Entrada privada mascarada; chave pública e fingerprint derivados após salvar |
| K02 | Importar chave por arquivo | Origem preservada; arquivo gerenciado `0600`; dados públicos conferíveis |
| K03 | Importar OpenSSH protegido com passphrase correta e gerar Ed25519 com passphrase | Cadastro válido, metadado protegido e conteúdo privado continua criptografado no disco |
| K04 | Passphrase incorreta ao criar ou substituir | Erro sem novo cadastro nem alteração do material/vínculos existentes |
| K05 | Renomear e reiniciar o aplicativo | Nome atualizado, ID e vínculos preservados |
| K06 | Substituir material de identidade compartilhada | Mesmo ID; chave pública/fingerprint atualizados; hosts continuam vinculados |
| K07 | Selecionar vários hosts SSH/SFTP | Todos apontam para a identidade; contagem e seletor refletem a mudança |
| K08 | Selecionar host que já usa outra identidade | Editor indica a substituição antes de salvar; nenhuma troca silenciosa |
| K09 | Desmarcar um host ou remover referência | Só os vínculos pertinentes são removidos; arquivo privado permanece |
| K10 | Tentar associar identidade a FTP | FTP não aparece como destino elegível e regra de domínio recusa associação |
| K11 | Abrir workspace JSON anterior sem `encrypted` | Leitura com padrão `false`, sem perda de hosts ou identidades |
| K12 | Optar por guardar ou atualizar só a passphrase | Chave atual validada; Keychain recebe entrada após salvar metadados; JSON sem segredo; falha de metadados não limpa credencial antiga |
| K13 | Editar nome/vínculos, substituir material sem guardar passphrase e abrir SSH/túnel | Nome/vínculos preservam entrada; substituição limpa credencial antiga depois de salvar; SSH pede interativamente |
| K14 | Navegar pela página demo de chaves | Só metadados fictícios, sem chave privada, Keychain, persistência ou rede |
| K15 | Inspecionar cards, editor, inputs e seletor em janela menor | Raios consistentes, campos legíveis e conteúdo acessível por rolagem |

Os resultados da nova matriz são registrados em [SPEC-002/tasks.md](../specs/002-key-workspace/tasks.md). Resultados anteriores da fundação não confirmam esses fluxos novos.

## Cenário SOCKS5

Depois de iniciar o túnel para um servidor de laboratório, use um endereço que você controla:

```bash
curl --proxy socks5h://127.0.0.1:1080 https://example.com
lsof -nP -iTCP:1080 -sTCP:LISTEN
```

A opção `socks5h` pede resolução DNS pelo proxy. Compare a saída e os registros do seu laboratório. Encerre a sessão e confirme que o listener desapareceu.

## Checklist de interface

Verifique janela inicial e redimensionada, contraste de textos e estados, navegação por teclado, mensagens de validação, listas vazias, operações em curso e erros. Screenshots devem ser capturas da aplicação e identificadas como demonstração quando usarem dados fictícios.

## Limites da alpha

Automação repetível de interface, fuzzing de protocolo, testes prolongados, acessibilidade completa e matriz de servidores e arquiteturas ainda precisam ser ampliados. O cadastro em demonstração foi exercitado pela interface nativa; persistência foi validada nos testes de armazenamento. Keychain real e autenticação interativa externa ainda não foram exercitados. Um build e testes locais bem-sucedidos não provam a interoperabilidade de todos os métodos de autenticação.
