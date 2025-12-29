<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();
  let entities = [];
  let newEntityName = '';
  let loading = false;
  let error = '';

  onMount(async () => {
    await loadEntities();
  });

  async function loadEntities() {
    try {
      const result = await invoke('get_entities');
      entities = result;
    } catch (e) {
      error = `Failed to load entities: ${e}`;
    }
  }

  async function selectEntity(entityId) {
    loading = true;
    error = '';

    try {
      await invoke('select_entity', { entityId });
      dispatch('selected');
    } catch (e) {
      error = `Failed to select entity: ${e}`;
    } finally {
      loading = false;
    }
  }

  async function addNewEntity() {
    if (!newEntityName.trim()) {
      error = 'Please enter a display name';
      return;
    }

    loading = true;
    error = '';

    try {
      await invoke('add_new_entity', { displayName: newEntityName });
      dispatch('selected');
    } catch (e) {
      error = `Failed to add entity: ${e}`;
    } finally {
      loading = false;
    }
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
        Select your identity
      </p>
    </div>

    <!-- Entity Selection -->
    <div class="card-elevated p-6 mb-6">
      <h2 class="text-xl font-semibold text-zinc-200 mb-3">
        Who are you?
      </h2>
      <p class="text-muted mb-4">
        Select your entity from the group or add a new one.
      </p>

      <!-- Existing Entities -->
      <div class="mb-6">
        <h3 class="text-sm font-medium text-zinc-300 mb-3">
          Existing Entities
        </h3>
        <div class="space-y-2">
          {#each entities as entity}
            <button
              type="button"
              class="w-full px-6 py-3 bg-orange-500 hover:bg-orange-600 text-white font-semibold rounded transition-colors disabled:opacity-50"
              on:click={() => selectEntity(entity.id)}
              disabled={loading}
            >
              {entity.display_name}
            </button>
          {/each}
        </div>
      </div>

      <!-- Add New Entity -->
      <div class="border-t border-zinc-700 pt-6">
        <h3 class="text-sm font-medium text-zinc-300 mb-3">
          Add New Entity
        </h3>
        <form on:submit|preventDefault={addNewEntity}>
          <input
            type="text"
            bind:value={newEntityName}
            placeholder="Enter display name"
            class="w-full bg-zinc-900 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none mb-4"
            disabled={loading}
          />
          <button
            type="submit"
            class="w-full px-6 py-3 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            disabled={loading}
          >
            Add New Entity and Continue
          </button>
        </form>
      </div>
    </div>

    {#if error}
      <div class="bg-red-950 border border-red-800 rounded-lg p-4">
        <div class="flex">
          <div class="flex-shrink-0">
            <svg class="h-5 w-5 text-red-400" fill="currentColor" viewBox="0 0 20 20">
              <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd" />
            </svg>
          </div>
          <div class="ml-3">
            <h3 class="text-sm font-medium text-red-300">Error</h3>
            <p class="mt-1 text-sm text-red-400">{error}</p>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>
