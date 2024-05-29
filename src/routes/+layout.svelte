<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog } from '../components/dialogs'
  import { setContext } from 'svelte'
  import { Accordion } from '@lib/components/ui/accordion'

  let { children } = $props()

  const collections = $state([])

  const createNewCollection = (name: string) => {
    console.log('create new collection', { name })
  }
</script>

<ModeWatcher />
<Toaster />

<nav></nav>

<div id="main-view">
  <aside class="h-full w-1/6 border-r p-4">
    <AddCollectionDialog onSend={createNewCollection} />
    {#if collections.length}
      <!-- content here -->
      <ScrollArea class="collection-list rounded-md border">
        <Accordion></Accordion>
      </ScrollArea>
    {/if}
  </aside>
  {@render children()}
</div>

<style>
  #main-view {
    @apply h-full;
    display: flex;
  }
</style>
