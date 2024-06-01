<script lang="ts">
  import { Input } from '@lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Button } from '$lib/components/ui/button'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog, AddRequestDialog } from '@components/dialogs'
  import { Result } from '@components/requestResult'
  import { RequestConfig } from '@components/forms'
  import {
    Accordion,
    AccordionItem,
    AccordionTrigger,
    AccordionContent
  } from '@lib/components/ui/accordion'
  import {
    Select,
    SelectTrigger,
    SelectContent,
    SelectItem,
    SelectGroup,
    SelectValue
  } from '@lib/components/ui/select'
  import { RadioGroup, RadioGroupItem } from '@lib/components/ui/radio-group'
  import { invoke } from '@tauri-apps/api/core'
  import { Method } from '@enums/methods'
  import type { Collection } from '../types/collection.type'
  import type { Request } from '../types/request.type'
  import { Send } from 'lucide-svelte'
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@lib/components/ui/tabs'
  import { ResizableHandle, ResizablePane, ResizablePaneGroup } from '@lib/components/ui/resizable'

  const defaultRequest = {
    name: 'nouvelle requête',
    url: '',
    method: Method.GET,
    id: 'no-id'
  }

  let collections: Collection[] = $state([])
  let selectedRequestId = $state('no-id')
  let selectedRequest: Request = $derived(selectRequest())

  let selectedMethod = $derived(
    selectedRequest.method
      ? {
          label: selectedRequest.method.toUpperCase(),
          value: selectedRequest.method
        }
      : undefined
  )

  function selectRequest() {
    for (const collection of collections) {
      const request = collection.requests.find(({ id }) => id === selectedRequestId)
      if (request) {
        return request
      }
    }
    return defaultRequest
  }

  const createNewCollection = async (name: string) => {
    const newCollectionToAdd: Collection = { name, requests: [] }
    await invoke('create_collection', { collectionName: name, config: newCollectionToAdd })
    collections.push(newCollectionToAdd)
  }

  const createNewRequest = async (
    collection: Collection,
    name: string,
    url: string,
    method: Method
  ) => {
    collection.requests.push({ name, url, method, id: '' })
    await invoke('update_collection', { collectionName: collection.name, config: collection })
    collections = await invoke('get_collections')
  }

  const getCollections = async () => {
    collections = await invoke('get_collections')
  }

  let sendRequestPromise: Promise<string> = $state(Promise.resolve(''))
  const sendRequest = async () => {
    sendRequestPromise = invoke('make_api_call', selectedRequest)
  }
</script>

{#snippet configView()}
  <RequestConfig />
{/snippet}

{#snippet resultView()}
  {#await sendRequestPromise}
    <!-- promise is pending -->
    sending unicorns...
  {:then result}
    <!-- promise was fulfilled -->
    <Result {result} />
  {:catch error}
    <!-- promise was rejected -->
    {error}
  {/await}{/snippet}

{#await getCollections() then _}
  <aside class="h-full max-w-md border-r p-4">
    <AddCollectionDialog onSend={createNewCollection} />
    {#if collections.length}
      <!-- content here -->
      <ScrollArea class="collection-list rounded-md border p-4">
        <Accordion>
          {#each collections as collection}
            <AccordionItem value={collection.name}>
              <AccordionTrigger>{collection.name}</AccordionTrigger>
              <AccordionContent>
                <AddRequestDialog
                  onSend={(name: string, url: string, method: Method) => createNewRequest(collection, name, url, method)}
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
      </ScrollArea>
    {/if}
  </aside>
{/await}

<div id="main" class="h-full w-full p-4">
  <h1>{selectedRequest.name}</h1>
  <div class="grid grid-cols-5 gap-2 p-2">
    <Select
      selected={selectedMethod}
      onSelectedChange={(v) => {
        v && (selectedRequest.method = v.value)
      }}
    >
      <SelectTrigger class="col-span-1">
        <SelectValue placeholder="Sélectionnez la méthode" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {#each Object.entries(Method) as [key, value]}
            <SelectItem {value} label={key}>{key}</SelectItem>
          {/each}
        </SelectGroup>
      </SelectContent>
    </Select>
    <Input class="col-span-3" placeholder="url" bind:value={selectedRequest.url} type="url" />
    <Button class="col-span-1 gap-2" onclick={sendRequest}>Envoyer <Send /></Button>
  </div>

  <Tabs class="h-dvh w-full">
    <TabsList>
      <TabsTrigger value="config">Configuration</TabsTrigger>
      <TabsTrigger value="combo">Vue scindée</TabsTrigger>
      <TabsTrigger value="result">Résultats</TabsTrigger>
    </TabsList>
    <TabsContent value="config">
      {@render configView()}
    </TabsContent>
    <TabsContent value="combo">
      <ResizablePaneGroup direction="horizontal" class="max-w">
        <ResizablePane defaultSize={30}>{@render configView()}</ResizablePane>
        <ResizableHandle withHandle />
        <ResizablePane defaultSize={70}>
          <div class="p-2">
            {@render resultView()}
          </div>
        </ResizablePane>
      </ResizablePaneGroup>
    </TabsContent>
    <TabsContent class="h-full" value="result">
      {@render resultView()}
    </TabsContent>
  </Tabs>
</div>

<style>
  #main {
    display: flex;
    flex-direction: column;
    @apply overflow-hidden;
  }
</style>
