<script>
  let { isOpen, onClose, svgContent } = $props();
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80" onclick={onClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-8 max-w-lg mx-4" onclick={(e) => e.stopPropagation()}>
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-2xl font-light text-zinc-200">Export Configuration</h2>
        <button
          class="p-2 text-zinc-400 hover:text-orange-500 hover:bg-zinc-800 rounded transition-colors"
          onclick={onClose}
          title="Close"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>

      <div class="bg-white p-4 rounded-lg mb-4">
        {@html svgContent}
      </div>

      <p class="text-muted text-sm mb-4">
        Scan this QR code with another Borrow Checker app to import your configuration and keys.
      </p>

      <div class="bg-orange-950 border border-orange-800 rounded-lg p-4">
        <p class="text-xs text-orange-400">
          <strong>Warning:</strong> This QR code contains your private SSH key. Only share it with devices you control and in a secure environment.
        </p>
      </div>
    </div>
  </div>
{/if}
