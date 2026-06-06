# SPEC-008 - Gerenciamento de perfis

Status: Draft
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

## Experiencia do usuario

O usuario consegue criar um perfil pela sidebar, editar dados basicos, conectar com um clique e localizar perfis por busca, tag ou grupo.

## Seguranca e privacidade

- A lista de perfis nao deve expor senha ou chave.
- Campos sensiveis devem apontar para o vault, nao para texto puro.

## Plano de implementacao

1. Criar formulario de perfil.
2. Implementar lista, busca e filtros.
3. Implementar grupo e tags.
4. Integrar acao `connect`.
5. Registrar ultimo uso e ordenar recentes.

## Criterios de aceite

- CRUD funciona sem reload.
- Busca encontra nome, host e tags.
- Perfil duplicado nao reutiliza `id`.
- Deletar perfil com sessao ativa exige tratamento explicito.

## Plano de testes

- Unit tests de validacao.
- Teste manual de CRUD.
- Teste manual de busca e recentes.

## Riscos e decisoes abertas

- Decidir se host e username sao considerados sensiveis o bastante para criptografia obrigatoria.
