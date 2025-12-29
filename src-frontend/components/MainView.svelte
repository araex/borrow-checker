<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Header from './Header.svelte';
  import LedgerHeader from './LedgerHeader.svelte';
  import Transaction from './Transaction.svelte';
  import Settings from './Settings.svelte';
  import ExpenseForm from './ExpenseForm.svelte';

  let { onUserReset } = $props();

  let appState = $state(null);
  let transactions = $state([]);
  let showSettings = $state(false);
  let showExpenseForm = $state(false);
  let selectedExpenseId = $state(null);

  onMount(async () => {
    await loadAppState();
  });

  async function loadAppState() {
    try {
      appState = await invoke('get_app_state');
      await loadTransactions();
    } catch (e) {
      console.error('Failed to load app state:', e);
    }
  }

  async function loadTransactions() {
    try {
      transactions = await invoke('get_transactions');
    } catch (e) {
      console.error('Failed to load transactions:', e);
    }
  }

  async function handleLedgerChange(ledgerId) {
    try {
      console.log('Switching to ledger:', ledgerId);
      await invoke('switch_ledger', { ledgerId });
      console.log('Switch complete, reloading state...');
      await loadAppState();
      console.log('State reloaded. New transaction count:', transactions.length);
    } catch (e) {
      console.error('Failed to switch ledger:', e);
    }
  }

  function handleSettingsClick() {
    showSettings = true;
    showExpenseForm = false;
  }

  function handleTransactionClick(expenseId) {
    selectedExpenseId = expenseId;
    showExpenseForm = true;
    showSettings = false;
  }

  function handleAddExpense() {
    selectedExpenseId = null;
    showExpenseForm = true;
    showSettings = false;
  }

  function handleCloseView() {
    showSettings = false;
    showExpenseForm = false;
    selectedExpenseId = null;
  }

  async function handleExpenseSaved() {
    // Reload data after creating/updating expense
    await loadTransactions();
    handleCloseView();
  }

  async function handleExpenseDeleted() {
    // Reload data after deleting expense
    await loadTransactions();
    handleCloseView();
  }

  async function handleSettingsChange() {
    console.log('MainView: Settings reset event received, dispatching userReset');
    // User was reset, need to propagate to App to return to entity selection
    onUserReset();
  }
</script>

{#if appState}
  <div id="header">
    <Header
      currentUserName={appState.user_name}
      groupMembers={appState.group_members || []}
      onSettingsClick={handleSettingsClick}
    />
  </div>

  <main class="relative overflow-y-auto flex flex-col z-10">
    {#if showSettings}
      <Settings
        onClose={handleCloseView}
        onReset={handleSettingsChange}
      />
    {:else if showExpenseForm}
      <ExpenseForm
        expenseId={selectedExpenseId}
        onClose={handleCloseView}
        onSaved={handleExpenseSaved}
        onDeleted={handleExpenseDeleted}
      />
    {:else}
      <!-- Main Content -->
      <LedgerHeader
        ledgers={appState.ledgers || []}
        currentLedgerId={appState.current_ledger_id}
        balances={appState.balances || []}
        currency={appState.currency || 'USD'}
        onLedgerChange={handleLedgerChange}
      />
      
      <div class="flex-1">
        {#each transactions as transaction (transaction.expense_id)}
          <Transaction
            expenseId={transaction.expense_id}
            description={transaction.description}
            payerName={transaction.payer_name}
            totalAmount={transaction.total_amount}
            currency={transaction.currency}
            date={transaction.date}
            userAmount={transaction.user_amount}
            onClick={handleTransactionClick}
          />
        {/each}
      </div>

      <!-- Add Expense Button -->
      <button
        class="fixed bottom-8 right-8 w-14 h-14 bg-orange-500 hover:bg-orange-600 text-white rounded-full shadow-lg flex items-center justify-center transition-colors"
        onclick={handleAddExpense}
        title="Add Expense"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
        </svg>
      </button>
    {/if}
  </main>
{:else}
  <div class="flex items-center justify-center h-full">
    <p class="font-mono text-gray-600 text-sm">Loading...</p>
  </div>
{/if}
