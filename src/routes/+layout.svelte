<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
</script>

<ModeWatcher />
<Toaster />

<nav></nav>

<div id="main-view">
  <aside class="h-full p-4">Here the collection list</aside>
  <slot></slot>
</div>

<style>
  #main-view {
    @apply h-full;
    display: flex;
    aside {
      background: red;
    }
  }
</style>
