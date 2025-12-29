<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let { onClose, onReset } = $props();
  
  let settings = $state(null);
  let loading = $state(true);
  let error = $state('');
  let copiedField = $state(null); // 'private_key' or 'public_key'

  onMount(async () => {
    console.log('Settings component mounted');
    await loadSettings();
  });

  async function loadSettings() {
    console.log('Loading settings...');
    try {
      loading = true;
      error = '';
      settings = await invoke('get_settings');
      console.log('Settings loaded:', settings);
    } catch (e) {
      error = `Failed to load settings: ${e}`;
      console.error(error);
    } finally {
      loading = false;
    }
  }

  async function handleResetUser() {
    console.log('Reset user button clicked - proceeding without confirmation');
    
    try {
      console.log('Calling reset_user command...');
      await invoke('reset_user');
      console.log('reset_user succeeded, dispatching reset event');
      // Trigger app to go back to entity selection
      onReset();
    } catch (e) {
      console.error('reset_user failed:', e);
      error = `Failed to reset user: ${e}`;
    }
  }

  async function copyToClipboard(text, fieldName) {
    try {
      await navigator.clipboard.writeText(text);
      console.log('Copied to clipboard:', text);
      copiedField = fieldName;
      setTimeout(() => {
        copiedField = null;
      }, 2000);
    } catch (e) {
      console.error('Failed to copy to clipboard:', e);
      error = `Failed to copy: ${e}`;
    }
  }

  function handleClose() {
    onClose();
  }
</script>

<div class="flex flex-col h-full p-6 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-light text-zinc-200">Settings</h2>
    <button
      class="p-2 text-zinc-400 hover:text-orange-500 hover:bg-zinc-800 rounded transition-colors"
      onclick={handleClose}
      title="Close"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <p class="font-mono text-gray-600 text-sm">Loading settings...</p>
    </div>
  {:else if error}
    <div class="bg-red-950 border border-red-800 rounded-lg p-4">
      <p class="text-sm text-red-400">{error}</p>
    </div>
  {:else if settings}
    <div class="flex flex-col gap-4">
      <!-- User Settings Section -->
      <div class="bg-zinc-900 border border-zinc-700 rounded-lg">
        <h3 class="text-lg font-semibold text-zinc-200 px-6 py-4 border-b border-zinc-700">User Information</h3>
        <div class="p-6 space-y-4">
          <div>
            <label for="user-name" class="block text-xs text-gray-500 uppercase mb-2">Display Name</label>
            <input
              id="user-name"
              type="text"
              value={settings.user_name}
              class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200"
              readonly
            />
          </div>
          <div>
            <label for="user-id" class="block text-xs text-gray-500 uppercase mb-2">User ID</label>
            <input
              id="user-id"
              type="text"
              value={settings.user_id}
              class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 font-mono text-sm"
              readonly
            />
          </div>
          <div class="pt-2">
            <button
              type="button"
              class="w-full px-6 py-3 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-semibold rounded transition-colors"
              onclick={() => {
                console.log('Button onclick fired');
                handleResetUser();
              }}
            >
              Select Different User
            </button>
          </div>
        </div>
      </div>

      <!-- Group Settings Section -->
      <div class="bg-zinc-900 border border-zinc-700 rounded-lg">
        <h3 class="text-lg font-semibold text-zinc-200 px-6 py-4 border-b border-zinc-700">Group Configuration</h3>
        <div class="p-6 space-y-4">
          <div>
            <label for="group-repo-url" class="block text-xs text-gray-500 uppercase mb-2">Group Repository URL</label>
            <input
              id="group-repo-url"
              type="text"
              value={settings.group_remote_url}
              class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 font-mono text-sm"
              readonly
            />
          </div>
          <div>
            <h4 class="text-sm font-medium text-zinc-300 mb-3">Group Members</h4>
            <div class="space-y-2">
              {#each settings.group_members as member}
                <div class="flex items-center justify-between px-4 py-2 bg-zinc-950 border border-zinc-700 rounded">
                  <span class="text-zinc-200">{member}</span>
                </div>
              {/each}
            </div>
          </div>
        </div>
      </div>

      <!-- Ledger Settings Section -->
      <div class="bg-zinc-900 border border-zinc-700 rounded-lg">
        <h3 class="text-lg font-semibold text-zinc-200 px-6 py-4 border-b border-zinc-700">Ledgers</h3>
        <div class="p-6 space-y-2">
          {#each settings.ledgers as ledger}
            <div class="flex items-center justify-between px-4 py-2 bg-zinc-950 border border-zinc-700 rounded">
              <span class="text-zinc-200">{ledger.name}</span>
              {#if ledger.is_current}
                <span class="px-2 py-1 bg-orange-500/20 border border-orange-500 rounded text-xs text-orange-400 font-semibold">
                  CURRENT
                </span>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <!-- SSH Keys Section -->
      <div class="bg-zinc-900 border border-zinc-700 rounded-lg">
        <h3 class="text-lg font-semibold text-zinc-200 px-6 py-4 border-b border-zinc-700">SSH Keys</h3>
        <div class="p-6 space-y-4">
          <div>
            <label for="private-key-path" class="block text-xs text-gray-500 uppercase mb-2">Private Key Location</label>
            <div class="flex gap-2">
              <input
                id="private-key-path"
                type="text"
                value={settings.ssh_private_key_path}
                class="flex-1 bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 font-mono text-sm"
                readonly
              />
              <button
                class="p-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-600 text-zinc-300 rounded transition-colors"
                onclick={() => copyToClipboard(settings.ssh_private_key_path, 'private_key')}
                title={copiedField === 'private_key' ? 'Copied!' : 'Copy to clipboard'}
              >
                {#if copiedField === 'private_key'}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                {/if}
              </button>
            </div>
          </div>
          <div>
            <label for="public-key" class="block text-xs text-gray-500 uppercase mb-2">Public Key</label>
            <div class="flex gap-2">
              <input
                id="public-key"
                type="text"
                value={settings.ssh_public_key}
                class="flex-1 bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 font-mono text-xs"
                readonly
              />
              <button
                class="p-2 bg-zinc-800 hover:bg-zinc-700 border border-zinc-600 text-zinc-300 rounded transition-colors"
                onclick={() => copyToClipboard(settings.ssh_public_key, 'public_key')}
                title={copiedField === 'public_key' ? 'Copied!' : 'Copy to clipboard'}
              >
                {#if copiedField === 'public_key'}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
                  </svg>
                {:else}
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z" />
                  </svg>
                {/if}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  {/if}
</div>
