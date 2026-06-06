# SPEC-004 - Control Plane IPC

Status: Done
Prioridade: P0
Fonte: ARCHITECTURE.md

## Problema

Operacoes de baixa frequencia devem passar por comandos Tauri previsiveis, enquanto bytes de terminal ficam fora do IPC. Sem uma especificacao do Control Plane, frontend e backend tendem a acoplar detalhes e vazar responsabilidades.

## Objetivos

- Definir comandos Tauri para configuracao, perfis, vault, sessoes e sync.
- Padronizar erros retornados ao frontend.
- Garantir que IPC nao transporte stream de terminal.

## Nao-objetivos

- Definir protocolo binario do Data Plane.
- Implementar UX final de todos os comandos.

## Contratos

### Envelope de erro

```ts
type CommandError = {
  code: string;
  message: string;
  recoverable: boolean;
};
```

### Comandos Tauri

| Comando | Entrada | Saida | Spec dona |
| --- | --- | --- | --- |
| `get_app_status` | vazio | `AppStatus` | SPEC-001 |
| `create_vault` | `CreateVaultRequest` | `VaultStatus` | SPEC-003 |
| `unlock_vault` | `UnlockVaultRequest` | `VaultStatus` | SPEC-003 |
| `list_profiles` | vazio | `ConnectionProfileSummary[]` | SPEC-002 |
| `save_profile` | `ConnectionProfileDraft` | `ConnectionProfile` | SPEC-002 |
| `connect_ssh` | `ConnectSshRequest` | `ConnectSshResponse` | SPEC-006 |
| `disconnect_session` | `{ sessionId: string }` | `SessionStatus` | SPEC-006 |
| `resize_session` | `SessionResizeRequest` | `SessionStatus` | SPEC-006 |
| `trigger_cloud_sync` | `SyncRequest` | `SyncJobStatus` | SPEC-010 |
| `start_tunnel` | `TunnelRequest` | `TunnelStatus` | SPEC-013 |
| `stop_tunnel` | `{ tunnelId: string }` | `TunnelStatus` | SPEC-013 |
| `list_tunnels` | vazio | `TunnelStatus[]` | SPEC-013 |
| `list_session_tunnels` | `{ sessionId: string }` | `TunnelStatus[]` | SPEC-013 |

## Seguranca e privacidade

- Comandos que recebem segredo devem redigir logs.
- Erros devem ser uteis, mas nao podem incluir host interno sensivel, senha, chave ou payload descriptografado.
- Comandos de tunnel devem tratar `bindHost`, `bindPort`, `targetHost` e `targetPort` como configuracao operacional sensivel em logs compartilhaveis.

## Plano de implementacao

1. Criar modulo `commands` dividido por dominio ou funcoes simples.
2. Criar tipo de erro Rust serializavel.
3. Registrar todos os comandos no `invoke_handler`.
4. Criar wrapper TypeScript `src/lib/api.ts` para `invoke`.
5. Mapear erros Rust para UI sem `throw` cru.

## Criterios de aceite

- Todo comando documentado possui wrapper TypeScript.
- Nenhum stream de terminal passa por IPC.
- Erros sao serializados com `code`, `message` e `recoverable`.
- Comandos de tunnel retornam status imediato e nao bloqueiam o IPC durante trafego encaminhado.

## Plano de testes

- Unit tests para conversao de erros.
- Testes de wrapper TypeScript com mocks quando viavel.
- Smoke test manual chamando `get_app_status`.

## Decisoes tomadas

- A borda IPC usa `CommandError` como envelope unico para erros serializados ao frontend.
- `VaultError`, `StorageError` e `AppStateError` sao convertidos para `CommandError` em `src-tauri/src/commands.rs`.
- `src/lib/api.ts` centraliza chamadas em `invokeCommand`, normalizando erros desconhecidos para `{ code, message, recoverable }`.
- Os comandos de SSH, sync e tunnel foram registrados como stubs explicitos do Control Plane.
- Stubs de SSH e sync retornam `not_implemented`; stubs de tunnel retornam `unsupported_mode`, `not_found` ou lista vazia conforme o comando.
- Nenhum stub transporta bytes de terminal, trafego de tunnel ou payload de sync por IPC.
- Os comandos permanecem em `src-tauri/src/commands.rs` por enquanto; a divisao por dominio fica para quando SPEC-006, SPEC-010 e SPEC-013 forem implementadas.

## Riscos e decisoes abertas

- Separar `commands.rs` por dominio quando os comandos de SSH, sync e tunnel deixarem de ser stubs.
