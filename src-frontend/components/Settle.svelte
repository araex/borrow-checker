<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  let { onClose } = $props();

  let settlementData = $state(null);
  let isLoading = $state(true);
  let error = $state(null);
  
  // Currency conversion state
  let targetCurrency = $state('');
  let conversionRates = $state({});
  let isConverting = $state(false);
  let showConvertedResults = $state(false);

  onMount(async () => {
    await loadSettlement();
  });

  async function loadSettlement() {
    try {
      isLoading = true;
      error = null;
      settlementData = await invoke('get_settlement');
      
      // Initialize conversion rates object
      if (settlementData.currencies && settlementData.currencies.length > 0) {
        targetCurrency = settlementData.currencies[0].code;
        conversionRates = {};
        settlementData.currencies.forEach(currency => {
          conversionRates[currency.code] = '';
        });
      }
    } catch (e) {
      console.error('Failed to load settlement:', e);
      error = `Failed to load settlement: ${e}`;
    } finally {
      isLoading = false;
    }
  }

  async function handleConvert() {
    try {
      isConverting = true;
      error = null;
      
      // Build conversion input array
      const conversionInputs = Object.entries(conversionRates)
        .filter(([code, _]) => code !== targetCurrency)
        .map(([code, rate]) => ({
          currency_code: code,
          fixed_rate: rate !== '' ? parseFloat(rate) : null
        }));
      
      const result = await invoke('convert_settlement', {
        targetCurrency,
        conversionRates: conversionInputs
      });
      
      settlementData = result;
      showConvertedResults = true;
    } catch (e) {
      console.error('Failed to convert settlement:', e);
      error = `Failed to convert settlement: ${e}`;
    } finally {
      isConverting = false;
    }
  }

  function handleRateChange(currencyCode, value) {
    conversionRates[currencyCode] = value;
  }

  function resetConversion() {
    showConvertedResults = false;
    loadSettlement();
  }
</script>

