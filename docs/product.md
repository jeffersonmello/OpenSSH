# Produto e escopo

## Objetivo

Permitir que desenvolvedores e administradores organizem o acesso a ambientes remotos no macOS por meio de um workspace local, com terminal, identidades, roteamento e arquivos em uma interface nativa.

## Pessoas e necessidades

| Pessoa | Necessidade | Resultado esperado |
| --- | --- | --- |
| Desenvolvedor | Acessar APIs, bancos e ambientes de teste | Encontrar um host e abrir uma sessão com poucos passos |
| Administrador | Manter conexões e identidades separadas por contexto | Grupos, tags e fingerprint identificáveis |
| Operador de infraestrutura | Alcançar serviços de uma rede privada | SOCKS5 e encaminhamento de portas explícitos |
| Colaborador open source | Entender e alterar o produto | Especificações, documentação e builds reproduzíveis |

## Princípios

- Dados locais por padrão, sem conta obrigatória.
- Trabalho de rede fora da renderização da interface.
- Confiança no servidor nunca é desativada para facilitar conexões.
- Estado visual deve refletir o estado real; exemplos não simulam servidores conectados.
- Compatibilidade e desempenho são medidos antes de promessas quantitativas.
- Recursos futuros são documentados como planejados.

## Fluxos principais

1. **Host:** criar endereço e credenciais de referência, organizar, buscar e conectar.
2. **Identidade:** importar uma chave própria ou gerar Ed25519, conferir fingerprint e associar a um host.
3. **Túnel:** escolher host e tipo, informar portas, iniciar e encerrar a sessão.
4. **Arquivo:** selecionar protocolo, autenticar, listar e transferir um arquivo.
5. **Demonstração:** explorar o layout com dados fictícios sem persistência ou rede.

## Fora do escopo da alpha

Servidor SSH, armazenamento multiusuário, sincronização, marketplace, acesso compartilhado, auto-update, assinatura Developer ID, notarização, terminal com paridade completa, FTPS, proxies encadeados e acesso por hardware de segurança.

O projeto não implementa algoritmos criptográficos próprios. Aproveita o cliente SSH do sistema para terminais e túneis e bibliotecas especializadas para transferências.
