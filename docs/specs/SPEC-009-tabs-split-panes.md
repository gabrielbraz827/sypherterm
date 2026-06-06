# SPEC-009 - Tabs e split panes

Status: Draft
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
};

type PaneNode =
  | { type: "terminal"; sessionId: string }
  | { type: "split"; direction: "horizontal" | "vertical"; ratio: number; first: PaneNode; second: PaneNode };
```

## Seguranca e privacidade

- Layout persistido nao deve conter scrollback, output ou segredos.
- Titulo automatico nao deve incluir segredo de comando.

## Plano de implementacao

1. Criar store de tabs e panes.
2. Implementar criar/fechar/renomear tab.
3. Implementar split horizontal e vertical.
4. Implementar foco e atalhos opcionais.
5. Persistir somente layout seguro.

## Criterios de aceite

- Usuario abre duas sessoes em tabs diferentes.
- Usuario divide uma tab em dois panes.
- Fechar pane encerra ou desanexa sessao conforme confirmacao definida.
- Layout recarrega sem restaurar bytes sensiveis.

## Plano de testes

- Unit tests para manipulacao de arvore de panes.
- Teste manual de foco.
- Teste manual de resize de panes.

## Riscos e decisoes abertas

- Definir comportamento quando a ultima pane de uma tab e fechada.
