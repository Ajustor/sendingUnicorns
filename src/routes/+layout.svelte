<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  // import { invoke } from '@tauri-apps/api/core'
  import { toast } from 'svelte-sonner'

  let { children } = $props()
  // document.addEventListener('DOMContentLoaded', () => {
  //   // This will wait for the window to load, but you could
  //   // run this function on whatever trigger you want
  //   invoke('close_splashscreen')
  // })
</script>

<ModeWatcher />
<Toaster />

<nav></nav>
<div id="main-view">
  {@render children()}
</div>

<style>
  #main-view {
    @apply h-full;
    display: flex;
  }
</style>
