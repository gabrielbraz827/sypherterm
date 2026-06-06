# SPEC-002 - Modelo de dominio e armazenamento local

Status: Done
Prioridade: P0
Fonte: README.md, ARCHITECTURE.md

## Problema

SypherTerm precisa representar perfis SSH, preferencias, snippets e layout local de forma consistente antes de criptografar, sincronizar ou renderizar esses dados.

## Objetivos

- Definir modelos versionados para dados locais.
- Separar metadata nao sensivel de segredos.
- Usar `tauri-plugin-store` para persistencia local inicial.
- Preparar os modelos para criptografia e sync.

## Nao-objetivos

- Criptografar dados; isso pertence a SPEC-003.
- Implementar provedores de sync; isso pertence a SPEC-010.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `list_profiles` | vazio | `ConnectionProfileSummary[]` | `store_unavailable` |
| `save_profile` | `ConnectionProfileDraft` | `ConnectionProfile` | `validation_error`, `store_unavailable` |
| `delete_profile` | `{ id: string }` | `{ deleted: boolean }` | `not_found`, `store_unavailable` |
| `get_preferences` | vazio | `UserPreferences` | `store_unavailable` |
| `save_preferences` | `UserPreferences` | `UserPreferences` | `validation_error`, `store_unavailable` |

### Modelos persistidos

```ts
type ConnectionProfile = {
  id: string;
  version: 1;
  name: string;
  host: string;
  port: number;
  username?: string;
  groupId?: string;
  tags: string[];
  credentialRef?: string;
  createdAt: string;
  updatedAt: string;
};

type ConnectionProfileDraft = {
  id?: string;
  name: string;
  host: string;
  port: number;
  username?: string;
  groupId?: string;
  tags?: string[];
  credentialRef?: string;
};

type ConnectionProfileSummary = Omit<ConnectionProfile, "credentialRef"> & {
  hasCredential: boolean;
};

type UserPreferences = {
  version: 1;
  theme: "system" | "dark" | "light";
  fontFamily: string;
  fontSize: number;
  cursorStyle: "block" | "bar" | "underline";
};
```

## Seguranca e privacidade

- `ConnectionProfileSummary` nao deve expor `credentialRef` sensivel nem segredo.
- Chaves privadas, senhas e passphrases nao entram em store nao criptografado.

## Plano de implementacao

1. Criar tipos Rust serializaveis para modelos.
2. Espelhar tipos TypeScript no frontend.
3. Implementar validacao de host, porta, nome e tags.
4. Implementar camada de repository sobre `tauri-plugin-store`.
5. Adicionar migracao por campo `version`.

## Criterios de aceite

- Perfil valido pode ser criado, listado, atualizado e removido.
- Porta fora de `1..=65535` retorna erro de validacao.
- Store corrompido retorna erro controlado.
- Preferencias possuem defaults quando ausentes.

## Plano de testes

- Unit tests de validacao dos modelos.
- Unit tests de serializacao/deserializacao.
- Teste manual de CRUD via UI minima ou comando.

## Decisoes tomadas

- A persistencia local inicial usa o arquivo `sypherterm.local.json` do `tauri-plugin-store`.
- Perfis ficam na chave `profiles` como array versionado; preferencias ficam na chave `preferences`.
- `ConnectionProfileDraft.port` aceita numero inteiro amplo no backend e valida explicitamente `1..=65535` para retornar `validation_error`.
- Timestamps `createdAt` e `updatedAt` foram armazenados como strings de segundos Unix para evitar uma dependencia de data apenas nesta fundacao.
- Tags sao normalizadas com trim, deduplicadas e ordenadas.
- `ConnectionProfileSummary` remove `credentialRef` e expoe apenas `hasCredential`.
- A UI minima da tela inicial permite criar, listar e remover perfis locais enquanto as specs de UX completas ainda nao existem.

## Riscos e decisoes abertas

- Definir quando metadata de perfil tambem deve entrar no vault criptografado.
- Avaliar migrar timestamps para RFC3339 quando a camada de sync precisar comparar versoes entre dispositivos.
