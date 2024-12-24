<script lang="ts">
  import '../app.css'
  import { ModeWatcher, toggleMode } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { listen } from '@tauri-apps/api/event'
  import * as Sidebar from '$lib/components/ui/sidebar/index.js'
  import AppSidebar from '@components/app-sidebar.svelte'

  let { children } = $props()
  listen('toggle-theme', () => {
    toggleMode()
  })
</script>

<ModeWatcher />
<Toaster />
<Sidebar.Provider>
  <AppSidebar />
  <main id="main-view">
    <nav>
      <Sidebar.Trigger />
    </nav>
    <div class="flex">
      {@render children()}
    </div>
  </main>
</Sidebar.Provider>

<style>
  nav {
    display: flex;
    @apply p-2;
  }
  #main-view {
    @apply h-full max-h-dvh w-full overflow-hidden;
    display: flex;
    flex-direction: column;
  }
</style>
