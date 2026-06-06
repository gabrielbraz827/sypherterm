# SPEC-007 - Terminal UI

Status: Draft
Prioridade: P0
Fonte: README.md, ARCHITECTURE.md

## Problema

O frontend precisa renderizar terminal de alta frequencia sem travar, com uma experiencia moderna, configuravel e integrada ao Data Plane.

## Objetivos

- Integrar `xterm.js` com Svelte.
- Conectar entrada e saida ao WebSocket local.
- Suportar fit/resize, temas, copy/paste e links.
- Isolar ciclo de vida do terminal por sessao.
- Usar Tailwind CSS para layout, superficies, toolbars e estados da UI, mantendo CSS local apenas para integracao fina do `xterm.js`.

## Nao-objetivos

- Implementar tabs e split panes completos.
- Implementar SFTP visual.

## Contratos

### Componentes

| Componente | Responsabilidade |
| --- | --- |
| `TerminalInstance.svelte` | Montar xterm, conectar WebSocket e gerenciar ciclo de vida |
| `TerminalToolbar.svelte` | Acoes de copiar, colar, limpar, reconectar |
| `SessionStatus.svelte` | Mostrar estado da sessao |
| `TunnelIndicator.svelte` | Mostrar tunnels ativos associados a sessao |

### API frontend

```ts
type TerminalConnection = {
  sessionId: string;
  wsUrl: string;
  authToken: string;
};
```

## Seguranca e privacidade

- Clipboard deve ser tratado como dado sensivel.
- Terminal nao deve armazenar scrollback em store persistente por padrao.
- Logs de frontend nao devem imprimir output do terminal.

## Plano de implementacao

1. Adicionar `xterm` e addons necessarios.
2. Criar wrapper `TerminalSocket` em `src/lib/websocket.ts`.
3. Criar `TerminalInstance.svelte`.
4. Implementar resize usando addon fit e `resize_session`.
5. Implementar copy/paste com plugin clipboard quando necessario.
6. Aplicar preferencias de tema e fonte.
7. Aplicar Tailwind para layout responsivo, toolbar, estados de sessao e paineis laterais.

## Criterios de aceite

- Terminal renderiza prompt remoto.
- Digitar envia bytes ao backend.
- Output continuo nao causa congelamento perceptivel.
- Resize do container ajusta cols/rows.
- Desmontar componente fecha handlers e conexao.
- UI de terminal usa Tailwind para elementos de aplicacao e nao sobrescreve estilos internos do canvas/DOM do `xterm.js` de forma fragil.

## Plano de testes

- Teste manual de conexao interativa.
- Teste manual de paste grande.
- Teste de build e typecheck.
- Verificacao visual em viewport desktop e menor.

## Riscos e decisoes abertas

- Definir limite de scrollback padrao para equilibrar memoria e usabilidade.
