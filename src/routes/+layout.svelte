<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog } from '../components/dialogs'
  import { setContext } from 'svelte'
  import { Accordion } from '@lib/components/ui/accordion'
  import { invoke } from '@tauri-apps/api/core'

  let { children } = $props()

  let collections = $state([])

  const createNewCollection = async (name: string) => {
    console.log('create new collection', { name })
    await invoke('create_collection', { collectionName: name, config: { url: '', method: 'get' } })
    collections.push({ collectionName: name, config: { url: '', method: 'get' } })
  }

  const getCollections = async () => {
    collections = await invoke('get_collections')
  }
</script>

<ModeWatcher />
<Toaster />

<nav></nav>
{#await getCollections() then _}
  <!-- promise was fulfilled -->
  <div id="main-view">
    <aside class="h-full border-r p-4">
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
{/await}

<style>
  #main-view {
    @apply h-full;
    display: flex;
  }
</style>
