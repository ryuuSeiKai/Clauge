<script lang="ts">
  import Modal from '$lib/shared/primitives/Modal.svelte';
  import type { SqlConnectionConfig, SqlDriver, SqlConnection } from '../types';
  import { sqlTestConnection } from '../commands';
  import { showToast } from '$lib/shared/primitives/toast';

  interface Props {
    show: boolean;
    editConnection?: SqlConnection | null;
    onsave?: (config: SqlConnectionConfig) => void;
    onclose?: () => void;
  }

  let { show = $bindable(false), editConnection = null, onsave, onclose }: Props = $props();

  const DEFAULT_PORTS: Record<SqlDriver, number> = {
    postgresql: 5432,
    mysql: 3306,
    sqlite: 0,
  };

  let name = $state('');
  let driver = $state<SqlDriver>('postgresql');
  let host = $state('localhost');
  let port = $state(5432);
  let database = $state('');
  let username = $state('');
  let password = $state('');
  let ssl = $state(false);
  let testing = $state(false);

  $effect(() => {
    if (show && editConnection) {
      name = editConnection.name;
      driver = editConnection.driver;
      host = editConnection.host;
      port = editConnection.port;
      database = editConnection.databaseName;
      username = editConnection.username;
      password = editConnection.password;
      ssl = !!editConnection.ssl;
    } else if (show && !editConnection) {
      name = '';
      driver = 'postgresql';
      host = 'localhost';
      port = 5432;
      database = '';
      username = '';
      password = '';
      ssl = false;
    }
  });

  function handleDriverChange(e: Event) {
    const newDriver = (e.target as HTMLSelectElement).value as SqlDriver;
    driver = newDriver;
    port = DEFAULT_PORTS[newDriver];
  }

  async function handleTest() {
    testing = true;
    try {
      const result = await sqlTestConnection({ name, driver, host, port, database, username, password, ssl });
      showToast(result || 'Connection successful', 'success');
    } catch (err: any) {
      showToast(err.toString(), 'error');
    } finally {
      testing = false;
    }
  }

  function handleSave() {
    if (!name.trim()) {
      showToast('Connection name is required', 'error');
      return;
    }
    onsave?.({ name: name.trim(), driver, host, port, database, username, password, ssl });
    show = false;
  }
</script>

<Modal bind:show title={editConnection ? 'Edit Connection' : 'New Connection'} width="460px" {onclose}>
  <div class="conn-form">
    <label class="conn-field">
      <span class="conn-label">Name</span>
      <input class="conn-input" type="text" bind:value={name} placeholder="My Database" />
    </label>

    <label class="conn-field">
      <span class="conn-label">Driver</span>
      <select class="conn-select" value={driver} onchange={handleDriverChange}>
        <option value="postgresql">PostgreSQL</option>
        <option value="mysql">MySQL</option>
        <option value="sqlite">SQLite</option>
      </select>
    </label>

    {#if driver !== 'sqlite'}
      <div class="conn-row">
        <label class="conn-field" style="flex:2">
          <span class="conn-label">Host</span>
          <input class="conn-input" type="text" bind:value={host} placeholder="localhost" />
        </label>
        <label class="conn-field" style="flex:1">
          <span class="conn-label">Port</span>
          <input class="conn-input" type="number" bind:value={port} />
        </label>
      </div>
    {/if}

    <label class="conn-field">
      <span class="conn-label">{driver === 'sqlite' ? 'File Path' : 'Database'}</span>
      <input class="conn-input" type="text" bind:value={database} placeholder={driver === 'sqlite' ? '/path/to/db.sqlite' : 'mydb'} />
    </label>

    {#if driver !== 'sqlite'}
      <div class="conn-row">
        <label class="conn-field" style="flex:1">
          <span class="conn-label">Username</span>
          <input class="conn-input" type="text" bind:value={username} placeholder="user" />
        </label>
        <label class="conn-field" style="flex:1">
          <span class="conn-label">Password</span>
          <input class="conn-input" type="password" bind:value={password} placeholder="password" />
        </label>
      </div>

      <label class="conn-check">
        <input type="checkbox" bind:checked={ssl} />
        <span>Use SSL</span>
      </label>
    {/if}

    <div class="conn-actions">
      <button class="conn-btn outline" onclick={handleTest} disabled={testing}>
        {testing ? 'Testing...' : 'Test Connection'}
      </button>
      <div style="flex:1"></div>
      <button class="conn-btn outline" onclick={() => show = false}>Cancel</button>
      <button class="conn-btn primary" onclick={handleSave}>Save</button>
    </div>
  </div>
</Modal>

<style>
  .conn-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .conn-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .conn-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--t2);
    font-family: var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .conn-input, .conn-select {
    height: 32px;
    background: var(--e);
    border: 1px solid var(--b1);
    border-radius: 6px;
    padding: 0 10px;
    font-size: 12.5px;
    font-family: var(--mono);
    color: var(--t1);
    outline: none;
    transition: border-color 0.15s;
  }
  .conn-input:focus, .conn-select:focus {
    border-color: var(--acc);
  }
  .conn-input::placeholder {
    color: var(--t3);
  }
  .conn-select {
    cursor: default;
    font-family: var(--ui);
  }
  .conn-row {
    display: flex;
    gap: 10px;
  }
  .conn-check {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: var(--t2);
    font-family: var(--ui);
    cursor: default;
  }
  .conn-check input {
    accent-color: var(--acc);
  }
  .conn-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--b1);
  }
  .conn-btn {
    height: 34px;
    padding: 0 20px;
    border-radius: 8px;
    font-size: 12px;
    font-family: var(--ui);
    cursor: default;
    transition: opacity 0.12s, border-color 0.12s, color 0.12s;
  }
  .conn-btn.outline {
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
  }
  .conn-btn.outline:hover:not(:disabled) {
    border-color: var(--b2);
    color: var(--t1);
  }
  .conn-btn.outline:disabled {
    opacity: 0.5;
  }
  .conn-btn.primary {
    border: none;
    background: var(--acc);
    color: #fff;
    font-weight: 600;
  }
  .conn-btn.primary:hover {
    opacity: 0.85;
  }
</style>
