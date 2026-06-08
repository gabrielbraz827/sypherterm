# SPEC-011 - Integracoes nativas

Status: Done
Prioridade: P1
Fonte: ARCHITECTURE.md

## Problema

SypherTerm usa plugins Tauri para clipboard, notificacoes, opener, OS info, store e WebSocket client. Essas integracoes precisam ter limites claros e permissoes minimas.

## Objetivos

- Usar plugins oficiais Tauri v2 ja previstos.
- Mapear cada permissao para uma necessidade de produto.
- Criar wrappers frontend consistentes.
- Evitar permissao ampla sem justificativa.

## Nao-objetivos

- Criar plugins Tauri customizados.
- Usar WebSocket client para Data Plane.

## Contratos

### Capacidades

| Plugin | Uso |
| --- | --- |
| Store | Configuracoes e metadata local |
| Clipboard | Copy/paste no terminal |
| Notification | Eventos de conexao e sync |
| OS Info | Ajustes por plataforma |
| Opener | Abrir URLs e caminhos autorizados |
| WebSocket | Cliente auxiliar para features futuras |

## Seguranca e privacidade

- Clipboard pode conter segredos; evitar historico persistente.
- Notificacoes nao devem mostrar senha, chave, output sensivel ou comando completo.
- Opener nao deve abrir caminhos arbitrarios sem acao do usuario.

## Plano de implementacao

1. Criar wrappers em `src/lib/native.ts`.
2. Revisar `src-tauri/capabilities/default.json`.
3. Documentar justificativa de cada permissao.
4. Integrar clipboard ao terminal.
5. Integrar notificacoes a eventos de sync/conexao.

## Implementacao entregue

- `src/lib/native.ts` centraliza clipboard, notificacoes, OS info e opener.
- Clipboard do terminal passou a usar o wrapper nativo, sem persistir historico no app.
- Notificacoes foram integradas a conexao SSH e sync com mensagens sanitizadas.
- `docs/NATIVE_PERMISSIONS.md` documenta cada permissao de `src-tauri/capabilities/default.json`.
- O Runtime status mostra a plataforma via plugin OS.

## Criterios de aceite

- Cada permissao tem uso documentado.
- Clipboard funciona no terminal.
- Notificacoes ocultam dados sensiveis.
- Data Plane nao depende do plugin WebSocket client.

## Plano de testes

- Teste manual de copy/paste.
- Teste manual de notificacao.
- Revisao de capabilities.
- Unit tests para sanitizacao de notificacoes.

## Riscos e decisoes abertas

- Diferencas de comportamento entre Windows, macOS e Linux.
- Decisao: `websocket:default` permanece habilitado por estar previsto na arquitetura, mas o Data Plane continua usando WebSocket nativo do browser contra o servidor local autenticado.
- Decisao: `opener:default` fica atras de wrapper que permite URLs `http/https` e paths apenas quando informados por acao explicita.
- Risco: permissoes de notificacao dependem de comportamento do sistema operacional e precisam validacao manual por plataforma.
