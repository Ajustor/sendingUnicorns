<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog } from '../components/dialogs'

  const createNewCollection = (name: string) => {
    console.log('create new collection', { name })
  }
</script>

<ModeWatcher defaultMode="system" />
<Toaster />

<nav></nav>

<div id="main-view">
  <aside class="border-right h-full p-4">
    <AddCollectionDialog onSend={createNewCollection} />
    <ScrollArea class="collection-list rounded-md border"></ScrollArea>
  </aside>
  <slot></slot>
</div>

<style>
  #main-view {
    @apply h-full;
    display: flex;
  }
</style>
