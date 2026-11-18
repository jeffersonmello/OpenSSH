# Política de segurança

## Versões

O projeto está em alpha. Somente a linha de desenvolvimento atual recebe correções; não há compromisso de suporte a versões antigas nesta fase.

## Reportar uma vulnerabilidade

Use [Report a vulnerability](https://github.com/jeffersonmello/OpenSSH/security/advisories/new) se o recurso estiver habilitado. Se não estiver disponível, peça um canal privado ao mantenedor pelo [perfil no GitHub](https://github.com/jeffersonmello), sem publicar detalhes de exploração ou credenciais.

Inclua versão/commit, ambiente, condição de disparo, impacto e passos mínimos de reprodução com dados fictícios. Não envie chaves privadas, senhas, tokens ou dados de servidores reais. Não execute testes contra sistemas de terceiros sem autorização.

A análise e o prazo de correção dependem da gravidade e da disponibilidade de manutenção. Combine a divulgação pública depois de avaliar e disponibilizar a correção.

## Limites conhecidos

A alpha não passou por auditoria externa. FTP é transporte sem criptografia; metadados locais não são criptografados; chaves geradas sem passphrase dependem da proteção do sistema e das permissões de arquivo. Builds locais usam assinatura ad hoc e não são notarizadas. Consulte [o modelo de segurança](docs/security.md).
