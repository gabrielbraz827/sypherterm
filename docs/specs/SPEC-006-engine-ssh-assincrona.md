# SPEC-006 - Engine SSH assincrona

Status: Done
Prioridade: P0
Fonte: README.md, ARCHITECTURE.md

## Problema

O nucleo do produto depende de sessoes SSH confiaveis, assincronas e integradas ao Data Plane. A engine deve gerenciar autenticacao, canais, resize, encerramento e erros recuperaveis.

## Objetivos

- Implementar conexao SSH assincrona.
- Suportar autenticacao por senha e chave privada.
- Expor ciclo de vida de sessao ao Control Plane.
- Conectar stdin/stdout/stderr ao Data Plane.
- Expor primitivas de forwarding para tunnels locais, remotos e dynamic conforme suporte da crate escolhida.

## Nao-objetivos

- SFTP e UX completa de port forwarding; serao tratados em specs proprias.
- Multiplexacao visual de panes.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `connect_ssh` | `ConnectSshRequest` | `ConnectSshResponse` | `auth_failed`, `network_error`, `host_unreachable`, `vault_locked` |
| `disconnect_session` | `{ sessionId: string }` | `SessionStatus` | `not_found` |
| `resize_session` | `SessionResizeRequest` | `SessionStatus` | `not_found`, `invalid_size` |

### Modelos

```ts
type ConnectSshRequest = {
  profileId?: string;
  host?: string;
  port?: number;
  username?: string;
  credentialRef?: string;
  password?: string;
  privateKeyPath?: string;
  passphrase?: string;
  cols: number;
  rows: number;
};

type SessionStatus = {
  sessionId: string;
  state: "connecting" | "connected" | "closing" | "closed" | "failed";
};
```

## Seguranca e privacidade

- Senhas e chaves devem vir do vault desbloqueado quando possivel.
- Erros de autenticacao nao podem vazar detalhes da credencial.
- Host key verification deve ser especificada antes do MVP publico.
- Decisao MVP: `password`, `privateKeyPath` e `passphrase` podem ser enviados inline ao comando para permitir conexao real antes do schema de credenciais do vault. Esses campos nao sao persistidos.
- Decisao MVP: host keys sao aceitas pela engine durante desenvolvimento. Antes do MVP publico, substituir por known_hosts/pinning com UX explicita.

## Plano de implementacao

1. Comparar e escolher crate SSH async.
2. Implementar `SshSession` e registry de sessoes.
3. Implementar autenticacao por senha.
4. Implementar autenticacao por chave privada.
5. Abrir canal shell com PTY.
6. Ligar canal shell ao Data Plane.
7. Implementar resize e disconnect.
8. Validar se a crate escolhida oferece APIs para `direct-tcpip`, `forwarded-tcpip` e dynamic/SOCKS, registrando limitacoes na SPEC-013.

## Decisoes de implementacao

- Crate escolhida: `russh` 0.61.
- Features: `default-features = false`, `features = ["ring", "rsa"]`. Isso evita `aws-lc-rs`/NASM no Windows e mantem suporte a chaves RSA.
- O comando `connect_ssh` abre a conexao SSH, autentica, cria canal shell com PTY e registra uma sessao do Data Plane com `sessionId`, `wsUrl` e `authToken`.
- O Data Plane encaminha binarios/texto de input para o canal SSH e publica `stdout`/`stderr` como frames binarios.
- `resize_session` e evento WebSocket `resize` usam `window_change`.
- `disconnect_session` envia controle de encerramento, fecha canal, desconecta SSH e remove a sessao do registry.
- Suporte futuro a tunnel: `russh` oferece abertura de canal `direct-tcpip` e primitivas de forward TCP no client, suficientes para local/remote forwarding; SOCKS dynamic sera implementado sobre local listener + `direct-tcpip` na SPEC-013.

## Criterios de aceite

- Conecta em host SSH valido e abre shell interativo.
- Falha de autenticacao retorna erro controlado.
- Resize reflete no terminal remoto.
- Fechar sessao libera recursos.
- A decisao da crate SSH documenta explicitamente o suporte necessario para tunnels.

## Plano de testes

- Unit tests para validacao de request.
- Integration test opcional com servidor SSH local/container.
- Teste manual em host de desenvolvimento.

Verificacoes executadas:

- `cargo fmt --check`
- `cargo check`
- `cargo test`
- `corepack pnpm check`
- `corepack pnpm build`

## Riscos e decisoes abertas

- Host key verification ainda precisa de UX e persistencia antes do MVP publico.
- Credenciais vindas do vault dependem do schema final de secrets; por enquanto o comando aceita credenciais inline nao persistidas.
- Integration test com servidor SSH local/container ainda e opcional e nao foi automatizado nesta spec.
