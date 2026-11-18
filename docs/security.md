# Modelo de segurança

## Ativos e limites de confiança

Os ativos principais são chaves privadas, senhas, metadados de servidores, conteúdo de arquivos e tráfego encaminhado. O aplicativo confia no usuário local, no sistema operacional, no cliente SSH instalado e nas bibliotecas do build.

A proteção local usa permissões de arquivo e Keychain. Esta alpha não pretende resistir a malware executado como o mesmo usuário, a um sistema comprometido, a leitura de memória por processos privilegiados ou a servidores já autorizados que ajam maliciosamente.

## Controles e riscos residuais

| Ameaça | Controle inicial | Limitação |
| --- | --- | --- |
| Servidor impostor | SSH com verificação normal; SFTP consulta `known_hosts` antes da autenticação | O usuário precisa conferir o fingerprint por canal confiável na primeira conexão |
| Injeção em processos | Argumentos separados e validação de host/porta/usuário | O cliente SSH e seus parsers também fazem parte da superfície de ataque |
| Exposição de credenciais em JSON | JSON contém referências; senhas no Keychain | Metadados do workspace não são criptografados |
| Leitura indevida de chave local | Diretórios restritos, chave privada `0600` | Chaves geradas nesta alpha não têm passphrase; usuário local comprometido pode lê-las |
| Listener de proxy exposto | Bind explícito em `127.0.0.1` | Outros processos do mesmo computador podem acessar portas locais |
| FTP em rede hostil | Confirmação explícita e SFTP como alternativa | FTP comum não tem confidencialidade ou integridade do transporte |
| Sobrescrita local por download | Criação exclusiva do arquivo de destino | Falhas tentam remover o destino local parcial; interrupção abrupta do processo ainda pode deixar resíduo |
| Sobrescrita remota por upload | SFTP cria destino exclusivo; FTP verifica existência antes de enviar | FTP não oferece criação exclusiva portável; há janela de corrida com outros clientes |
| Workspace corrompido | Gravação atômica e rejeição de schema inválido | Backup e recuperação continuam sob responsabilidade do usuário |
| Vazamento em screenshot/log | Nenhum material de produção nos exemplos; não registrar segredos | O terminal remoto pode exibir conteúdo sensível por comandos do usuário |

## Chaves e senhas

- Importação copia a chave para o diretório gerenciado; não substitui a origem. Nesta alpha, arquivos protegidos por passphrase são rejeitados. Para usá-los, carregue a chave no `ssh-agent` e deixe o host sem identidade gerenciada selecionada.
- Chaves privadas não entram em `Workspace`, histórico de atividade ou repositório.
- O gerador usa Ed25519 e o `ssh-keygen` do sistema. Uma chave gerada sem passphrase depende da proteção de arquivo e do disco.
- O Keychain armazena senhas de transferência por host. O terminal SSH usa seus próprios prompts e `ssh-agent`.
- Remover uma identidade do catálogo preserva seu arquivo privado. Remover um host preserva a entrada de senha no Keychain; apague a senha pelo editor antes de excluir o host quando desejar removê-la também.
- Não se deve passar senha ou passphrase pela linha de comando nem por mensagens de erro.

## Confiança no servidor

Não use `StrictHostKeyChecking=no`, descarte de `known_hosts` ou aceitação automática de mudança de chave para resolver problemas de conexão. Alterações de fingerprint exigem investigação por canal confiável. O aplicativo não deve remover entradas de confiança automaticamente.

Se o SFTP rejeitar um host desconhecido, conecte primeiro pelo terminal SSH e confira o fingerprint. Se rejeitar uma mudança de chave, interrompa o acesso até confirmar a origem da mudança.

## Túneis

SOCKS5 encaminha tráfego das aplicações configuradas para usá-lo; não transforma automaticamente o computador inteiro em uma VPN. Para evitar resolução DNS local, configure o aplicativo cliente para resolução pelo proxy, quando disponível.

Encaminhamento remoto solicita listener de loopback no servidor; a configuração `GatewayPorts` do servidor pode alterar a exposição efetiva. Confirme a política e os listeners no servidor antes de usar redes não confiáveis.

## Dependências e distribuição

`Cargo.lock` é versionado. Atualizações entram por revisão, com CI e leitura das mudanças relevantes. A alpha não passou por auditoria criptográfica ou revisão externa completa. Pacotes gerados pelo script usam assinatura ad hoc e não são notarizados.

Para relatar vulnerabilidade, veja [SECURITY.md](../SECURITY.md). Não abra uma issue pública contendo credenciais, material de chave ou dados reais de acesso.
