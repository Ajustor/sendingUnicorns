<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { toast } from 'svelte-sonner'

  let { children } = $props()
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
