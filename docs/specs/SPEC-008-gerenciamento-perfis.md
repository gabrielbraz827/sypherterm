# SPEC-008 - Gerenciamento de perfis

Status: Done
Prioridade: P1
Fonte: README.md

## Problema

Usuarios precisam criar, organizar e reutilizar conexoes SSH sem redigitar hosts, usuarios e opcoes. O gerenciamento de perfis deve ser rapido e seguro.

## Objetivos

- CRUD completo de perfis.
- Organizar por grupos, tags e recentes.
- Validar campos antes de persistir.
- Integrar credenciais via `credentialRef`.

## Nao-objetivos

- Sync remoto de perfis.
- Editor avancado de chaves.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `list_profiles` | filtros opcionais | `ConnectionProfileSummary[]` | `store_unavailable` |
| `save_profile` | `ConnectionProfileDraft` | `ConnectionProfile` | `validation_error` |
| `delete_profile` | `{ id: string }` | `{ deleted: boolean }` | `not_found` |
| `duplicate_profile` | `{ id: string }` | `ConnectionProfile` | `not_found` |

### Modelos

```ts
type ProfileListFilters = {
  query?: string;
  groupId?: string;
  tag?: string;
  recentFirst?: boolean;
};

type ConnectionProfileSummary = {
  id: string;
  version: 1;
  name: string;
  host: string;
  port: number;
  username?: string;
  groupId?: string;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string;
  hasCredential: boolean;
};
```

## Experiencia do usuario

O usuario consegue criar um perfil pela sidebar, editar dados basicos, conectar com um clique e localizar perfis por busca, tag ou grupo.

## Seguranca e privacidade

- A lista de perfis nao deve expor senha ou chave.
- Campos sensiveis devem apontar para o vault, nao para texto puro.
- `credentialRef` continua fora de `ConnectionProfileSummary`; editar um perfil sem novo valor preserva a referencia existente.

## Plano de implementacao

1. Criar formulario de perfil.
2. Implementar lista, busca e filtros.
3. Implementar grupo e tags.
4. Integrar acao `connect`.
5. Registrar ultimo uso e ordenar recentes.

## Decisoes de implementacao

- `lastUsedAt` foi adicionado como campo opcional e compativel com perfis existentes.
- `connect_ssh` marca `lastUsedAt` quando recebe `profileId`, sem falhar a sessao SSH se a atualizacao local do perfil nao puder ser gravada.
- `list_profiles` aceita filtros opcionais no backend; a UI tambem filtra localmente a lista ja carregada para resposta imediata.
- `duplicate_profile` cria novo `id`, limpa `lastUsedAt` e gera nome sem colisao no formato `nome copy`, `nome copy 2`, etc.
- Deletar o perfil selecionado com sessao ativa exige confirmacao e desconecta a sessao antes de remover.

## Criterios de aceite

- CRUD funciona sem reload.
- Busca encontra nome, host e tags.
- Perfil duplicado nao reutiliza `id`.
- Deletar perfil com sessao ativa exige tratamento explicito.

## Plano de testes

- Unit tests de validacao.
- Teste manual de CRUD.
- Teste manual de busca e recentes.

Verificacoes executadas:

- `corepack pnpm check`
- `corepack pnpm build`
- `cargo fmt --check`
- `cargo check`
- `cargo test`

## Riscos e decisoes abertas

- Decidir se host e username sao considerados sensiveis o bastante para criptografia obrigatoria.
- Teste manual de CRUD/busca/recentes ainda deve ser feito contra a UI em execucao.
