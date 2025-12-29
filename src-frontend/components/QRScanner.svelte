<script>
  import { onMount, onDestroy } from 'svelte';
  import { Html5Qrcode } from 'html5-qrcode';
  import { platform } from '@tauri-apps/plugin-os';

  let { isOpen, onClose, onScanSuccess } = $props();
  
  let html5QrCode = $state(null);
  let scanning = $state(false);
  let error = $state('');
  let isDesktop = $state(true); // Default to desktop
  let pastedContent = $state('');

  async function startScanning() {
    try {
      error = '';
      html5QrCode = new Html5Qrcode("qr-reader");
      
      await html5QrCode.start(
        { facingMode: "environment" }, // Use back camera
        {
          fps: 10,
          qrbox: { width: 250, height: 250 }
        },
        onScanSuccessHandler,
        onScanFailureHandler
      );
      
      scanning = true;
    } catch (err) {
      console.error('Failed to start QR scanner:', err);
      error = `Failed to start camera: ${err.message || err}`;
    }
  }

  function onScanSuccessHandler(decodedText, decodedResult) {
    console.log('QR code scanned:', decodedText);
    stopScanning();
    onScanSuccess(decodedText);
  }

  function onScanFailureHandler(errorMessage) {
    // This is called frequently as it scans, so we don't log it
  }

  async function stopScanning() {
    if (html5QrCode && scanning) {
      try {
        await html5QrCode.stop();
        html5QrCode.clear();
        scanning = false;
      } catch (err) {
        console.error('Error stopping scanner:', err);
      }
    }
  }

  function handleClose() {
    stopScanning();
    pastedContent = '';
    error = '';
    onClose();
  }

  function handlePaste() {
    if (!pastedContent.trim()) {
      error = 'Please paste content from the QR code';
      return;
    }
    
    try {
      // Process the pasted content
      onScanSuccess(pastedContent.trim());
      pastedContent = '';
    } catch (err) {
      error = `Failed to process content: ${err.message || err}`;
    }
  }

  onMount(async () => {
    // Detect platform
    const platformName = await platform();
    isDesktop = platformName !== 'android' && platformName !== 'ios';
    console.log('Platform detected:', platformName, 'isDesktop:', isDesktop);
  });

  onDestroy(() => {
    stopScanning();
  });

  $effect(() => {
    if (isOpen && !scanning && !isDesktop) {
      startScanning();
    } else if (!isOpen && scanning) {
      stopScanning();
    }
  });
</script>

{#if isOpen}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/80" onclick={handleClose}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="bg-zinc-900 border border-zinc-700 rounded-lg p-8 max-w-lg mx-4 w-full" onclick={(e) => e.stopPropagation()}>
      <div class="flex items-center justify-between mb-6">
        <h2 class="text-2xl font-light text-zinc-200">
          {isDesktop ? 'Import Configuration' : 'Scan QR Code'}
        </h2>
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

      {#if error}
        <div class="bg-red-950 border border-red-800 rounded-lg p-4 mb-4">
          <p class="text-sm text-red-400">{error}</p>
        </div>
      {/if}

      {#if isDesktop}
        <!-- Desktop: Paste Interface -->
        <div class="mb-4">
          <p class="text-muted text-sm mb-4">
            Scan the QR code with your phone's camera app, then paste the decoded content below:
          </p>
          <textarea
            bind:value={pastedContent}
            placeholder="Paste QR code content here..."
            class="w-full h-48 bg-zinc-800 border border-zinc-600 rounded-lg px-4 py-3 text-zinc-200 focus:border-orange-500 focus:outline-none resize-none font-mono text-sm"
          ></textarea>
        </div>
        <button
          onclick={handlePaste}
          class="w-full px-6 py-3 bg-orange-500 hover:bg-orange-600 text-white font-semibold rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={!pastedContent.trim()}
        >
          Import Configuration
        </button>
      {:else}
        <!-- Mobile: Camera Scanner -->
        <div id="qr-reader" class="rounded-lg overflow-hidden mb-4"></div>
        <p class="text-muted text-sm text-center">
          Position the QR code within the frame to scan
        </p>
      {/if}
    </div>
  </div>
{/if}