<div class="flex flex-col h-full bg-black">
  <!-- Header -->
  <div class="flex items-center justify-between p-6 border-b border-zinc-700">
    <h2 class="text-xl font-semibold text-gray-200">Settle Up</h2>
    <button
      onclick={onClose}
      class="text-gray-400 hover:text-gray-200 transition-colors"
      title="Close"
    >
      <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>

  <div class="flex-1 overflow-y-auto p-6">
    {#if isLoading}
      <div class="flex items-center justify-center h-full">
        <p class="font-mono text-gray-600 text-sm">Loading settlement...</p>
      </div>
    {:else if error}
      <div class="flex items-center justify-center h-full">
        <p class="font-mono text-red-500 text-sm">{error}</p>
      </div>
    {:else if settlementData}
      <!-- Settlement Payments -->
      <div class="mb-8">
        <h3 class="text-lg font-semibold text-gray-200 mb-4">
          {showConvertedResults ? 'Optimized Payments (Converted)' : 'Optimized Payments'}
        </h3>
        
        {#if settlementData.payments && settlementData.payments.length > 0}
          <div class="space-y-3">
            {#each settlementData.payments as payment}
              <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-4">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <span class="text-gray-200 font-medium">{payment.from_name}</span>
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                    </svg>
                    <span class="text-gray-200 font-medium">{payment.to_name}</span>
                  </div>
                  <div class="text-right">
                    <span class="text-lg font-semibold text-orange-500">
                      {payment.amount.toFixed(2)} {payment.currency}
                    </span>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          <p class="text-gray-500 text-sm">All settled up! No payments needed.</p>
        {/if}
      </div>

      <!-- Currency Conversion Section -->
      {#if !showConvertedResults}
        <div class="mb-8">
          <h3 class="text-lg font-semibold text-gray-200 mb-4">Currency Conversion</h3>
          
          {#if settlementData.currencies && settlementData.currencies.length > 1}
            <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-4 space-y-4">
              <!-- Target Currency Selection -->
              <div>
                <label for="target-currency" class="block text-sm font-medium text-gray-400 mb-2">
                  Convert all to:
                </label>
                <select
                  id="target-currency"
                  bind:value={targetCurrency}
                  class="w-full bg-zinc-800 border border-zinc-600 rounded-lg px-4 py-2 text-gray-200 focus:outline-none focus:border-orange-500"
                >
                  {#each settlementData.currencies as currency}
                    <option value={currency.code}>{currency.code}</option>
                  {/each}
                </select>
              </div>

              <!-- Conversion Rates -->
              <div>
                <div class="block text-sm font-medium text-gray-400 mb-2">
                  Conversion Rates (optional - leave blank for automatic lookup)
                </div>
                <div class="space-y-2">
                  {#each settlementData.currencies as currency}
                    {#if currency.code !== targetCurrency}
                      <div class="flex items-center gap-3">
                        <span class="text-gray-300 w-16">{currency.code}:</span>
                        <input
                          type="number"
                          step="0.0001"
                          placeholder="Auto"
                          value={conversionRates[currency.code]}
                          oninput={(e) => handleRateChange(currency.code, e.target.value)}
                          class="flex-1 bg-zinc-800 border border-zinc-600 rounded-lg px-4 py-2 text-gray-200 placeholder-gray-600 focus:outline-none focus:border-orange-500"
                        />
                        <span class="text-gray-500 w-24">{targetCurrency}</span>
                      </div>
                    {/if}
                  {/each}
                </div>
              </div>

              <!-- Convert Button -->
              <button
                onclick={handleConvert}
                disabled={isConverting}
                class="w-full bg-orange-500 hover:bg-orange-600 disabled:bg-zinc-700 disabled:text-gray-500 text-white font-medium py-3 rounded-lg transition-colors"
              >
                {isConverting ? 'Converting...' : 'Convert & Recalculate'}
              </button>
            </div>
          {:else}
            <p class="text-gray-500 text-sm">All transactions use the same currency.</p>
          {/if}
        </div>
      {/if}

      <!-- Converted Transactions Summary -->
      {#if showConvertedResults && settlementData.converted_transactions && settlementData.converted_transactions.length > 0}
        <div class="mb-8">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-200">Converted Transactions</h3>
            <button
              onclick={resetConversion}
              class="text-sm text-orange-500 hover:text-orange-400 transition-colors"
            >
              Reset
            </button>
          </div>
          
          {#if settlementData.total_converted}
            <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-4 mb-4">
              <p class="text-sm text-gray-400">Total Volume (Converted):</p>
              <p class="text-xl font-semibold text-gray-200">
                {settlementData.total_converted.toFixed(2)} {settlementData.target_currency}
              </p>
            </div>
          {/if}
          
          <div class="space-y-2 max-h-96 overflow-y-auto">
            {#each settlementData.converted_transactions as transaction}
              <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-3">
                <div class="flex justify-between items-start mb-2">
                  <span class="text-gray-200 text-sm font-medium">{transaction.description}</span>
                  <span class="text-xs text-gray-500">{transaction.date.split('T')[0]}</span>
                </div>
                <div class="flex justify-between items-center text-xs">
                  <span class="text-gray-400">
                    {transaction.amount.toFixed(2)} {transaction.original_currency}
                  </span>
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-600" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                  </svg>
                  <span class="text-gray-300">
                    {transaction.converted_amount.toFixed(2)} {transaction.target_currency}
                  </span>
                  <span class="text-gray-500">
                    @ {transaction.conversion_rate.toFixed(4)}
                  </span>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Original Currencies Summary -->
      {#if !showConvertedResults && settlementData.currencies && settlementData.currencies.length > 0}
        <div>
          <h3 class="text-lg font-semibold text-gray-200 mb-4">Currencies Used</h3>
          <div class="grid grid-cols-2 gap-3">
            {#each settlementData.currencies as currency}
              <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-4">
                <p class="text-sm text-gray-400 mb-1">{currency.code}</p>
                <p class="text-lg font-semibold text-gray-200">
                  {currency.total_amount.toFixed(2)}
                </p>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
