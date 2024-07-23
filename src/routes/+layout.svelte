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

<div id="main-view">
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
  <div class="flex">
    {@render children()}
  </div>
</div>

<style>
  nav {
    display: flex;
    justify-content: flex-end;
  }
  #main-view {
    @apply h-full max-h-dvh overflow-hidden;
    display: flex;
    flex-direction: column;
  }
</style>
