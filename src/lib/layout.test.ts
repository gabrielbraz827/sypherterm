import { describe, expect, it } from 'vitest';

import {
  closePane,
  closeTab,
  createDefaultLayout,
  flattenTerminalPanes,
  splitPane,
  toPersistedSessionLayout,
  updatePaneSession,
} from './layout';

describe('session layout', () => {
  it('splits a terminal pane without replacing the existing pane id', () => {
    const layout = createDefaultLayout();
    const originalPane = flattenTerminalPanes(layout.tabs[0].rootPane)[0];
    const split = splitPane(layout, originalPane.paneId, 'horizontal');
    const panes = flattenTerminalPanes(split.tabs[0].rootPane);

    expect(panes).toHaveLength(2);
    expect(panes[0].paneId).toBe(originalPane.paneId);
    expect(split.tabs[0].activePaneId).toBe(panes[1].paneId);
  });

  it('closes a pane and collapses the split tree', () => {
    const layout = createDefaultLayout();
    const originalPane = flattenTerminalPanes(layout.tabs[0].rootPane)[0];
    const split = splitPane(layout, originalPane.paneId, 'vertical');
    const newPane = flattenTerminalPanes(split.tabs[0].rootPane)[1];
    const closed = closePane(split, newPane.paneId);
    const panes = flattenTerminalPanes(closed.tabs[0].rootPane);

    expect(panes).toHaveLength(1);
    expect(panes[0].paneId).toBe(originalPane.paneId);
  });

  it('creates a fresh empty tab when the last tab is closed', () => {
    const layout = createDefaultLayout();
    const closed = closeTab(layout, layout.tabs[0].id);

    expect(closed.tabs).toHaveLength(1);
    expect(closed.activeTabId).toBe(closed.tabs[0].id);
  });

  it('does not persist live session ids', () => {
    const layout = createDefaultLayout();
    const pane = flattenTerminalPanes(layout.tabs[0].rootPane)[0];
    const withSession = updatePaneSession(layout, pane.paneId, 'live-session', 'Prod');
    const persisted = toPersistedSessionLayout(withSession);

    expect(flattenTerminalPanes(persisted.tabs[0].rootPane)[0].sessionId).toBeUndefined();
  });
});
