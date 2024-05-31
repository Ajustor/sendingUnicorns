<script lang="ts">
  import { Input } from '@lib/components/ui/input'
  import { Label } from '$lib/components/ui/label'
  import { Button } from '$lib/components/ui/button'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog, AddRequestDialog } from '../components/dialogs'
  import { setContext } from 'svelte'
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
  import type { Collection } from '@types/collection.type'
  import type { Request } from '@types/request.type'

  type Props = {
    headerTitle: string
  }

  let { headerTitle }: Props = $props()

  let collections = $state([])
  let selectedRequest: Request = $state({ name: 'nouvelle requête', url: '', method: Method.GET })

  const createNewCollection = async (name: string) => {
    console.log('create new collection', { name })
    const newCollectionToAdd = { name, requests: [] }
    await invoke('create_collection', { collectionName: name, config: newCollectionToAdd })
    collections.push(newCollectionToAdd)
  }

  const createNewRequest = async (
    collection: Collection,
    name: string,
    url: string,
    method: Method
  ) => {
    collection.requests.push({ name, url, method })
    await invoke('update_collection', { collectionName: collection.name, config: collection })
  }

  const getCollections = async () => {
    collections = await invoke('get_collections')
  }

  let sendRequestPromise: Promise<string> = $state('')
  const sendRequest = async () => {
    sendRequestPromise = invoke('make_api_call', selectedRequest)
  }
</script>

{#await getCollections() then _}
  <aside class="h-full border-r p-4">
    <AddCollectionDialog onSend={createNewCollection} />
    {#if collections.length}
      <!-- content here -->
      <ScrollArea class="collection-list rounded-md border">
        <Accordion>
          {#each collections as collection}
            <AccordionItem value={collection.name}>
              <AccordionTrigger>{collection.name}</AccordionTrigger>
              <AccordionContent>
                <AddRequestDialog
                  onSend={(name: string, url: string, method: Method) => createNewRequest(collection, name, url, method)}
                />
                <RadioGroup bind:value={selectedRequest}>
                  {#each collection.requests as request}
                    <div class="flex items-center space-x-2">
                      <RadioGroupItem value={request} id={request.name} />
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
  <div class="grid grid-cols-5 p-2">
    <Select
      class=""
      onSelectedChange={({ value }) => {
        value && (selectedRequest.method = value)
      }}
    >
      <SelectTrigger class="col-span-3">
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
    <Input
      class="col-span-3 col-start-2"
      placeholder="url"
      bind:value={selectedRequest.url}
      type="url"
    />
    <Button class="" onclick={sendRequest}>Envoyer</Button>
  </div>
  {#await sendRequestPromise}
    <!-- promise is pending -->
    sending unicorns...
  {:then result}
    <!-- promise was fulfilled -->
    <div class="overflow-hidden">
      {@html result}
    </div>
  {:catch error}
    <!-- promise was rejected -->
    {error}
  {/await}
</div>

<style>
  #main {
    display: flex;
    flex-direction: column;
  }
</style>
