# SPEC-002 — Editor de chaves e vínculos de conexão

- Estado: implementada na alpha, validada localmente
- Criada em: 2026-11-18
- Produto: OpenSSH Manager
- Evolui: [SPEC-001](../001-native-workspace/spec.md), RF01 e RF02

## Problema

A lista inicial de identidades não oferece um editor completo nem deixa explícita a relação entre cada chave e suas conexões. O usuário quer um fluxo visual semelhante ao de gerenciadores como Termius: cartões de chaves, editor lateral, associação às conexões e uma interface mais arredondada.

As capturas são referência de interação. Dados, conteúdo privado e identidade visual de terceiros não são reproduzidos.

## Escopo

Grade de identidades, editor lateral, importação por arquivo ou texto colado, geração Ed25519 com passphrase opcional, metadados públicos derivados, vínculos compartilhados entre uma identidade e vários hosts SSH/SFTP, seleção de chave no editor de host e tratamento visual mais arredondado.

## Requisitos e aceite

| ID | Requisito | Resultado observável |
| --- | --- | --- |
| RF01 | Apresentar grade e editor | Selecionar cartão abre editor lateral com nome, algoritmo, fingerprint e estado de proteção |
| RF02 | Importar por arquivo ou texto | Um arquivo próprio ou uma chave OpenSSH colada e validada gera uma identidade; texto privado fica mascarado por padrão |
| RF03 | Validar e aplicar passphrase | Chave OpenSSH protegida exige passphrase correta; geração Ed25519 aceita passphrase opcional; erro não altera identidade, arquivo ou vínculos existentes |
| RF04 | Derivar chave pública | Chave pública e fingerprint vêm do material privado validado e são campos de leitura |
| RF05 | Compartilhar uma identidade | Selecionar vários hosts SSH/SFTP vincula todos à mesma identidade; cada host usa no máximo uma identidade |
| RF06 | Tornar troca de vínculo explícita | Hosts que já usam outra identidade indicam a substituição antes de salvar; desmarcar desfaz apenas o vínculo desta identidade |
| RF07 | Editar sem perder referências | Renomear ou substituir o material mantém `Identity.id`; conexões vinculadas continuam apontando para a identidade |
| RF08 | Selecionar chave na conexão | Editor de host mostra nome, algoritmo e fingerprint da identidade e permite abrir seu gerenciamento |
| RF09 | Manter FTP separado | Host FTP não pode receber identidade SSH pelo editor ou pelas regras de vínculo |
| RF10 | Guardar passphrase com opção explícita | Opção selecionada guarda passphrase no Keychain por identidade após salvar metadados; validação da chave atual permite atualizar só a passphrase; nome/vínculos preservam a entrada; conteúdo privado permanece em arquivo `0600` |
| RF11 | Refinar a interface | Cards, botões, inputs e painéis têm cantos arredondados consistentes, sem corte de conteúdo ou perda de foco |
| RF12 | Demonstrar sem segredos | Página de chaves em demo usa metadados fictícios, sem arquivo privado, Keychain ou conexão de rede |

## Requisitos não funcionais

| ID | Requisito | Evidência esperada |
| --- | --- | --- |
| RNF01 | Segredos fora do JSON, argv e capturas | Testes de serialização/processos, máscara real de paint/clipboard e undo limpo; demo sem material privado |
| RNF02 | Arquivo privado protegido | Permissão `0600`; material importado protegido permanece criptografado |
| RNF03 | Compatibilidade e integridade | Workspaces anteriores abrem; erro de validação preserva dados existentes |
| RNF04 | Interface consistente | Campos e ações continuam acessíveis em janela menor e sem perda de foco |

## Dados e compatibilidade

`Host.identity_id` continua como referência canônica. `Identity.id` permanece estável durante edição; não é substituído por um novo ID ao renomear ou trocar material.

`Identity.encrypted` é um metadado opcional com padrão `false` na desserialização. O schema continua em `1`; workspaces anteriores abrem sem migração destrutiva. Senha, passphrase e chave privada não pertencem ao JSON.

Uma identidade compartilhada representa um único material privado: sua substituição afeta autenticações futuras de todos os hosts vinculados. Remover a referência no catálogo preserva o arquivo privado conforme o fluxo existente.

## Limites de autenticação

A passphrase salva por identidade é destinada ao adaptador SFTP. Terminal SSH e túneis continuam solicitando a passphrase interativamente ao cliente SSH do sistema; não recebem segredo por argumento ou injeção no PTY.

Certificados, Touch ID, FIDO2, exposição automática da chave privada já salva e sincronização permanecem fora do escopo.

## Validação

Testes direcionados para importação protegida, passphrase incorreta, preservação do conteúdo criptografado, permissões, ID estável, vínculos, FTP e compatibilidade de schema. Inspeção visual do editor e do seletor de identidade, incluindo captura real de demo sem segredos. A matriz está em [docs/testing.md](../../docs/testing.md); evidências ficam em [tasks.md](tasks.md).
