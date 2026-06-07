# SPEC-009 - Tabs e split panes

Status: Done
Prioridade: P1
Fonte: README.md

## Problema

Uma ferramenta SSH moderna precisa permitir multiplas sessoes simultaneas, alternancia rapida e layouts com divisoes, sem perder estado ou foco.

## Objetivos

- Criar tabs de sessoes.
- Criar split panes com layout persistivel.
- Manter foco de terminal previsivel.
- Fechar sessoes e panes com limpeza correta.

## Nao-objetivos

- Multiplexacao SSH no servidor remoto.
- Restaurar sessoes SSH apos reiniciar app.

## Contratos

### Modelos persistidos

```ts
type SessionLayout = {
  version: 1;
  tabs: TerminalTab[];
  activeTabId?: string;
};

type TerminalTab = {
  id: string;
  title: string;
  rootPane: PaneNode;
  activePaneId?: string;
};

type PaneNode =
  | { type: "terminal"; paneId: string; sessionId?: string; title?: string }
  | { type: "split"; direction: "horizontal" | "vertical"; ratio: number; first: PaneNode; second: PaneNode };
```

## Seguranca e privacidade

- Layout persistido nao deve conter scrollback, output ou segredos.
- Titulo automatico nao deve incluir segredo de comando.
- `sessionId` vivo e removido antes da persistencia; o layout salvo guarda apenas forma, titulos seguros e ids de panes.

## Plano de implementacao

1. Criar store de tabs e panes.
2. Implementar criar/fechar/renomear tab.
3. Implementar split horizontal e vertical.
4. Implementar foco e atalhos opcionais.
5. Persistir somente layout seguro.

## Decisoes de implementacao

- O layout fica em `src/lib/layout.ts` como modulo puro, com testes unitarios via Vitest.
- Cada terminal tem `paneId` estavel para preservar o componente `xterm` quando a arvore ganha novos splits.
- Tabs inativas permanecem montadas e ocultas para manter sessoes WebSocket/SSH vivas durante alternancia.
- A persistencia MVP usa `localStorage` com chave `sypherterm.sessionLayout`, sem scrollback, tokens, credenciais ou sessoes vivas.
- Renderizacao visual de splits usa as folhas da arvore para preservar instancias; splits aninhados sao representados no modelo, enquanto o layout visual MVP usa a direcao principal da tab.
- Fechar a ultima pane fecha a tab; fechar a ultima tab cria uma nova tab vazia.
- Fechar pane/tab com conexao ativa pede confirmacao e desmonta o terminal, encerrando a conexao local.

## Criterios de aceite

- Usuario abre duas sessoes em tabs diferentes.
- Usuario divide uma tab em dois panes.
- Fechar pane encerra ou desanexa sessao conforme confirmacao definida.
- Layout recarrega sem restaurar bytes sensiveis.

## Plano de testes

- Unit tests para manipulacao de arvore de panes.
- Teste manual de foco.
- Teste manual de resize de panes.

Verificacoes executadas:

- `corepack pnpm check`
- `corepack pnpm build`
- `corepack pnpm test:unit`
- `cargo fmt --check`
- `cargo check`
- `cargo test`

## Riscos e decisoes abertas

- Teste manual de foco/resize em panes com sessoes SSH reais ainda deve ser feito.
- O layout visual MVP usa a direcao principal do split para grade; refinamento futuro pode renderizar ratios e splits aninhados de forma fiel.
