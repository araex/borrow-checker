<script>
    let {
        ledgers = [],
        currentLedgerId,
        balances = [],
        currency = "USD",
        onLedgerChange,
    } = $props();
</script>

<header
    class="px-6 py-6 flex justify-between items-start border-b border-zinc-700 bg-gradient-to-b from-zinc-900 to-transparent"
>
    <div class="title-group">
        <span class="font-mono text-xs text-orange-500 uppercase tracking-wide"
            >LEDGER</span
        >
        <div class="flex items-center gap-2">
            <select
                class="text-5xl font-light uppercase tracking-tight leading-tight bg-transparent text-white border-none outline-none cursor-pointer"
                style="-webkit-appearance: none; -moz-appearance: none; appearance: none; width: fit-content;"
                value={currentLedgerId}
                onchange={(e) => onLedgerChange(e.target.value)}
            >
                {#each ledgers as ledger}
                    <option value={ledger.id}>{ledger.name}</option>
                {/each}
            </select>
            <span class="pointer-events-none opacity-40">
                <svg
                    width="20"
                    height="20"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.5"
                >
                    <path d="M7 10l5 5 5-5" />
                </svg>
            </span>
        </div>
    </div>

    <div class="balance text-right">
        <span class="font-mono text-xs text-gray-500 uppercase block mb-3"
            >YOUR BALANCES</span
        >
        {#if balances.length === 0}
            <span class="font-mono text-xl text-gray-600">All settled up</span>
        {:else}
            <div class="space-y-2">
                {#each balances as balance}
                    {#if balance.amount < 0}
                        <div class="text-sm text-gray-300">
                            you owe {balance.user_name} <span class="text-red-500">{currency} {Math.abs(balance.amount).toFixed(2)}</span>
                        </div>
                    {:else if balance.amount > 0}
                        <div class="text-sm text-gray-300">
                            {balance.user_name} owes you <span class="text-green-400">{currency} {balance.amount.toFixed(2)}</span>
                        </div>
                    {:else}
                        <div class="text-sm text-gray-600">
                            settled with {balance.user_name}
                        </div>
                    {/if}
                {/each}
            </div>
        {/if}
    </div>
</header>
