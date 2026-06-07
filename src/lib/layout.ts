export type SplitDirection = 'horizontal' | 'vertical';

export type SessionLayout = {
  version: 1;
  tabs: TerminalTab[];
  activeTabId?: string;
};

export type TerminalTab = {
  id: string;
  title: string;
  rootPane: PaneNode;
  activePaneId?: string;
};

export type TerminalPaneNode = {
  type: 'terminal';
  paneId: string;
  sessionId?: string;
  title?: string;
};

export type SplitPaneNode = {
  type: 'split';
  direction: SplitDirection;
  ratio: number;
  first: PaneNode;
  second: PaneNode;
};

export type PaneNode = TerminalPaneNode | SplitPaneNode;

const LAYOUT_VERSION = 1;
const STORAGE_KEY = 'sypherterm.sessionLayout';

export function createDefaultLayout(): SessionLayout {
  const pane = createTerminalPane('Terminal');
  const tab = createTab('Session 1', pane);

  return {
    version: LAYOUT_VERSION,
    tabs: [tab],
    activeTabId: tab.id,
  };
}

export function createTab(title: string, pane = createTerminalPane(title)): TerminalTab {
  return {
    id: crypto.randomUUID(),
    title,
    rootPane: pane,
    activePaneId: pane.paneId,
  };
}

export function createTerminalPane(title = 'Terminal'): TerminalPaneNode {
  return {
    type: 'terminal',
    paneId: crypto.randomUUID(),
    title,
  };
}

export function getActiveTab(layout: SessionLayout): TerminalTab | undefined {
  return layout.tabs.find((tab) => tab.id === layout.activeTabId) ?? layout.tabs[0];
}

export function getActivePaneId(layout: SessionLayout): string {
  const activeTab = getActiveTab(layout);
  return activeTab?.activePaneId ?? flattenTerminalPanes(activeTab?.rootPane).at(0)?.paneId ?? '';
}

export function flattenTerminalPanes(pane?: PaneNode): TerminalPaneNode[] {
  if (!pane) {
    return [];
  }

  if (pane.type === 'terminal') {
    return [pane];
  }

  return [...flattenTerminalPanes(pane.first), ...flattenTerminalPanes(pane.second)];
}

export function primarySplitDirection(pane?: PaneNode): SplitDirection {
  return pane?.type === 'split' ? pane.direction : 'horizontal';
}

export function addTab(layout: SessionLayout, title = `Session ${layout.tabs.length + 1}`): SessionLayout {
  const tab = createTab(title);

  return {
    ...layout,
    tabs: [...layout.tabs, tab],
    activeTabId: tab.id,
  };
}

export function setActiveTab(layout: SessionLayout, tabId: string): SessionLayout {
  if (!layout.tabs.some((tab) => tab.id === tabId)) {
    return layout;
  }

  return {
    ...layout,
    activeTabId: tabId,
  };
}

export function setActivePane(layout: SessionLayout, paneId: string): SessionLayout {
  return updateActiveTab(layout, (tab) => ({
    ...tab,
    activePaneId: paneId,
  }));
}

export function renameActiveTab(layout: SessionLayout, title: string): SessionLayout {
  const normalized = title.trim();
  if (!normalized) {
    return layout;
  }

  return updateActiveTab(layout, (tab) => ({
    ...tab,
    title: normalized,
  }));
}

export function splitPane(layout: SessionLayout, paneId: string, direction: SplitDirection): SessionLayout {
  const newPane = createTerminalPane('Terminal');

  return updateActiveTab(layout, (tab) => ({
    ...tab,
    rootPane: splitPaneNode(tab.rootPane, paneId, direction, newPane),
    activePaneId: newPane.paneId,
  }));
}

export function updatePaneSession(
  layout: SessionLayout,
  paneId: string,
  sessionId: string,
  title: string,
): SessionLayout {
  return updateActiveTab(layout, (tab) => ({
    ...tab,
    title: tab.title.startsWith('Session ') ? title : tab.title,
    rootPane: mapPane(tab.rootPane, (pane) =>
      pane.paneId === paneId ? { ...pane, sessionId, title } : pane,
    ),
    activePaneId: paneId,
  }));
}

export function clearPaneSession(layout: SessionLayout, paneId: string): SessionLayout {
  return updateActiveTab(layout, (tab) => ({
    ...tab,
    rootPane: mapPane(tab.rootPane, (pane) =>
      pane.paneId === paneId ? { ...pane, sessionId: undefined } : pane,
    ),
  }));
}

