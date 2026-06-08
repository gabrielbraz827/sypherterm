# SPEC-013 - Port forwarding

Status: Done
Prioridade: P1
Fonte: README.md

## Problema

Desenvolvedores usam SSH para tunnels locais, remotos e SOCKS/dynamic. SypherTerm deve coordenar esses tunnels com visibilidade e encerramento seguro.

## Objetivos

- Suportar local forwarding.
- Suportar remote forwarding quando a engine SSH permitir.
- Suportar dynamic/SOCKS como meta posterior.
- Mostrar status, portas e erros.
- Permitir iniciar tunnels a partir de uma sessao SSH ativa ou de um perfil salvo.

## Nao-objetivos

- VPN ou proxy global do sistema.
- Persistir trafego ou inspecionar conteudo.
- Garantir remote/dynamic no primeiro incremento se a crate SSH ainda nao oferecer suporte maduro.

## Experiencia do usuario

O usuario abre um perfil ou uma sessao SSH ativa, acessa a area de tunnels, escolhe o tipo `local`, informa porta local, host/porta de destino e inicia o tunnel. A UI mostra status, porta vinculada, destino, sessao associada e um botao claro para parar. Se a porta estiver ocupada, o app informa o conflito sem derrubar a sessao SSH.

## Contratos

### Comandos Tauri

| Comando | Entrada | Saida | Erros |
| --- | --- | --- | --- |
| `start_tunnel` | `TunnelRequest` | `TunnelStatus` | `port_in_use`, `permission_denied`, `ssh_error`, `unsupported_mode`, `invalid_bind` |
| `stop_tunnel` | `{ tunnelId: string }` | `TunnelStatus` | `not_found` |
| `list_tunnels` | vazio | `TunnelStatus[]` | `internal_error` |
| `list_session_tunnels` | `{ sessionId: string }` | `TunnelStatus[]` | `not_found` |

### Modelos

```ts
type TunnelRequest = {
  sessionId?: string;
  profileId?: string;
  mode: "local" | "remote" | "dynamic";
  bindHost: string;
  bindPort: number;
  targetHost?: string;
  targetPort?: number;
  label?: string;
  allowExternalBind?: boolean;
};

type TunnelStatus = {
  tunnelId: string;
  sessionId: string;
  mode: "local" | "remote" | "dynamic";
  state: "starting" | "running" | "stopping" | "stopped" | "failed";
  bindHost: string;
  bindPort: number;
  targetHost?: string;
  targetPort?: number;
  label?: string;
  startedAt?: string;
  lastError?: string;
};
```

### Incrementos

| Incremento | Escopo | Status esperado |
| --- | --- | --- |
| TUN-1 | Local forwarding em sessao SSH ativa | Implementado |
| TUN-2 | Local forwarding iniciando a partir de perfil salvo | Depois de perfis/vault estaveis |
| TUN-3 | Remote forwarding | Depende da crate SSH |
| TUN-4 | Dynamic/SOCKS forwarding | Depende da crate SSH e UX de seguranca |

## Seguranca e privacidade

- Bind default deve ser `127.0.0.1`.
- Bind externo deve exigir confirmacao explicita.
- Logs nao inspecionam trafego.
- `targetHost` e `targetPort` podem revelar topologia interna; logs devem redigir ou reduzir esses campos em modo compartilhavel.
- O trafego encaminhado nao deve passar pelo frontend.
- Tunnels devem parar automaticamente quando a sessao SSH associada for encerrada.

## Plano de implementacao

1. Implementar registry de tunnels.
2. Validar suporte da crate SSH para local forwarding antes de fechar a decisao da SPEC-006.
3. Implementar local forwarding com bind default em `127.0.0.1`.
4. Adicionar comandos `start_tunnel`, `stop_tunnel`, `list_tunnels` e `list_session_tunnels`.
5. Criar UI de criar/parar tunnel usando Tailwind para formulario, tabela de status e indicadores.
6. Adicionar cleanup automatico ao fechar sessao.
7. Expandir para remote/dynamic conforme crate SSH.

## Implementacao entregue

- `TunnelRegistry` gerencia tunnels locais em memoria, associados a `sessionId`.
- `start_tunnel`, `stop_tunnel`, `list_tunnels` e `list_session_tunnels` usam implementacao real para `mode: "local"`.
- Cada conexao recebida no bind local abre um canal SSH `direct-tcpip` via `russh`; o trafego nao passa pelo frontend, IPC ou WebSocket do terminal.
- `disconnect_session` solicita parada dos tunnels da sessao antes de encerrar o shell.
- A UI ganhou painel para iniciar/parar tunnels locais e o indicador de tunnels passou a refletir tunnels ativos.
- Bind externo exige confirmacao no frontend e `allowExternalBind` no comando.

## Criterios de aceite

- Tunnel local encaminha trafego.
- Porta em uso retorna erro claro.
- Parar tunnel libera porta.
- Bind externo exige confirmacao.
- Tunnel aparece associado a sessao correta.
- Encerrar sessao SSH para todos os tunnels relacionados.
- Trafego de tunnel nao passa por IPC nem pelo WebSocket do terminal.

## Plano de testes

- Integration test com servico local simples.
- Teste manual de tunnel local.
- Teste manual de erro de porta ocupada.
- Teste manual de cleanup ao desconectar sessao.
- Teste manual de tentativa de bind externo.
- Unit tests para validacao de bind externo e campos obrigatorios.

## Riscos e decisoes abertas

- Decisao: TUN-1 foi implementado primeiro; TUN-2, TUN-3 e TUN-4 seguem adiados.
- Decisao: tunnels locais usam uma conexao SSH separada derivada da `sessionId` ativa para cada conexao TCP aceita, preservando a sessao shell do terminal.
- Remote/dynamic forwarding podem depender fortemente da crate SSH.
- UX deve deixar claro que tunnel ativo expoe uma porta local enquanto estiver rodando.
- Risco: cleanup automatico cobre `disconnect_session` e monitoramento de sessao no accept loop; quedas abruptas ainda precisam validacao manual contra hosts reais.
