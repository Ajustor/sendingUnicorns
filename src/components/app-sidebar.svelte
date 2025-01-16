<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar/index.js'
  import { AddCollectionDialog, AddRequestDialog } from '@components/dialogs'
  import { invoke } from '@tauri-apps/api/core'
  import { toast } from 'svelte-sonner'
  import { commands, type CollectionConfig, type Request } from '../tauriApi'
  import {
    Accordion,
    AccordionContent,
    AccordionItem,
    AccordionTrigger
  } from '@lib/components/ui/accordion'
  import { Method } from '@enums/methods'
  import { RadioGroup, RadioGroupItem } from '@lib/components/ui/radio-group'
  import { Label } from '@lib/components/ui/label'
  import { collectionsStore } from '../stores/collections.svelte'
  import { requestStore } from '../stores/request.svelte'

  let defaultRequest: Request = $state({
    name: 'New request',
    url: '',
    method: Method.GET,
    id: 'no-id',
    options: {
      body: { form_data: [], json: '' },
      headers: [],
      params: []
    },
    pre_request_script: null,
    test: null
  })

  const createNewCollection = async (name: string) => {
    const newCollectionToAdd: CollectionConfig = { name, requests: [], environments: [] }
    await commands.createCollection(name, newCollectionToAdd)
    collectionsStore.collections.push(newCollectionToAdd)
    toast.success('Collection created')
  }

  const createNewRequest = async (
    collection: CollectionConfig,
    name: string,
    url: string,
    method: Method
  ) => {
    collection.requests.push({ ...defaultRequest, name, url, method })
    await invoke('update_collection', { collectionName: collection.name, config: collection })
    collectionsStore.collections = await commands.getCollections()
    toast.success('Request created')
  }
  let selectedRequestId = $state('no-id')

  $effect(selectRequest)

  function selectRequest() {
    for (const collection of collectionsStore.collections) {
      const request = collection.requests.find(({ id }) => id === selectedRequestId)
      if (request) {
        requestStore.request = request
        return
      }
    }
    requestStore.request = defaultRequest
  }
</script>

<Sidebar.Root>
  <Sidebar.Header>
    <AddCollectionDialog onSend={createNewCollection} />
  </Sidebar.Header>
  <Sidebar.Content>
    {#if collectionsStore.collections?.length}
      <Sidebar.Group>
        <Accordion type="multiple">
          {#each collectionsStore.collections as collection}
            <AccordionItem value={collection.name}>
              <AccordionTrigger>{collection.name}</AccordionTrigger>
              <AccordionContent>
                <AddRequestDialog
                  onSend={(name: string, url: string, method: Method) =>
                    createNewRequest(collection, name, url, method)}
                />
                <RadioGroup class="p-4" bind:value={selectedRequestId}>
                  {#each collection.requests as request}
                    <div class="flex items-center space-x-2">
                      <RadioGroupItem value={request.id} id={request.name} />
                      <Label for={request.name}>{request.name}</Label>
                    </div>
                  {/each}
                </RadioGroup>
              </AccordionContent>
            </AccordionItem>
          {/each}
        </Accordion>
      </Sidebar.Group>
    {/if}
    <Sidebar.Group />
  </Sidebar.Content>
  <Sidebar.Footer />
</Sidebar.Root>
