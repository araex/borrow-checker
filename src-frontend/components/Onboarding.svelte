<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();
  let sshPublicKey = '';
  let repoUrl = '';
  let loading = false;
  let error = '';

  onMount(async () => {
    try {
      sshPublicKey = await invoke('get_ssh_public_key');
    } catch (e) {
      error = `Failed to load SSH key: ${e}`;
    }
  });

  async function handleJoinGroup() {
    if (!repoUrl) {
      error = 'Please enter a repository URL';
      return;
    }

    loading = true;
    error = '';

    try {
      await invoke('join_group', { url: repoUrl });
      dispatch('complete');
    } catch (e) {
      error = `Failed to join group: ${e}`;
    } finally {
      loading = false;
    }
  }

  function copyToClipboard() {
    navigator.clipboard.writeText(sshPublicKey);
  }
</script>

<div class="min-h-full flex items-start justify-center overflow-y-auto">
  <div class="w-full max-w-2xl px-6 py-12">
    <!-- Header -->
    <div class="mb-6 text-center">
      <h1 class="text-orange-500 font-bold tracking-[0.5rem] uppercase text-4xl">
        Borrow Checker
      </h1>
      <p class="text-lg text-zinc-400 mb-2">
        Join a group by connecting to a repository
      </p>
    </div>

    <!-- SSH Key Display -->
    <div class="card-elevated p-6 mb-6">
      <h2 class="text-xl font-semibold text-zinc-200 mb-3">
        Step 1: Add SSH Key
      </h2>
      <p class="text-muted mb-4">
        Copy the SSH public key of your Borrow Checker app and add it to your Git provider with
        <strong class="text-zinc-200">write access</strong>.
      </p>

      <div class="relative mb-4">
        <div class="flex gap-2 items-stretch">
          <pre
            class="flex-1 bg-black border border-zinc-700 p-4 rounded text-xs overflow-x-auto font-mono text-zinc-300"
          >{sshPublicKey}</pre>
          <button
            class="px-3 bg-zinc-800 border border-zinc-700 text-zinc-400 hover:bg-zinc-700 hover:text-orange-500 hover:border-orange-500 rounded transition-colors flex-shrink-0"
            title="Copy to clipboard"
            on:click={copyToClipboard}
          >
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 16H6a2 2 0 01-2-2V6a2 2 0 012-2h8a2 2 0 012 2v2m-6 12h8a2 2 0 002-2v-8a2 2 0 00-2-2h-8a2 2 0 00-2 2v8a2 2 0 002 2z"></path>
            </svg>
          </button>
        </div>
      </div>
    </div>

    <!-- Repository URL -->
    <div class="card-elevated p-6">
      <h2 class="text-xl font-semibold text-zinc-200 mb-3">
        Step 2: Connect Repository
      </h2>
      <p class="text-muted mb-4">
        Enter the SSH URL of your group's repository.
      </p>

      <form on:submit|preventDefault={handleJoinGroup}>
        <input
          type="text"
          bind:value={repoUrl}
          placeholder="git@github.com:user/repo.git"
          class="w-full bg-zinc-900 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none mb-4"
          disabled={loading}
        />

        <button
          type="submit"
          class="w-full px-6 py-3 bg-orange-500 hover:bg-orange-600 text-white font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={loading}
        >
          {loading ? 'Connecting...' : 'Join Group'}
        </button>
      </form>

      {#if error}
        <div class="mt-4 bg-red-950 border border-red-800 rounded p-4">
          <p class="text-sm text-red-400">{error}</p>
        </div>
      {/if}
    </div>
  </div>
</div>
