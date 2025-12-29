<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let { 
    expenseId = null,  // null for new expense, string for edit
    onClose,
    onSaved,
    onDeleted
  } = $props();
  
  let loading = $state(true);
  let saving = $state(false);
  let error = $state('');
  
  // Form fields
  let description = $state('');
  let amount = $state(0);
  let currency = $state('USD');
  let paidBy = $state('');
  let date = $state(new Date().toISOString().split('T')[0]);
  let participants = $state([]);
  let splitConfig = $state({}); // { entityId: { included: bool, numerator: number, denominator: number } }

  onMount(async () => {
    await loadFormData();
  });

  async function loadFormData() {
    try {
      loading = true;
      error = '';
      
      if (expenseId) {
        // Load existing expense
        const expense = await invoke('get_expense', { expenseId });
        description = expense.description;
        amount = expense.amount;
        currency = expense.currency;
        paidBy = expense.paid_by;
        date = expense.date.split('T')[0]; // Extract date part
        participants = expense.participants;
        
        // Set up split config
        splitConfig = {};
        for (const participant of participants) {
          const split = expense.split_ratios.find(s => s.entity_id === participant.id);
          if (split) {
            splitConfig[participant.id] = {
              included: true,
              numerator: split.numerator,
              denominator: split.denominator,
            };
          } else {
            splitConfig[participant.id] = {
              included: false,
              numerator: 0,
              denominator: 1,
            };
          }
        }
      } else {
        // New expense - get participants from app state
        const appState = await invoke('get_app_state');
        participants = appState.group_members.map((name, index) => ({
          id: '', // Will be filled when we have full entity data
          name,
        }));
        
        // Get full entity data
        const entities = await invoke('get_entities');
        participants = entities;
        
        // Default to current user as payer
        paidBy = appState.user_id || (participants.length > 0 ? participants[0].id : '');
        
        // Default: all participants included with equal split
        splitConfig = {};
        for (const participant of participants) {
          splitConfig[participant.id] = {
            included: true,
            numerator: 1,
            denominator: participants.length,
          };
        }
      }
    } catch (e) {
      error = `Failed to load form data: ${e}`;
      console.error(error);
    } finally {
      loading = false;
    }
  }

  function adjustSplitRatios() {
    const includedCount = Object.values(splitConfig).filter(s => s.included).length;
    
    if (includedCount === 0) return;
    
    // Update ratios for all participants
    for (const [entityId, config] of Object.entries(splitConfig)) {
      if (config.included) {
        splitConfig[entityId] = {
          included: true,
          numerator: 1,
          denominator: includedCount,
        };
      } else {
        splitConfig[entityId] = {
          included: false,
          numerator: 0,
          denominator: 1,
        };
      }
    }
  }

  async function handleSubmit() {
    try {
      saving = true;
      error = '';
      
      // Validate
      if (!description.trim()) {
        error = 'Description is required';
        return;
      }
      if (amount <= 0) {
        error = 'Amount must be greater than 0';
        return;
      }
      if (!paidBy) {
        error = 'Please select who paid';
        return;
      }
      
      // Build split ratios array
      const splitRatios = Object.entries(splitConfig)
        .filter(([_, config]) => config.included)
        .map(([entityId, config]) => ({
          entity_id: entityId,
          numerator: config.numerator,
          denominator: config.denominator,
        }));
      
      if (splitRatios.length === 0) {
        error = 'At least one participant must be included in the split';
        return;
      }
      
      if (expenseId) {
        // Update existing
        await invoke('update_expense', {
          expenseId,
          description,
          amount: parseFloat(amount),
          currency,
          paidBy,
          date,
          splitRatios,
        });
      } else {
        // Create new
        await invoke('create_expense', {
          description,
          amount: parseFloat(amount),
          currency,
          paidBy,
          date,
          splitRatios,
        });
      }
      
      onSaved();
    } catch (e) {
      error = `Failed to save expense: ${e}`;
      console.error(error);
    } finally {
      saving = false;
    }
  }

  async function handleDelete() {
    if (!expenseId) return;
    
    if (!confirm('Are you sure you want to delete this expense?')) {
      return;
    }
    
    try {
      saving = true;
      error = '';
      await invoke('delete_expense', { expenseId });
      onDeleted();
    } catch (e) {
      error = `Failed to delete expense: ${e}`;
      console.error(error);
    } finally {
      saving = false;
    }
  }

  function handleClose() {
    onClose();
  }
</script>

