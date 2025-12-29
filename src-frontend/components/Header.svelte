<script>
  let { 
    currentUserName = null,
    groupMembers = [],
    onSettingsClick
  } = $props();
</script>

<nav class="bg-zinc-900 px-6 py-6 flex justify-between items-center border-b border-zinc-700">
  <div class="brand">
    <h1 class="text-orange-500 font-bold tracking-[0.5rem] uppercase text-base">
      Borrow Checker
    </h1>
  </div>
  
  {#if currentUserName}
    <div class="flex items-center gap-6">
      {#if groupMembers.length > 0}
        <div class="flex items-center gap-3">
          <span class="text-xs text-gray-500 uppercase">Group:</span>
          <div class="flex gap-2">
            {#each groupMembers as member}
              <span class="px-2 py-1 bg-zinc-800 border border-zinc-700 rounded text-xs text-zinc-300">
                {member}
              </span>
            {/each}
          </div>
        </div>
      {/if}
      
      <div class="flex items-center gap-2">
        <span class="text-xs text-gray-500 uppercase">Logged in as:</span>
        <span class="text-orange-400 font-semibold text-sm">{currentUserName}</span>
      </div>
      
      <button
        class="p-2 text-zinc-400 hover:text-orange-500 hover:bg-zinc-800 rounded transition-colors"
        onclick={onSettingsClick}
        title="Settings"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
      </button>
    </div>
  {/if}
</nav>
