<script lang="ts">
  import '../app.css'
  import { ModeWatcher, toggleMode, mode } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { Moon, Sun } from 'lucide-svelte'
  import { Button } from '@lib/components/ui/button'

  let { children } = $props()
</script>

<ModeWatcher />
<Toaster />

<nav>
  <Button title="Changer le thème" onclick={toggleMode}>
    {#if $mode === 'light'}
      <!-- content here -->
      <Sun />
    {:else}
      <Moon />
    {/if}
  </Button>
</nav>
<div id="main-view">
  {@render children()}
</div>

<style>
  nav {
    display: flex;
    justify-content: flex-end;
  }
  #main-view {
    @apply h-full;
    display: flex;
  }
</style>
