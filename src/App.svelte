<script lang="ts">
  import { onMount } from 'svelte';

  import TerminalPane from './lib/components/TerminalPane.svelte';
  import TunnelIndicator from './lib/components/TunnelIndicator.svelte';
  import {
    changeMasterPassword,
    connectSsh,
    createVault,
    deleteProfile,
    duplicateProfile,
    getAppStatus,
    getPreferences,
    lockVault,
    listProfiles,
    savePreferences,
    saveProfile,
    unlockVault,
    type AppStatus,
    type ConnectionProfileSummary,
    type UserPreferences,
    type VaultStatus,
  } from './lib/api';
  import {
    addTab,
    clearPaneSession,
    closePane,
    closeTab,
    collectPaneIds,
    createDefaultLayout,
    flattenTerminalPanes,
    getActivePaneId,
    getActiveTab,
    loadSessionLayout,
    persistSessionLayout,
    primarySplitDirection,
    renameActiveTab,
    setActivePane,
    setActiveTab,
    splitPane,
    updatePaneSession,
    type SessionLayout,
    type TerminalTab,
  } from './lib/layout';
  import type { TerminalConnection } from './lib/websocket';
  import {
    appStatus as appStatusStore,
    preferences as preferencesStore,
    profiles as profilesStore,
    vaultStatus as vaultStatusStore,
  } from './lib/store';

  let appStatus: AppStatus | null = null;
  let vaultStatus: VaultStatus | null = null;
  let profiles: ConnectionProfileSummary[] = [];
  let preferences: UserPreferences | null = null;
  let loadError = '';
  let formError = '';
  let vaultError = '';
  let vaultMessage = '';
  let masterPassword = '';
  let currentPassword = '';
  let newPassword = '';
  let profileName = '';
  let profileHost = '';
  let profilePort = 22;
  let profileUsername = '';
  let profileTags = '';
  let profileGroup = '';
  let profileCredentialRef = '';
  let editingProfileId = '';
  let profileSearch = '';
  let groupFilter = '';
  let tagFilter = '';
  let recentFirst = true;
  let selectedProfileId = '';
  let selectedProfile: ConnectionProfileSummary | null = null;
  let connectUsername = '';
  let authMode: 'password' | 'key' = 'password';
  let sshPassword = '';
  let privateKeyPath = '';
  let passphrase = '';
  let connectionError = '';
  let isConnecting = false;
  let sessionLayout: SessionLayout = createDefaultLayout();
  let layoutReady = false;
  let paneConnections: Record<string, TerminalConnection | undefined> = {};
  let paneStatuses: Record<string, { state: string; message?: string }> = {};

  const statusLabel: Record<AppStatus['vault'] | AppStatus['dataPlane'] | UserPreferences['theme'], string> = {
    missing: 'Missing',
    locked: 'Locked',
    unlocked: 'Unlocked',
    stopped: 'Stopped',
    starting: 'Starting',
    running: 'Running',
    system: 'System',
    dark: 'Dark',
    light: 'Light',
  };

  onMount(async () => {
    try {
      sessionLayout = loadSessionLayout();
      layoutReady = true;
      await loadRuntimeData();
    } catch (error) {
      loadError = error instanceof Error ? error.message : String(error);
    }
  });

  $: selectedProfile = profiles.find((profile) => profile.id === selectedProfileId) ?? null;
  $: activeTab = getActiveTab(sessionLayout);
  $: activePaneId = getActivePaneId(sessionLayout);
  $: activePaneStatus = paneStatuses[activePaneId] ?? { state: 'idle', message: '' };
  $: if (layoutReady) {
    persistSessionLayout(sessionLayout);
  }
  $: allGroups = Array.from(
    new Set(profiles.map((profile) => profile.groupId).filter((group): group is string => Boolean(group))),
  ).sort((left, right) => left.localeCompare(right));
  $: allTags = Array.from(new Set(profiles.flatMap((profile) => profile.tags))).sort((left, right) =>
    left.localeCompare(right),
  );
  $: visibleProfiles = filteredProfiles(profiles, profileSearch, groupFilter, tagFilter, recentFirst);

  async function loadRuntimeData() {
    const [status, storedProfiles, storedPreferences] = await Promise.all([
      getAppStatus(),
      listProfiles(),
      getPreferences(),
    ]);

    appStatus = status;
    vaultStatus = { state: status.vault };
    profiles = storedProfiles;
    preferences = storedPreferences;
    appStatusStore.set(status);
    vaultStatusStore.set(vaultStatus);
    profilesStore.set(storedProfiles);
    preferencesStore.set(storedPreferences);

    if (!selectedProfileId && storedProfiles[0]) {
      selectProfile(storedProfiles[0]);
    }
  }

  async function refreshStatus() {
    appStatus = await getAppStatus();
    vaultStatus = { state: appStatus.vault, version: vaultStatus?.version };
    appStatusStore.set(appStatus);
    vaultStatusStore.set(vaultStatus);
  }

  async function reloadProfiles() {
    profiles = await listProfiles();
    profilesStore.set(profiles);
    if (selectedProfileId && !profiles.some((profile) => profile.id === selectedProfileId)) {
      selectedProfileId = profiles[0]?.id ?? '';
    }
  }

  async function submitCreateVault() {
    vaultError = '';
    vaultMessage = '';

    try {
      vaultStatus = await createVault({ masterPassword });
      vaultStatusStore.set(vaultStatus);
      await refreshStatus();
      vaultMessage = 'Vault created and unlocked.';
    } catch (error) {
      vaultError = formatError(error);
    } finally {
      masterPassword = '';
    }
  }

  async function submitUnlockVault() {
    vaultError = '';
    vaultMessage = '';

    try {
      vaultStatus = await unlockVault({ masterPassword });
      vaultStatusStore.set(vaultStatus);
      await refreshStatus();
      vaultMessage = 'Vault unlocked.';
    } catch (error) {
      vaultError = formatError(error);
    } finally {
      masterPassword = '';
    }
  }

  async function submitLockVault() {
    vaultError = '';
    vaultMessage = '';

    try {
      vaultStatus = await lockVault();
      vaultStatusStore.set(vaultStatus);
      await refreshStatus();
      vaultMessage = 'Vault locked.';
    } catch (error) {
      vaultError = formatError(error);
    }
  }

  async function submitChangeMasterPassword() {
    vaultError = '';
    vaultMessage = '';

    try {
      vaultStatus = await changeMasterPassword({
        currentPassword,
        newPassword,
      });
      vaultStatusStore.set(vaultStatus);
      await refreshStatus();
      vaultMessage = 'Master password changed.';
    } catch (error) {
      vaultError = formatError(error);
    } finally {
      currentPassword = '';
      newPassword = '';
    }
  }

  async function submitProfile() {
    formError = '';

    try {
      const savedProfile = await saveProfile({
        id: editingProfileId || undefined,
        name: profileName,
        host: profileHost,
        port: profilePort,
        username: profileUsername || undefined,
        groupId: profileGroup || undefined,
        tags: profileTags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        credentialRef: profileCredentialRef || undefined,
      });

      resetProfileForm();
      await reloadProfiles();
      selectedProfileId = savedProfile.id;
      connectUsername = savedProfile.username ?? '';
    } catch (error) {
      formError = formatError(error);
    }
  }

  async function removeProfile(id: string) {
    formError = '';

    if (profileHasActivePane(id)) {
      const shouldDelete = window.confirm('This profile has an active terminal session. Disconnect and delete it?');
      if (!shouldDelete) {
        return;
      }
      closePanesForProfile(id);
    }

    try {
      await deleteProfile(id);
      await reloadProfiles();
      if (selectedProfileId === id) {
        selectedProfileId = profiles[0]?.id ?? '';
        connectUsername = profiles[0]?.username ?? '';
      }
    } catch (error) {
      formError = formatError(error);
    }
  }

  function editProfile(profile: ConnectionProfileSummary) {
    editingProfileId = profile.id;
    profileName = profile.name;
    profileHost = profile.host;
    profilePort = profile.port;
    profileUsername = profile.username ?? '';
    profileGroup = profile.groupId ?? '';
    profileTags = profile.tags.join(', ');
    profileCredentialRef = profile.hasCredential ? '' : '';
    formError = '';
  }

  async function duplicateSelectedProfile(id: string) {
    formError = '';

    try {
      const duplicated = await duplicateProfile(id);
      await reloadProfiles();
      selectedProfileId = duplicated.id;
      connectUsername = duplicated.username ?? '';
      editProfile({
        ...duplicated,
        hasCredential: Boolean(duplicated.credentialRef),
      });
    } catch (error) {
      formError = formatError(error);
    }
  }

  function resetProfileForm() {
    editingProfileId = '';
    profileName = '';
    profileHost = '';
    profilePort = 22;
    profileUsername = '';
    profileGroup = '';
    profileTags = '';
    profileCredentialRef = '';
  }

  function selectProfile(profile: ConnectionProfileSummary) {
    selectedProfileId = profile.id;
    connectUsername = profile.username ?? '';
    connectionError = '';
  }

  async function connectSelectedProfile() {
    if (!selectedProfile) {
      return;
    }

    const paneId = activePaneId;
    if (!paneId) {
      return;
    }

    connectionError = '';
    isConnecting = true;
    paneStatuses = {
      ...paneStatuses,
      [paneId]: { state: 'connecting', message: '' },
    };

    try {
      const connection = await connectSsh({
        profileId: selectedProfile.id,
        host: selectedProfile.host,
        port: selectedProfile.port,
        username: connectUsername || selectedProfile.username,
        password: authMode === 'password' ? sshPassword || undefined : undefined,
        privateKeyPath: authMode === 'key' ? privateKeyPath || undefined : undefined,
        passphrase: authMode === 'key' ? passphrase || undefined : undefined,
        cols: 80,
        rows: 24,
      });
      paneConnections = {
        ...paneConnections,
        [paneId]: connection,
      };
      sessionLayout = updatePaneSession(sessionLayout, paneId, connection.sessionId, selectedProfile.name);
      sshPassword = '';
      passphrase = '';
      await reloadProfiles();
      await refreshStatus();
    } catch (error) {
      connectionError = formatError(error);
      paneStatuses = {
        ...paneStatuses,
        [paneId]: { state: 'failed', message: connectionError },
      };
    } finally {
      isConnecting = false;
    }
  }

  function addSessionTab() {
    sessionLayout = addTab(sessionLayout);
  }

  function activateTab(tabId: string) {
    sessionLayout = setActiveTab(sessionLayout, tabId);
  }

  function renameCurrentTab() {
    const tab = getActiveTab(sessionLayout);
    const title = window.prompt('Tab title', tab?.title ?? '');
    if (title !== null) {
      sessionLayout = renameActiveTab(sessionLayout, title);
    }
  }

  function closeSessionTab(tab: TerminalTab) {
    const paneIds = collectPaneIds(tab.rootPane);
    const hasConnections = paneIds.some((paneId) => paneConnections[paneId]);
    if (hasConnections && !window.confirm('Close this tab and disconnect its panes?')) {
      return;
    }

    const nextConnections = { ...paneConnections };
    const nextStatuses = { ...paneStatuses };
    for (const paneId of paneIds) {
      delete nextConnections[paneId];
      delete nextStatuses[paneId];
    }
    paneConnections = nextConnections;
    paneStatuses = nextStatuses;
    sessionLayout = closeTab(sessionLayout, tab.id);
    void refreshStatus();
  }

  function focusPane(paneId: string) {
    sessionLayout = setActivePane(sessionLayout, paneId);
  }

  function splitActivePane(paneId: string, direction: 'horizontal' | 'vertical') {
    sessionLayout = splitPane(setActivePane(sessionLayout, paneId), paneId, direction);
  }

  function closeTerminalPane(paneId: string) {
    if (paneConnections[paneId] && !window.confirm('Close this pane and disconnect its SSH session?')) {
      return;
    }

    const nextConnections = { ...paneConnections };
    const nextStatuses = { ...paneStatuses };
    delete nextConnections[paneId];
    delete nextStatuses[paneId];
    paneConnections = nextConnections;
    paneStatuses = nextStatuses;
    sessionLayout = closePane(sessionLayout, paneId);
    void refreshStatus();
  }

  function handlePaneDisconnected(paneId: string) {
    const nextConnections = { ...paneConnections };
    delete nextConnections[paneId];
    paneConnections = nextConnections;
    paneStatuses = {
      ...paneStatuses,
      [paneId]: { state: 'closed', message: '' },
    };
    sessionLayout = clearPaneSession(sessionLayout, paneId);
    void refreshStatus();
  }

  function handlePaneStatus(event: CustomEvent<{ paneId: string; state: string; message?: string }>) {
    paneStatuses = {
      ...paneStatuses,
      [event.detail.paneId]: {
        state: event.detail.state,
        message: event.detail.message ?? '',
      },
    };

    if (event.detail.state === 'closed') {
      sessionLayout = clearPaneSession(sessionLayout, event.detail.paneId);
      void refreshStatus();
    }
  }

  function reconnectPane(paneId: string) {
    sessionLayout = setActivePane(sessionLayout, paneId);
    void connectSelectedProfile();
  }

  async function cycleTheme() {
    if (!preferences) {
      return;
    }

    const nextTheme: UserPreferences['theme'] =
      preferences.theme === 'system' ? 'dark' : preferences.theme === 'dark' ? 'light' : 'system';

    try {
      preferences = await savePreferences({
        ...preferences,
        theme: nextTheme,
      });
      preferencesStore.set(preferences);
    } catch (error) {
      formError = formatError(error);
    }
  }

  function formatError(error: unknown) {
    if (typeof error === 'object' && error && 'message' in error) {
      return String(error.message);
    }

    return error instanceof Error ? error.message : String(error);
  }

  function filteredProfiles(
    source: ConnectionProfileSummary[],
    query: string,
    groupId: string,
    tag: string,
    sortRecentFirst: boolean,
  ) {
    const normalizedQuery = query.trim().toLowerCase();
    const result = source.filter((profile) => {
      if (groupId && profile.groupId !== groupId) {
        return false;
      }
      if (tag && !profile.tags.includes(tag)) {
        return false;
      }
      if (!normalizedQuery) {
        return true;
      }

      return (
        profile.name.toLowerCase().includes(normalizedQuery) ||
        profile.host.toLowerCase().includes(normalizedQuery) ||
        (profile.username ?? '').toLowerCase().includes(normalizedQuery) ||
        (profile.groupId ?? '').toLowerCase().includes(normalizedQuery) ||
        profile.tags.some((profileTag) => profileTag.toLowerCase().includes(normalizedQuery))
      );
    });

    return [...result].sort((left, right) => {
      if (sortRecentFirst) {
        const recent = (right.lastUsedAt ?? '').localeCompare(left.lastUsedAt ?? '');
        if (recent !== 0) {
          return recent;
        }
      }
      return left.name.localeCompare(right.name);
    });
  }

  function tabGridClass(tab: TerminalTab) {
    const panes = flattenTerminalPanes(tab.rootPane);
    if (panes.length <= 1) {
      return 'grid-cols-1';
    }

    return primarySplitDirection(tab.rootPane) === 'vertical'
      ? 'grid-cols-1 grid-rows-2'
      : 'grid-cols-1 lg:grid-cols-2';
  }

  function profileHasActivePane(profileId: string) {
    const profile = profiles.find((candidate) => candidate.id === profileId);
    if (!profile) {
      return false;
    }

    return Object.values(paneConnections).some((connection) =>
      Boolean(connection && sessionLayout.tabs.some((tab) =>
        flattenTerminalPanes(tab.rootPane).some((pane) => pane.sessionId === connection.sessionId && pane.title === profile.name),
      )),
    );
  }

  function closePanesForProfile(profileId: string) {
    const profile = profiles.find((candidate) => candidate.id === profileId);
    if (!profile) {
      return;
    }

    for (const tab of sessionLayout.tabs) {
      for (const pane of flattenTerminalPanes(tab.rootPane)) {
        if (pane.title === profile.name) {
          closeTerminalPane(pane.paneId);
        }
      }
    }
  }
