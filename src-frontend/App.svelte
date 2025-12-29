<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import Onboarding from './components/Onboarding.svelte';
  import EntitySelection from './components/EntitySelection.svelte';
  import MainView from './components/MainView.svelte';

  let appState = $state('loading'); // 'loading', 'onboarding', 'entity-selection', 'main', 'error'
  let errorMessage = $state('');

  onMount(async () => {
    console.log('App mounted, checking if window.__TAURI__ exists:', window.__TAURI__);
    
    // Wait for Tauri to be ready
    if (!window.__TAURI__) {
      console.error('Tauri API not available');
      errorMessage = 'Tauri API not available. Make sure you are running in Tauri environment.';
      appState = 'error';
      return;
    }

    try {
      console.log('Checking onboarding status...');
      
      const isOnboarded = await invoke('is_onboarded');
      console.log('Is onboarded:', isOnboarded);
      
      if (!isOnboarded) {
        appState = 'onboarding';
      } else {
        // Check if entity is selected by attempting to load main content
        try {
          console.log('Loading app state...');
          const state = await invoke('get_app_state');
          console.log('App state:', state);
          
          if (state && state.user_id) {
            appState = 'main';
          } else {
            appState = 'entity-selection';
          }
        } catch (e) {
          console.log('No entity selected yet:', e);
          appState = 'entity-selection';
        }
      }
    } catch (error) {
      console.error('Error checking onboarding status:', error);
      errorMessage = `Error: ${error}`;
      appState = 'onboarding';
    }
  });

  function handleOnboardingComplete() {
    appState = 'entity-selection';
  }

  function handleEntitySelected() {
    appState = 'main';
  }

  function handleUserReset() {
    console.log('App: User reset event received, changing to entity-selection');
    appState = 'entity-selection';
  }
</script>

<div class="bg-zinc-950 text-gray-200 min-h-screen flex justify-center items-center font-sans relative">
  <!-- Grain texture overlay -->
  <div class="fixed inset-0 pointer-events-none opacity-10 bg-[url('/noise.svg')] -z-10"></div>

  <div class="flex flex-col w-[calc(100vw-48px)] max-w-[1400px] h-[calc(100vh-48px)] bg-black border border-zinc-700 shadow-[20px_20px_60px_#050505,-5px_-5px_20px_rgba(255,255,255,0.02)] relative overflow-hidden">
    {#if appState === 'loading'}
      <div class="flex items-center justify-center h-full p-12">
        <p class="font-mono text-gray-600 text-sm">Loading...</p>
      </div>
    {:else if appState === 'error'}
      <div class="flex flex-col items-center justify-center h-full p-12 gap-4">
        <p class="font-mono text-red-500 text-sm">Error: {errorMessage}</p>
        <p class="font-mono text-gray-600 text-xs">Check the console for more details</p>
      </div>
    {:else if appState === 'onboarding'}
      <Onboarding onComplete={handleOnboardingComplete} />
    {:else if appState === 'entity-selection'}
      <EntitySelection onSelected={handleEntitySelected} />
    {:else if appState === 'main'}
      <MainView onUserReset={handleUserReset} />
    {/if}
  </div>
</div>
