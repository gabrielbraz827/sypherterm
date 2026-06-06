# SPEC-014 - Snippet organizer

Status: Draft
Prioridade: P2
Fonte: README.md

## Problema

Usuarios repetem comandos, scripts curtos e blocos de configuracao. Snippets aumentam produtividade, mas podem conter segredos e devem ser tratados como dados sensiveis.

## Objetivos

- Criar, editar, buscar e remover snippets.
- Suportar tags e variaveis.
- Inserir snippet no terminal com confirmacao quando necessario.
- Armazenar snippets no vault criptografado.

## Nao-objetivos

- Executar snippets automaticamente sem acao explicita.
- Criar linguagem de template complexa.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `list_snippets` | filtros opcionais | `SnippetSummary[]` | `vault_locked` |
| `save_snippet` | `SnippetDraft` | `Snippet` | `vault_locked`, `validation_error` |
| `delete_snippet` | `{ id: string }` | `{ deleted: boolean }` | `not_found` |

### Modelos

```ts
type Snippet = {
  id: string;
  version: 1;
  name: string;
  body: string;
  tags: string[];
  variables: string[];
  createdAt: string;
  updatedAt: string;
};
```

## Seguranca e privacidade

- Snippets ficam criptografados no vault.
- Preview deve evitar expor conteudo sensivel em notificacoes.
- Insercao deve ser diferenciada de execucao automatica.

## Plano de implementacao

1. Definir modelo e validacao.
2. Persistir snippets no vault.
3. Criar busca por nome/tag.
4. Criar painel de snippets.
5. Implementar insercao no terminal ativo.

## Criterios de aceite

- Snippet e salvo e recuperado apos desbloquear vault.
- Busca filtra por nome e tags.
- Inserir snippet nao pressiona Enter automaticamente por padrao.
- Vault bloqueado impede listagem.

## Plano de testes

- Unit tests de validacao.
- Unit tests de variaveis.
- Teste manual de insercao no terminal.

## Riscos e decisoes abertas

- Definir sintaxe minima de variaveis.
