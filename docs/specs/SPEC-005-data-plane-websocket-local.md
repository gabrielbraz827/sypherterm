# SPEC-005 - Data Plane WebSocket local

Status: Done
Prioridade: P0
Fonte: ARCHITECTURE.md

## Problema

Streaming de terminal via IPC pode travar a UI e criar overhead de serializacao. O Data Plane precisa transportar bytes de SSH diretamente entre backend e `xterm.js` usando WebSocket local.

## Objetivos

- Hospedar servidor WebSocket local em Rust com `tokio-tungstenite`.
- Fazer bind apenas em `127.0.0.1:0`.
- Usar token por sessao com expiracao curta.
- Suportar frames binarios para input/output do terminal.

## Nao-objetivos

- Usar `tauri-plugin-websocket` como servidor.
- Suportar acesso remoto ao Data Plane.

## Contratos

### Handshake

```ts
type ConnectSshResponse = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};

type DataPlaneSession = ConnectSshResponse & {
  expiresAt: string;
};
```

### Eventos WebSocket

| Evento | Direcao | Payload | Observacao |
| --- | --- | --- | --- |
| `auth` | Frontend -> Backend | `{ token: string }` | Primeiro frame logico |
| `input` | Frontend -> Backend | bytes | Frame binario preferencial |
| `resize` | Frontend -> Backend | `{ cols: number; rows: number }` | Frame texto JSON |
| `output` | Backend -> Frontend | bytes | Frame binario |
| `status` | Backend -> Frontend | `{ state: string }` | Frame texto JSON |
| `error` | Backend -> Frontend | `CommandError` | Sem segredos |
| `heartbeat` | ambos | `{ ts: string }` | Liveness |

## Seguranca e privacidade

- Bind deve ser somente `127.0.0.1`, nunca `0.0.0.0`.
- Token deve ser unico, aleatorio, curto e invalidado apos uso ou timeout.
- Logs nao devem imprimir bytes de terminal.

## Plano de implementacao

1. Adicionar `tokio`, `tokio-tungstenite`, `futures-util` e `uuid`.
2. Criar `ws::StreamServer`.
3. Iniciar servidor no bootstrap do app ou sob demanda.
4. Registrar sessoes aguardando conexao do frontend.
5. Implementar autenticacao do primeiro frame.
6. Implementar heartbeat e cleanup de conexoes mortas.

## Criterios de aceite

- Servidor escuta em porta efemera local.
- Frontend conecta usando `wsUrl` retornada por IPC.
- Conexao sem token valido e rejeitada.
- Saida intensa usa frames binarios sem passar por IPC.

## Plano de testes

- Unit test para geracao e expiracao de token.
- Integration test local conectando WebSocket e validando auth.
- Teste manual com terminal enviando e recebendo bytes.

## Decisoes tomadas

- `StreamServer` inicia no bootstrap do Tauri e fica gerenciado como `tauri::State`.
- O servidor faz bind somente em `127.0.0.1:0`; o sistema operacional escolhe a porta efemera.
- `AppState.dataPlane` e atualizado para `starting` durante inicializacao e `running` apos o bind.
- A SPEC-005 adicionou o comando `open_data_plane_session` para validar a infraestrutura antes da SPEC-006.
- `connect_ssh` continua reservado para a SPEC-006 e nao cria sessao SSH falsa.
- O primeiro frame de autenticacao aceita `{ "token": "..." }` ou `{ "event": "auth", "token": "..." }`.
- Tokens sao aleatorios, possuem 32 bytes codificados em hex, expiram em 60 segundos e sao removidos no primeiro uso.
- Conexao sem token valido recebe frame `error` e e encerrada.
- Enquanto SSH real nao existe, frames binarios autenticados sao ecoados apenas para validar o caminho binario do Data Plane.
- Heartbeat responde com `{ "event": "heartbeat", "ts": "..." }`.

## Riscos e decisoes abertas

- Definir limite de buffer e estrategia de backpressure para saidas muito grandes.
- Definir politica de backpressure e tamanho maximo de frame quando a SPEC-006 ligar SSH real ao Data Plane.
