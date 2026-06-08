# SPEC-010 - Sync BYOi MVP

Status: Done
Prioridade: P1
Fonte: README.md, ARCHITECTURE.md

## Problema

O diferencial do SypherTerm e sincronizar dados usando infraestrutura do usuario, sem servidores proprietarios e sem expor dados descriptografados ao provider.

## Objetivos

- Implementar interface de sync provider.
- Comecar por provider local ou S3 compativel.
- Enviar somente blob criptografado.
- Tratar conflitos sem sobrescrever silenciosamente.

## Nao-objetivos

- Suportar todos os providers listados no README no MVP.
- Resolver merge sem interacao para todos os conflitos.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `test_sync_provider` | `SyncProviderConfig` | `SyncProviderStatus` | `auth_failed`, `network_error` |
| `trigger_cloud_sync` | `SyncRequest` | `SyncJobStatus` | `vault_locked`, `conflict_detected`, `sync_failed` |
| `list_sync_versions` | `SyncProviderConfig` | `SyncVersion[]` | `sync_failed` |

### Modelos

```ts
type SyncRequest = {
  providerId: string;
  direction: "push" | "pull" | "bidirectional";
  deviceId?: string;
};

type SyncProviderConfig = {
  providerId: string;
  kind: "local";
  localPath: string;
  deviceId?: string;
};

type SyncVersion = {
  versionId: string;
  deviceId: string;
  payloadHash: string;
  createdAt: string;
};
```

## Seguranca e privacidade

- Provider recebe somente `VaultEnvelope`.
- Tokens de provider devem ficar no vault ou em armazenamento seguro.
- Logs mostram status e hash, nunca conteudo.

## Plano de implementacao

1. Definir trait `SyncProvider`.
2. Implementar provider local para desenvolvimento.
3. Implementar provider S3 compativel quando credenciais estiverem modeladas.
4. Criar politica de conflito por hash, timestamp e device id.
5. Integrar notificacoes de sucesso/falha.

## Implementacao entregue

- `SyncProvider` foi definido no backend com provider local inicial.
- `test_sync_provider`, `trigger_cloud_sync` e `list_sync_versions` foram expostos via Tauri.
- O provider local grava `sypherterm-sync-index.json` e arquivos `vault-{versionId}.json` na pasta escolhida.
- O payload sincronizado e sempre o `VaultEnvelope` criptografado; hashes e metadados aparecem na UI, mas o conteudo nao.
- O fluxo de UI permite configurar pasta local, testar escrita, executar push/pull/bidirecional e listar versoes.

## Criterios de aceite

- Push envia blob criptografado.
- Pull em instalacao limpa restaura apos senha correta.
- Conflito retorna estado explicito.
- Sync falho nao corrompe vault local.

## Plano de testes

- Unit tests para deteccao de conflito.
- Integration test com provider local.
- Teste manual de restore.

## Riscos e decisoes abertas

- Decisao: o provider inicial e arquivo local. S3 compativel fica adiado ate o schema de credenciais/tokens do provider estar modelado no vault ou em secure storage.
- Decisao: no MVP, `providerId` em `trigger_cloud_sync` e tratado como o caminho local do provider. Uma registry de providers pode substituir isso quando houver multiplos providers.
- Decisao: conflitos sao detectados por hash da ultima versao remota e `deviceId`; o sync retorna `conflict_detected` e nao sobrescreve o vault local.
- Risco: restore completo em uma instalacao limpa ainda precisa validacao manual em app empacotado.
