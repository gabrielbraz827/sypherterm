<script lang="ts">
  import { onMount } from 'svelte';

  import {
    changeMasterPassword,
    createVault,
    deleteProfile,
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
      await loadRuntimeData();
    } catch (error) {
      loadError = error instanceof Error ? error.message : String(error);
    }
  });

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
  }

  async function refreshStatus() {
    appStatus = await getAppStatus();
    vaultStatus = { state: appStatus.vault, version: vaultStatus?.version };
    appStatusStore.set(appStatus);
    vaultStatusStore.set(vaultStatus);
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
      await saveProfile({
        name: profileName,
        host: profileHost,
        port: profilePort,
        username: profileUsername || undefined,
        tags: profileTags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
      });

      profileName = '';
      profileHost = '';
      profilePort = 22;
      profileUsername = '';
      profileTags = '';
      profiles = await listProfiles();
      profilesStore.set(profiles);
    } catch (error) {
      formError = formatError(error);
    }
  }

  async function removeProfile(id: string) {
    formError = '';

    try {
      await deleteProfile(id);
      profiles = await listProfiles();
      profilesStore.set(profiles);
    } catch (error) {
      formError = formatError(error);
    }
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
</script>

<main class="min-h-screen bg-sypher-bg text-sypher-text">
  <section class="grid min-h-screen grid-cols-[280px_1fr]">
    <aside class="border-r border-sypher-border bg-sypher-panel p-4">
      <div class="mb-6">
        <p class="text-xs uppercase text-sypher-muted">SypherTerm</p>
        <h1 class="mt-1 text-xl font-semibold">Local-first SSH</h1>
      </div>

      <div class="space-y-3">
        {#if profiles.length === 0}
          <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-muted">
            No profiles yet
          </div>
        {:else}
          {#each profiles as profile}
            <div class="rounded-panel border border-sypher-border bg-sypher-surface p-3">
              <div class="flex items-start justify-between gap-2">
                <div class="min-w-0">
                  <p class="truncate text-sm font-medium">{profile.name}</p>
                  <p class="truncate text-xs text-sypher-muted">{profile.username ? `${profile.username}@` : ''}{profile.host}:{profile.port}</p>
                </div>
                <button
                  class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                  type="button"
                  on:click={() => removeProfile(profile.id)}
                >
                  Delete
                </button>
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
          <p class="text-sm font-medium">App foundation</p>
          <p class="text-xs text-sypher-muted">Control Plane ready for real commands</p>
        </div>
        <div class="rounded-panel border border-sypher-border bg-sypher-surface px-3 py-1 text-xs text-sypher-muted">
          v{appStatus?.appVersion ?? '...'}
        </div>
      </header>

      <div class="grid flex-1 grid-cols-[1fr_360px] gap-4 p-5">
        <section class="flex min-h-0 flex-col rounded-panel border border-sypher-border bg-black font-mono">
          <div class="flex h-10 items-center border-b border-sypher-border px-3 text-xs text-sypher-muted">
            terminal preview
          </div>
          <div class="flex flex-1 items-center justify-center p-6 text-center">
            <div>
              <p class="text-sm text-sypher-text">Terminal engine is not connected yet.</p>
              <p class="mt-2 text-xs text-sypher-muted">
                SPEC-001 exposes app status; SSH and Data Plane arrive in later specs.
              </p>
            </div>
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
              <h2 class="text-sm font-semibold">Local profile</h2>
              <button
                class="rounded border border-sypher-border px-2 py-1 text-xs text-sypher-muted hover:text-sypher-text"
                type="button"
                on:click={cycleTheme}
              >
                Cycle theme
              </button>
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
                Tags
                <input
                  class="mt-1 w-full rounded border border-sypher-border bg-sypher-surface px-3 py-2 text-sm text-sypher-text outline-none focus:border-sypher-accent"
                  bind:value={profileTags}
                  placeholder="prod, linux"
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
                Save profile
              </button>
            </form>
          </div>
        </aside>
      </div>
    </section>
  </section>
</main>
