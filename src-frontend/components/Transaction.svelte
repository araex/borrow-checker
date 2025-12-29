<script>
  export let expenseId;
  export let description;
  export let payerName;
  export let totalAmount;
  export let currency;
  export let date;
  export let userAmount;
  export let onClick;
</script>

<button
  class="expense-item group relative grid grid-cols-[1fr_auto_auto] items-center px-12 py-6 border-b border-zinc-700 cursor-pointer transition-colors hover:bg-zinc-900 w-full text-left"
  type="button"
  on:click={() => onClick(expenseId)}
>
  <!-- Hover background effect -->
  <div
    class="absolute inset-0 bg-gradient-to-r from-transparent to-white/[0.03] scale-x-0 origin-left transition-transform duration-600 ease-out group-hover:scale-x-100 pointer-events-none"
  ></div>

  <!-- Main info -->
  <div class="relative z-10">
    <h3 class="text-xl font-light mb-1">{description}</h3>
    <span class="font-mono text-xs text-gray-400 uppercase">
      Paid by: {payerName} • Total: {currency} {totalAmount.toFixed(2)} • {date}
    </span>
  </div>

  <!-- Status -->
  <div class="relative z-10 text-right mr-8 font-mono">
    {#if userAmount < 0}
      <span class="block text-[0.65rem] text-gray-500 uppercase">YOU BORROWED</span>
      <span class="text-lg text-red-500">
        -{currency} {Math.abs(userAmount).toFixed(2)}
      </span>
    {:else if userAmount > 0}
      <span class="block text-[0.65rem] text-gray-500 uppercase">YOU LENT</span>
      <span class="text-lg text-green-400">
        {currency} {userAmount.toFixed(2)}
      </span>
    {:else}
      <span class="block text-[0.65rem] text-gray-500 uppercase">SETTLED</span>
      <span class="text-lg text-gray-600">{currency} 0.00</span>
    {/if}
  </div>

  <!-- Chevron -->
  <div class="relative z-10">
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
      <path d="M9 18l6-6-6-6" />
    </svg>
  </div>
</button>
