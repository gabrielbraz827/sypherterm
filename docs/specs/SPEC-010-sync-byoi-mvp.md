# SPEC-010 - Sync BYOi MVP

Status: Draft
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

- Escolher provider inicial entre arquivo local e S3 compativel.
