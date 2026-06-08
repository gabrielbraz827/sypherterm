# Permissoes nativas do SypherTerm

Este documento mapeia as permissoes Tauri habilitadas em `src-tauri/capabilities/default.json` para usos concretos do produto.

| Permissao | Plugin | Uso no produto | Limite de seguranca |
| --- | --- | --- | --- |
| `core:default` | Core | Permite IPC basico da janela principal com os comandos registrados. | Comandos retornam `CommandError` padronizado e nao devem incluir segredos em mensagens. |
| `store:default` | Store | Persistencia local de preferencias, metadata de perfis, layout e envelope criptografado do vault. | Segredos ficam no `VaultEnvelope`; metadata nao deve conter senha, chave privada ou output de terminal. |
| `clipboard-manager:default` | Clipboard | Copy/paste manual no terminal via `src/lib/native.ts`. | Clipboard nao e persistido pelo app e o wrapper so manipula texto acionado pelo usuario. |
| `notification:default` | Notification | Notificacoes de conexao SSH e sync. | Corpo passa por sanitizacao e nao inclui senha, chave, token, comando completo, output ou hash longo. |
| `os:default` | OS Info | Mostra plataforma no status e prepara ajustes por sistema operacional. | Informacao usada apenas para UI/configuracao local. |
| `opener:default` | Opener | Abrir URLs http/https e revelar caminhos autorizados por acao explicita do usuario. | Wrapper bloqueia protocolos nao web para URL e exige path informado explicitamente. |
| `websocket:default` | WebSocket client | Reservado para features futuras que precisem de cliente WebSocket externo. | O Data Plane atual usa `WebSocket` do browser contra servidor local autenticado, nao este plugin. |

## Decisoes

- As integracoes nativas do frontend passam por `src/lib/native.ts`.
- Notificacoes sao tratadas como mensagens de status, nunca como local para dados operacionais sensiveis.
- O uso de opener deve continuar partindo de acao explicita do usuario.
- A permissao `websocket:default` permanece habilitada por enquanto porque estava prevista na arquitetura, mas nao e dependencia do Data Plane local.