export function closePane(layout: SessionLayout, paneId: string): SessionLayout {
  const activeTab = getActiveTab(layout);
  if (!activeTab) {
    return createDefaultLayout();
  }

  const remaining = removePane(activeTab.rootPane, paneId);
  if (!remaining) {
    return closeTab(layout, activeTab.id);
  }

  const panes = flattenTerminalPanes(remaining);
  return updateActiveTab(layout, (tab) => ({
    ...tab,
    rootPane: remaining,
    activePaneId: panes.at(0)?.paneId,
  }));
}

export function closeTab(layout: SessionLayout, tabId: string): SessionLayout {
  const tabs = layout.tabs.filter((tab) => tab.id !== tabId);
  if (tabs.length === 0) {
    return createDefaultLayout();
  }

  const activeTabId =
    layout.activeTabId === tabId ? tabs.at(Math.max(0, layout.tabs.findIndex((tab) => tab.id === tabId) - 1))?.id : layout.activeTabId;

  return {
    ...layout,
    tabs,
    activeTabId: activeTabId ?? tabs[0].id,
  };
}

export function collectPaneIds(pane?: PaneNode): string[] {
  return flattenTerminalPanes(pane).map((terminalPane) => terminalPane.paneId);
}

export function loadSessionLayout(): SessionLayout {
  const rawLayout = localStorage.getItem(STORAGE_KEY);
  if (!rawLayout) {
    return createDefaultLayout();
  }

  try {
    return sanitizeLayout(JSON.parse(rawLayout) as SessionLayout);
  } catch {
    return createDefaultLayout();
  }
}

export function persistSessionLayout(layout: SessionLayout) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(stripLiveSessions(layout)));
}

export function toPersistedSessionLayout(layout: SessionLayout): SessionLayout {
  return stripLiveSessions(layout);
}

function updateActiveTab(layout: SessionLayout, update: (tab: TerminalTab) => TerminalTab): SessionLayout {
  const activeTab = getActiveTab(layout);
  if (!activeTab) {
    return layout;
  }

  return {
    ...layout,
    activeTabId: activeTab.id,
    tabs: layout.tabs.map((tab) => (tab.id === activeTab.id ? update(tab) : tab)),
  };
}

function splitPaneNode(
  pane: PaneNode,
  paneId: string,
  direction: SplitDirection,
  newPane: TerminalPaneNode,
): PaneNode {
  if (pane.type === 'terminal') {
    return pane.paneId === paneId
      ? {
          type: 'split',
          direction,
          ratio: 0.5,
          first: pane,
          second: newPane,
        }
      : pane;
  }

  return {
    ...pane,
    first: splitPaneNode(pane.first, paneId, direction, newPane),
    second: splitPaneNode(pane.second, paneId, direction, newPane),
  };
}

function removePane(pane: PaneNode, paneId: string): PaneNode | null {
  if (pane.type === 'terminal') {
    return pane.paneId === paneId ? null : pane;
  }

  const first = removePane(pane.first, paneId);
  const second = removePane(pane.second, paneId);

  if (!first) {
    return second;
  }
  if (!second) {
    return first;
  }

  return {
    ...pane,
    first,
    second,
  };
}

function mapPane(pane: PaneNode, update: (pane: TerminalPaneNode) => TerminalPaneNode): PaneNode {
  if (pane.type === 'terminal') {
    return update(pane);
  }

  return {
    ...pane,
    first: mapPane(pane.first, update),
    second: mapPane(pane.second, update),
  };
}

function stripLiveSessions(layout: SessionLayout): SessionLayout {
  return {
    ...layout,
    tabs: layout.tabs.map((tab) => ({
      ...tab,
      rootPane: mapPane(tab.rootPane, (pane) => ({
        type: 'terminal',
        paneId: pane.paneId,
        title: pane.title,
      })),
    })),
  };
}

function sanitizeLayout(layout: SessionLayout): SessionLayout {
  if (layout.version !== LAYOUT_VERSION || layout.tabs.length === 0) {
    return createDefaultLayout();
  }

  return stripLiveSessions({
    version: LAYOUT_VERSION,
    tabs: layout.tabs,
    activeTabId: layout.activeTabId,
  });
}