<div class="flex flex-col h-full p-6 gap-4 overflow-y-auto">
  <div class="flex items-center justify-between">
    <h2 class="text-2xl font-light text-zinc-200">
      {expenseId ? 'Edit Expense' : 'Add Expense'}
    </h2>
    <button
      class="p-2 text-zinc-400 hover:text-orange-500 hover:bg-zinc-800 rounded transition-colors"
      onclick={handleClose}
      disabled={saving}
      title="Close"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <p class="font-mono text-gray-600 text-sm">Loading...</p>
    </div>
  {:else}
    <form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }} class="flex flex-col gap-4">
      {#if error}
        <div class="bg-red-950 border border-red-800 rounded-lg p-4">
          <p class="text-sm text-red-400">{error}</p>
        </div>
      {/if}

      <!-- Expense Info Card -->
      <div class="bg-zinc-900 border border-zinc-700 rounded-lg">
        <h3 class="text-lg font-semibold text-zinc-200 px-6 py-4 border-b border-zinc-700">Expense Info</h3>
        <div class="p-6 space-y-4">
          <!-- Description -->
          <div>
            <label for="description" class="block text-xs text-gray-500 uppercase mb-2">Description</label>
            <input
              type="text"
              id="description"
              bind:value={description}
              required
              disabled={saving}
              class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none"
            />
          </div>

          <!-- Amount and Currency -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label for="amount" class="block text-xs text-gray-500 uppercase mb-2">Amount</label>
              <input
                type="number"
                id="amount"
                bind:value={amount}
                step="0.01"
                min="0"
                required
                disabled={saving}
                class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none"
              />
            </div>

            <div>
              <label for="currency" class="block text-xs text-gray-500 uppercase mb-2">Currency</label>
              <select
                id="currency"
                bind:value={currency}
                required
                disabled={saving}
                class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none"
              >
                <option value="USD" class="bg-zinc-950 text-zinc-200">USD</option>
                <option value="EUR" class="bg-zinc-950 text-zinc-200">EUR</option>
                <option value="GBP" class="bg-zinc-950 text-zinc-200">GBP</option>
                <option value="CHF" class="bg-zinc-950 text-zinc-200">CHF</option>
                <option value="JPY" class="bg-zinc-950 text-zinc-200">JPY</option>
              </select>
            </div>
          </div>

          <!-- Paid By and Date -->
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label for="paid_by" class="block text-xs text-gray-500 uppercase mb-2">Paid By</label>
              <select
                id="paid_by"
                bind:value={paidBy}
                required
                disabled={saving}
                class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none"
              >
                {#each participants as participant}
                  <option value={participant.id} class="bg-zinc-950 text-zinc-200">{participant.display_name}</option>
                {/each}
              </select>
            </div>

            <div>
              <label for="date" class="block text-xs text-gray-500 uppercase mb-2">Date</label>
              <input
                type="date"
                id="date"
                bind:value={date}
                required
                disabled={saving}
                class="w-full bg-zinc-950 border border-zinc-600 rounded px-4 py-2 text-zinc-200 focus:border-orange-500 focus:outline-none"
              />
            </div>
          </div>
        </div>
      </div>

      <!-- Split Card -->
      <details class="bg-zinc-900 border border-zinc-700 rounded-lg" open>
        <summary class="cursor-pointer hover:bg-zinc-800/50 rounded-lg transition-colors list-none px-6 py-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-semibold text-zinc-200">Split</h3>
            <svg class="w-5 h-5 text-zinc-400 transition-transform details-arrow" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
            </svg>
          </div>
        </summary>
        <div class="px-6 pb-6 space-y-3">
          {#each participants as participant}
            <div class="flex items-center justify-between px-4 py-2 bg-zinc-950 border border-zinc-700 rounded">
              <div class="flex items-center gap-4">
                <input
                  type="checkbox"
                  id="split_include_{participant.id}"
                  bind:checked={splitConfig[participant.id].included}
                  onchange={adjustSplitRatios}
                  disabled={saving}
                  class="w-4 h-4 accent-orange-500"
                />
                <label for="split_include_{participant.id}" class="text-zinc-200">
                  {participant.display_name}
                </label>
              </div>
              <input
                type="text"
                bind:value={splitConfig[participant.id].numerator}
                placeholder="1"
                disabled={saving}
                class="w-16 bg-zinc-900 border border-zinc-600 rounded px-3 py-1 text-zinc-200 text-sm text-center focus:border-orange-500 focus:outline-none"
              />
              <span class="text-zinc-500 mx-1">/</span>
              <input
                type="text"
                bind:value={splitConfig[participant.id].denominator}
                placeholder="1"
                disabled={saving}
                class="w-16 bg-zinc-900 border border-zinc-600 rounded px-3 py-1 text-zinc-200 text-sm text-center focus:border-orange-500 focus:outline-none"
              />
            </div>
          {/each}
        </div>
      </details>

      <!-- Action buttons -->
      <div class="flex gap-3 pt-4">
        <button
          type="submit"
          disabled={saving}
          class="flex-1 px-6 py-3 bg-orange-500 hover:bg-orange-600 text-white font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {saving ? 'Saving...' : (expenseId ? 'Update' : 'Create')}
        </button>
        
        {#if expenseId}
          <button
            type="button"
            onclick={handleDelete}
            disabled={saving}
            class="px-6 py-3 bg-red-600 hover:bg-red-700 text-white font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Delete
          </button>
        {/if}
      </div>
    </form>
  {/if}
</div>