</script>

<main class="min-h-screen bg-sypher-bg text-sypher-text">
  <section class="grid min-h-screen grid-cols-1 lg:grid-cols-[280px_1fr]">
    <aside class="border-r border-sypher-border bg-sypher-panel p-4">
      <div class="mb-6">
        <p class="text-xs uppercase text-sypher-muted">SypherTerm</p>
        <h1 class="mt-1 text-xl font-semibold">Local-first SSH</h1>
      </div>

      <div class="mb-4 space-y-2">
        <input
          class="w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
          bind:value={profileSearch}
          placeholder="Search profiles"
        />
        <div class="grid grid-cols-2 gap-2">
          <select
            class="min-w-0 rounded border border-sypher-border bg-sypher-surface px-2 py-2 text-xs text-sypher-text outline-none focus:border-sypher-accent"
            bind:value={groupFilter}
          >
            <option value="">All groups</option>
            {#each allGroups as group}
              <option value={group}>{group}</option>
            {/each}
          </select>
          <select
            class="min-w-0 rounded border border-sypher-border bg-sypher-surface px-2 py-2 text-xs text-sypher-text outline-none focus:border-sypher-accent"
            bind:value={tagFilter}
          >
            <option value="">All tags</option>
            {#each allTags as tag}
              <option value={tag}>{tag}</option>
            {/each}
          </select>
        </div>
        <label class="flex items-center gap-2 text-xs text-sypher-muted">
          <input class="h-4 w-4 accent-sypher-accent" bind:checked={recentFirst} type="checkbox" />
          Recent first
        </label>
      </div>

      <div class="space-y-3">
        {#if profiles.length === 0}
          <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-muted">
            No profiles yet
          </div>
        {:else if visibleProfiles.length === 0}
          <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-muted">
            No matches
          </div>
        {:else}
          {#each visibleProfiles as profile}
            <div
              class={`rounded-panel border p-3 ${
                selectedProfileId === profile.id
                  ? 'border-sypher-accent bg-sypher-surface'
                  : 'border-sypher-border bg-sypher-surface'
              }`}
            >
              <div class="flex items-start justify-between gap-2">
                <button class="min-w-0 flex-1 text-left" type="button" on:click={() => selectProfile(profile)}>
                  <p class="truncate text-sm font-medium">{profile.name}</p>
                  <p class="truncate text-xs text-sypher-muted">{profile.username ? `${profile.username}@` : ''}{profile.host}:{profile.port}</p>
                </button>
                <button
                  class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                  type="button"
                  on:click={() => editProfile(profile)}
                >
                  Edit
                </button>
              </div>
              <div class="mt-2 flex items-center justify-between gap-2">
                <div class="min-w-0 text-xs text-sypher-muted">
                  {#if profile.groupId}
                    <span>{profile.groupId}</span>
                  {:else}
                    <span>No group</span>
                  {/if}
                  {#if profile.lastUsedAt}
                    <span> - recent</span>
                  {/if}
                </div>
                <div class="flex shrink-0 gap-2">
                  <button
                    class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                    type="button"
                    on:click={() => duplicateSelectedProfile(profile.id)}
                  >
                    Copy
                  </button>
                  <button
                    class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                    type="button"
                    on:click={() => removeProfile(profile.id)}
                  >
                    Delete
                  </button>
                </div>
              </div>
              {#if profile.tags.length}
                <p class="mt-2 truncate text-xs text-sypher-muted">{profile.tags.join(', ')}</p>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    </aside>

    <section class="flex min-w-0 flex-col">
      <header class="flex h-14 items-center justify-between border-b border-sypher-border px-5">
        <div>
          <p class="text-sm font-medium">{activeTab?.title ?? 'Terminal'}</p>
          <p class="text-xs text-sypher-muted">
            {selectedProfile ? `${selectedProfile.host}:${selectedProfile.port}` : 'No profile selected'}
          </p>
        </div>
        <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-1 text-xs text-sypher-muted">
          v{appStatus?.appVersion ?? '...'}
        </div>
      </header>

      <div class="grid flex-1 grid-cols-1 gap-4 p-5 xl:grid-cols-[minmax(0,1fr)_360px]">
        <section class="flex min-h-[520px] min-w-0 flex-col rounded-panel border border-sypher-border bg-sypher-bg">
          <div class="flex min-h-12 items-center justify-between gap-3 border-b border-sypher-border px-3 py-2">
            <div class="flex min-w-0 flex-1 gap-2 overflow-x-auto">
              {#each sessionLayout.tabs as tab}
                <button
                  class={`min-w-0 rounded border px-3 py-2 text-left text-xs ${
                    sessionLayout.activeTabId === tab.id
                      ? 'border-sypher-accent bg-sypher-surface text-sypher-text'
                      : 'border-sypher-border bg-sypher-panel text-sypher-muted'
                  }`}
                  type="button"
                  on:click={() => activateTab(tab.id)}
                >
                  <span class="block max-w-36 truncate">{tab.title}</span>
                </button>
              {/each}
            </div>
            <div class="flex shrink-0 gap-2">
              <button
                class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                type="button"
                on:click={addSessionTab}
              >
                New tab
              </button>
              <button
                class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                type="button"
                on:click={renameCurrentTab}
              >
                Rename
              </button>
              {#if activeTab}
                <button
                  class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                  type="button"
                  on:click={() => closeSessionTab(activeTab)}
                >
                  Close tab
                </button>
              {/if}
            </div>
          </div>

          <div class="relative min-h-0 flex-1">
            {#each sessionLayout.tabs as tab (tab.id)}
              <div
                class={`absolute inset-0 grid gap-3 p-3 ${tabGridClass(tab)} ${
                  sessionLayout.activeTabId === tab.id ? '' : 'pointer-events-none hidden'
                }`}
              >
                {#each flattenTerminalPanes(tab.rootPane) as pane (pane.paneId)}
                  <TerminalPane
                    {pane}
                    active={activePaneId === pane.paneId && sessionLayout.activeTabId === tab.id}
                    connection={paneConnections[pane.paneId]}
                    {preferences}
                    state={paneStatuses[pane.paneId]?.state ?? 'idle'}
                    message={paneStatuses[pane.paneId]?.message ?? ''}
                    on:focus={(event) => focusPane(event.detail.paneId)}
                    on:split={(event) => splitActivePane(event.detail.paneId, event.detail.direction)}
                    on:close={(event) => closeTerminalPane(event.detail.paneId)}
                    on:reconnect={(event) => reconnectPane(event.detail.paneId)}
                    on:disconnected={(event) => handlePaneDisconnected(event.detail.paneId)}
                    on:status={handlePaneStatus}
                  />
                {/each}
              </div>
            {/each}
          </div>
        </section>

        <aside class="rounded-panel border border-sypher-border bg-sypher-panel p-4">
          <h2 class="text-sm font-semibold">Runtime status</h2>

          {#if loadError}
            <p class="mt-4 rounded-panel border border-red-900/60 bg-red-950/40 p-3 text-sm text-red-200">
              {loadError}
            </p>
          {:else if appStatus}
            <dl class="mt-4 space-y-3 text-sm">
              <div class="flex items-center justify-between">
                <dt class="text-sypher-muted">Vault</dt>
                <dd>{statusLabel[appStatus.vault]}</dd>
              </div>
              <div class="flex items-center justify-between">
                <dt class="text-sypher-muted">Data Plane</dt>
                <dd>{statusLabel[appStatus.dataPlane]}</dd>
              </div>
              <div class="flex items-center justify-between">
                <dt class="text-sypher-muted">Sessions</dt>
                <dd>{appStatus.activeSessions}</dd>
              </div>
              {#if preferences}
                <div class="flex items-center justify-between">
                  <dt class="text-sypher-muted">Theme</dt>
                  <dd>{statusLabel[preferences.theme]}</dd>
                </div>
              {/if}
            </dl>
          {:else}
            <p class="mt-4 text-sm text-sypher-muted">Loading status...</p>
          {/if}

          <div class="mt-6 grid grid-cols-2 gap-2">
            <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-2 text-xs text-sypher-muted">
              <div class="flex items-center justify-between gap-3">
                <span>Pane</span>
                <span class="truncate text-sypher-text">{activePaneStatus.state}</span>
              </div>
            </div>
            <TunnelIndicator activeCount={0} />
          </div>

          <div class="mt-6 border-t border-sypher-border pt-4">
            <h2 class="text-sm font-semibold">Connection</h2>

            {#if selectedProfile}
              <form class="mt-4 space-y-3" on:submit|preventDefault={connectSelectedProfile}>
                <div class="rounded-panel border border-sypher-border bg-sypher-surface p-3 text-sm">
                  <div class="flex items-center justify-between gap-3">
                    <span class="truncate font-medium">{selectedProfile.name}</span>
                    <span class="shrink-0 text-xs text-sypher-muted">{selectedProfile.host}:{selectedProfile.port}</span>
                  </div>
                </div>

                <label class="block text-xs text-sypher-muted">
                  Username
                  <input
                    class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                    bind:value={connectUsername}
                    autocomplete="username"
                    placeholder="deploy"
                  />
                </label>

                <div class="grid grid-cols-2 gap-2 rounded-panel border border-sypher-border bg-sypher-surface p-1">
                  <button
                    class={`rounded px-3 py-2 text-xs ${authMode === 'password' ? 'bg-sypher-accent text-sypher-bg' : 'text-sypher-muted hover:text-sypher-text'}`}
                    type="button"
                    on:click={() => (authMode = 'password')}
                  >
                    Password
                  </button>
                  <button
                    class={`rounded px-3 py-2 text-xs ${authMode === 'key' ? 'bg-sypher-accent text-sypher-bg' : 'text-sypher-muted hover:text-sypher-text'}`}
                    type="button"
                    on:click={() => (authMode = 'key')}
                  >
                    Key
                  </button>
                </div>

                {#if authMode === 'password'}
                  <label class="block text-xs text-sypher-muted">
                    Password
                    <input
                      class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                      bind:value={sshPassword}
                      type="password"
                      autocomplete="current-password"
                    />
                  </label>
                {:else}
                  <label class="block text-xs text-sypher-muted">
                    Private key path
                    <input
                      class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                      bind:value={privateKeyPath}
                      placeholder="C:/Users/me/.ssh/id_ed25519"
                    />
                  </label>
                  <label class="block text-xs text-sypher-muted">
                    Passphrase
                    <input
                      class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                      bind:value={passphrase}
                      type="password"
                      autocomplete="current-password"
                    />
                  </label>
                {/if}

                {#if connectionError}
                  <p class="rounded-panel border border-red-900/60 bg-red-950/40 p-2 text-xs text-red-200">
                    {connectionError}
                  </p>
                {/if}

                <button
                  class="w-full rounded-panel bg-sypher-accent px-3 py-2 text-sm font-semibold text-sypher-bg disabled:opacity-50"
                  type="submit"
                  disabled={isConnecting}
                >
                  {isConnecting ? 'Connecting...' : 'Connect'}
                </button>
              </form>
            {:else}
              <p class="mt-4 rounded-panel border border-sypher-border bg-sypher-surface p-3 text-sm text-sypher-muted">
                No profile selected
              </p>
            {/if}
          </div>

          <div class="mt-6 border-t border-sypher-border pt-4">
            <h2 class="text-sm font-semibold">Vault</h2>

            <div class="mt-3 rounded-panel border border-sypher-border bg-sypher-surface p-3">
              <div class="flex items-center justify-between text-sm">
                <span class="text-sypher-muted">State</span>
                <span>{vaultStatus ? statusLabel[vaultStatus.state] : 'Loading'}</span>
              </div>
              {#if vaultStatus?.version}
                <div class="mt-2 flex items-center justify-between text-sm">
                  <span class="text-sypher-muted">Version</span>
                  <span>{vaultStatus.version}</span>
                </div>
              {/if}
            </div>

            {#if vaultStatus?.state === 'missing'}
              <form class="mt-3 space-y-3" on:submit|preventDefault={submitCreateVault}>
                <label class="block text-xs text-sypher-muted">
                  Master password
                  <input
                    class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                    bind:value={masterPassword}
                    type="password"
                    autocomplete="new-password"
                    placeholder="At least 12 characters"
                  />
                </label>
                <button
                  class="w-full rounded-panel bg-sypher-accent px-3 py-2 text-sm font-semibold text-sypher-bg"
                  type="submit"
                >
                  Create vault
                </button>
              </form>
            {:else if vaultStatus?.state === 'locked'}
              <form class="mt-3 space-y-3" on:submit|preventDefault={submitUnlockVault}>
                <label class="block text-xs text-sypher-muted">
                  Master password
                  <input
                    class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                    bind:value={masterPassword}
                    type="password"
                    autocomplete="current-password"
                  />
                </label>
                <button
                  class="w-full rounded-panel bg-sypher-accent px-3 py-2 text-sm font-semibold text-sypher-bg"
                  type="submit"
                >
                  Unlock vault
                </button>
              </form>
            {:else if vaultStatus?.state === 'unlocked'}
              <div class="mt-3 space-y-3">
                <button
                  class="w-full rounded-panel border border-sypher-border px-3 py-2 text-sm text-sypher-text"
                  type="button"
                  on:click={submitLockVault}
                >
                  Lock vault
                </button>

                <form class="space-y-3" on:submit|preventDefault={submitChangeMasterPassword}>
                  <label class="block text-xs text-sypher-muted">
                    Current password
                    <input
                      class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                      bind:value={currentPassword}
                      type="password"
                      autocomplete="current-password"
                    />
                  </label>
                  <label class="block text-xs text-sypher-muted">
                    New password
                    <input
                      class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                      bind:value={newPassword}
                      type="password"
                      autocomplete="new-password"
                    />
                  </label>
                  <button
                    class="w-full rounded-panel bg-sypher-accent px-3 py-2 text-sm font-semibold text-sypher-bg"
                    type="submit"
                  >
                    Change password
                  </button>
                </form>
              </div>
            {/if}

            {#if vaultError}
              <p class="mt-3 rounded-panel border border-red-900/60 bg-red-950/40 p-2 text-xs text-red-200">
                {vaultError}
              </p>
            {:else if vaultMessage}
              <p class="mt-3 rounded-panel border border-emerald-900/60 bg-emerald-950/40 p-2 text-xs text-emerald-200">
                {vaultMessage}
              </p>
            {/if}
          </div>

          <div class="mt-6 border-t border-sypher-border pt-4">
            <div class="flex items-center justify-between">
              <h2 class="text-sm font-semibold">{editingProfileId ? 'Edit profile' : 'New profile'}</h2>
              <div class="flex gap-2">
                {#if editingProfileId}
                  <button
                    class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                    type="button"
                    on:click={resetProfileForm}
                  >
                    New
                  </button>
                {/if}
                <button
                  class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                  type="button"
                  on:click={cycleTheme}
                >
                  Theme
                </button>
              </div>
            </div>

            <form class="mt-4 space-y-3" on:submit|preventDefault={submitProfile}>
              <label class="block text-xs text-sypher-muted">
                Name
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileName}
                  placeholder="Production"
                />
              </label>

              <label class="block text-xs text-sypher-muted">
                Host
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileHost}
                  placeholder="example.com"
                />
              </label>

              <div class="grid grid-cols-[96px_1fr] gap-2">
                <label class="block text-xs text-sypher-muted">
                  Port
                  <input
                    class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                    bind:value={profilePort}
                    min="1"
                    max="65535"
                    type="number"
                  />
                </label>

                <label class="block text-xs text-sypher-muted">
                  Username
                  <input
                    class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                    bind:value={profileUsername}
                    placeholder="deploy"
                  />
                </label>
              </div>

              <label class="block text-xs text-sypher-muted">
                Group
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileGroup}
                  placeholder="Production"
                />
              </label>

              <label class="block text-xs text-sypher-muted">
                Tags
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileTags}
                  placeholder="prod, linux"
                />
              </label>

              <label class="block text-xs text-sypher-muted">
                Credential ref
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileCredentialRef}
                  placeholder="vault://profile-key"
                />
              </label>

              {#if formError}
                <p class="rounded-panel border border-red-900/60 bg-red-950/40 p-2 text-xs text-red-200">
                  {formError}
                </p>
              {/if}

              <button
                class="w-full rounded-panel bg-sypher-accent px-3 py-2 text-sm font-semibold text-sypher-bg"
                type="submit"
              >
                {editingProfileId ? 'Update profile' : 'Save profile'}
              </button>
            </form>
          </div>
        </aside>
      </div>
    </section>
  </section>
</main>
