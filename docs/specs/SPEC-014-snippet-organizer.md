# SPEC-014 - Snippet organizer

Status: Implementada
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
| `get_snippet` | `{ id: string }` | `Snippet` | `vault_locked`, `not_found` |
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

type SnippetDraft = {
  id?: string;
  name: string;
  body: string;
  tags?: string[];
  variables?: string[];
};

type SnippetSummary = Omit<Snippet, "body">;

type SnippetFilters = {
  query?: string;
  tag?: string;
};
```

Variaveis usam sintaxe minima `{{variable_name}}`, aceitando letras, numeros, `_` e `-`.

## Seguranca e privacidade

- Snippets ficam criptografados no vault.
- Preview deve evitar expor conteudo sensivel em notificacoes.
- Insercao deve ser diferenciada de execucao automatica.

## Plano de implementacao

1. [x] Definir modelo e validacao.
2. [x] Persistir snippets no vault.
3. [x] Criar busca por nome/tag.
4. [x] Criar painel de snippets.
5. [x] Implementar insercao no terminal ativo.

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

- Decidido: listagem retorna `SnippetSummary` sem `body`; `get_snippet` recupera o corpo apenas sob acao explicita de editar/inserir.
- Decidido: sintaxe minima de variaveis e `{{variable_name}}`.
- Decidido: insercao substitui variaveis via prompt local e nao envia Enter automaticamente.
- Teste manual de insercao depende de sessao SSH ativa.
