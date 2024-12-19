<script lang="ts">
  import * as Sidebar from '$lib/components/ui/sidebar/index.js'
  import { AddCollectionDialog, AddRequestDialog } from '@components/dialogs'
  import { invoke } from '@tauri-apps/api/core'
  import { toast } from 'svelte-sonner'
  import type { CollectionConfig, Request } from '../tauriApi'
  import {
    Accordion,
    AccordionContent,
    AccordionItem,
    AccordionTrigger
  } from '@lib/components/ui/accordion'
  import { Method } from '@enums/methods'
  import { RadioGroup, RadioGroupItem } from '@lib/components/ui/radio-group'
  import { Label } from '@lib/components/ui/label'

  let collections: CollectionConfig[] = $state([])

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
    await invoke('create_collection', { collectionName: name, config: newCollectionToAdd })
    collections.push(newCollectionToAdd)
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
    collections = await invoke('get_collections')
    toast.success('Request created')
  }
</script>

<Sidebar.Root>
  <Sidebar.Header>
    <AddCollectionDialog onSend={createNewCollection} />
  </Sidebar.Header>
  <Sidebar.Content>
    {#if collections.length}
      <Sidebar.Group>
        <Accordion type="multiple">
          {#each collections as collection}
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
