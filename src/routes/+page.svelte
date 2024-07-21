<script lang="ts">
  import { Input } from '@lib/components/ui/input'
  import { Label } from '@lib/components/ui/label'
  import { Button } from '@lib/components/ui/button'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { AddCollectionDialog, AddRequestDialog } from '@components/dialogs'
  import { RequestResultViewer } from '@components/requestResult'
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
  import { Send } from 'lucide-svelte'
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@lib/components/ui/tabs'
  import { ResizableHandle, ResizablePane, ResizablePaneGroup } from '@lib/components/ui/resizable'
  import { toast } from 'svelte-sonner'
  import {
    commands,
    type Request,
    type CollectionConfig,
    type Options,
    type Environment
  } from '../tauriApi'
  import { register } from '@tauri-apps/plugin-global-shortcut'
  import { debounce } from '@lib/utils'
  import AddEnvironmentDialog from '@components/dialogs/addEnvironmentDialog.svelte'
  import EditEnvironmentDialog from '@components/dialogs/editEnvironmentDialog.svelte'
  import Mustache from 'mustache'
  import { CodemirrorSingleLine } from '@lib/components/codemirror'

  register('CommandOrControl+S', (event) => {
    if (event.state === 'Pressed') {
      if (!requestCollection) {
        toast.info("Votre requête ne fait partie d'aucune collection", {
          description: "Merci de créer votre collection avant d'enregistrer votre requête"
        })
        return
      }
      updateCollection()
    }
  })

  let defaultRequest: Request = $state({
    name: 'nouvelle requête',
    url: '',
    method: Method.GET,
    id: 'no-id',
    options: {
      body: [],
      headers: [],
      params: []
    },
    pre_request_script: null,
    test: null
  })

  let defaultEnvironment: Environment = $state({
    name: 'défaut',
    id: 'nope',
    vars: []
  })

  let collections: CollectionConfig[] = $state([])
  let selectedRequestId = $state('no-id')
  let selectedEnvironmentId = $state('nope')
  let selectedRequest: Request = $derived(selectRequest())
  let requestCollection: null | CollectionConfig = $derived(getCollection())
  let selectedCollectionEnvironment: Environment = $derived(selectEnvironment())

  let selectedEnvironment = $derived(
    selectedCollectionEnvironment?.name
      ? {
          label: selectedCollectionEnvironment.name,
          value: selectedCollectionEnvironment.id
        }
      : undefined
  )
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

  function getCollection() {
    for (const collection of collections) {
      const request = collection.requests.find(({ id }) => id === selectedRequestId)
      if (request) {
        return collection
      }
    }
    return null
  }

  function selectEnvironment() {
    if (!requestCollection || !requestCollection.environments?.length) {
      return defaultEnvironment
    }

    const environment = requestCollection.environments.find(
      ({ id }) => id === selectedEnvironmentId
    )

    if (environment) {
      return environment
    }

    return defaultEnvironment
  }

  const createNewCollection = async (name: string) => {
    const newCollectionToAdd: CollectionConfig = { name, requests: [], environments: [] }
    await invoke('create_collection', { collectionName: name, config: newCollectionToAdd })
    collections.push(newCollectionToAdd)
    toast.success('Collection créée')
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
    toast.success('Requête créée')
  }

  const createNewEnvironment = async (name: string) => {
    if (!requestCollection) {
      return toast.error("Merci de selectionner une collecion avant d'y ajouter un environnement")
    }

    if (!requestCollection?.environments) {
      requestCollection.environments = []
    }
    requestCollection.environments.push({ name, vars: [], id: 'nope' })

    updateCollection()
  }

  const updateCollection = () => {
    if (!requestCollection) {
      return toast.error('Merci de selectionner la collection que vous voulez sauvegarder')
    }
    invoke('update_collection', {
      collectionName: requestCollection.name,
      config: requestCollection
    })
      .then(() => {
        toast.success('Collection mise à jours')
      })
      .catch((error) => {
        toast.error('Une erreur es survenue lors de la mise à jours de votre collection', {
          description: error
        })
      })
  }

  const getCollections = async () => {
    collections = await invoke('get_collections')
  }

  let sendRequestPromise: Promise<string> = $state(Promise.resolve(''))
  const sendRequest = async () => {
    const hasEnvVars = selectedCollectionEnvironment?.vars.length
    let unparsedEnvVars: Record<string, string> = selectedCollectionEnvironment?.vars.reduce(
      (acc, [key, value]) => ({ ...acc, [key]: value }),
      {}
    )

    const envVars = Object.entries(unparsedEnvVars).reduce(
      (acc, [key, value]) => ({
        ...acc,
        [key]: Mustache.render(value, unparsedEnvVars, {}, { escape: (s: string) => s })
      }),
      {}
    )

    if (!selectedRequest.url) {
      toast.error("Merci d'entrer une url")
      return
    }

    const { promise, resolve, reject } = Promise.withResolvers<string>()

    sendRequestPromise = promise

    const result = await commands.makeApiCall(
      selectedRequest.method,
      hasEnvVars
        ? Mustache.render(selectedRequest.url, envVars, {}, { escape: (s: string) => s })
        : selectedRequest.url,
      {
        ...selectedRequest.options,
        body: selectedRequest.options.body.reduce(
          (acc, [key, { is_active, value }]) => {
            if (!key || !is_active) {
              return acc
            }
            return [
              ...acc,
              [
                hasEnvVars ? Mustache.render(key, envVars, {}, { escape: (s: string) => s }) : key,
                hasEnvVars
                  ? Mustache.render(value, envVars, {}, { escape: (s: string) => s })
                  : value
              ]
            ]
          },
          [] as [unknown, unknown][]
        ),
        headers: selectedRequest.options.headers.reduce(
          (acc, [key, { is_active, value }]) => {
            if (!key || !is_active) {
              return acc
            }
            return [
              ...acc,
              [
                hasEnvVars ? Mustache.render(key, envVars, {}, { escape: (s: string) => s }) : key,
                hasEnvVars
                  ? Mustache.render(value, envVars, {}, { escape: (s: string) => s })
                  : value
              ]
            ]
          },
          [] as [string, string][]
        )
      }
    )

    if (result.status === 'error') {
      toast.error('Une erreur est survenue', { description: result.error })
      return reject(result.error)
    }

    resolve(result.data)
  }

  const saveParams = debounce((params: [string, Options][]) => {
    selectedRequest.options.params = params
  }, 600)

  const setParamsInUrl = debounce((url: string) => {
    const hasOptions = selectedRequest.options.params.some(([, { is_active }]) => is_active)
    selectedRequest.url = `${url}${
      hasOptions
        ? `?${selectedRequest.options.params
            .reduce<string[]>((params, [key, { value, is_active }]) => {
              if (!is_active) {
                return params
              }

              return [...params, `${key}=${value}`]
            }, [])
            .join('&')}`
        : ''
    }`
  }, 600)

  $effect(() => {
    const [url, requestParams] = selectedRequest.url.split('?')

    if (!url) {
      return
    }

    if (requestParams) {
      const params = requestParams
        .split('&')
        .map<[string, string]>((param) => param.split('=') as [string, string])
      saveParams(params.map(([key, value]) => [key, { is_active: true, value }]))
    }

    setParamsInUrl(url)
  })

  const setParamsToUrl = () => {
    const [url] = selectedRequest.url.split('?')
    if (selectedRequest.options.params.length) {
      setParamsInUrl(url)
    }
  }

  function addNewHeader() {
    selectedRequest.options.headers.push(['', { is_active: true, value: '' }])
  }

  function addNewBodyField() {
    selectedRequest.options.body.push(['', { is_active: true, value: '' }])
  }

  function addNewParamField() {
    selectedRequest.options.params = [
      ...selectedRequest.options.params,
      ['', { is_active: true, value: '' }]
    ]
  }

  function deleteBody(i: number) {
    selectedRequest.options.body.splice(i, 1)
  }
  function deleteHeader(i: number) {
    selectedRequest.options.headers.splice(i, 1)
  }
  function deleteParam(i: number) {
    selectedRequest.options.params.splice(i, 1)
  }
</script>

{#snippet configView()}
  <RequestConfig
    variables={selectedCollectionEnvironment.vars}
    bind:requestOptions={selectedRequest.options}
    {addNewHeader}
    {addNewBodyField}
    {addNewParamField}
    {deleteBody}
    {deleteHeader}
    {deleteParam}
    {setParamsToUrl}
  />
{/snippet}

{#snippet resultView()}
  {#await sendRequestPromise}
    <!-- promise is pending -->
    sending unicorns...
  {:then result}
    <!-- promise was fulfilled -->
    <RequestResultViewer {result} />
  {:catch error}
    <!-- promise was rejected -->
    {error}
  {/await}
{/snippet}

{#snippet environmentSelect()}
  <span class="flex min-w-64 max-w-[50%] gap-2">
    <Select
      disabled={!requestCollection}
      selected={selectedEnvironment}
      onSelectedChange={(v) => {
        v && (selectedEnvironmentId = v.value)
      }}
    >
      <SelectTrigger class="col-span-1">
        <SelectValue placeholder="Sélectionnez l'environnement" />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          {#if requestCollection?.environments}
            <!-- content here -->
            {#each requestCollection?.environments as { id, name }}
              <SelectItem class="flex justify-between" value={id} label={name}>{name}</SelectItem>
            {/each}
            <SelectItem
              class="flex justify-between"
              value={defaultEnvironment.id}
              label={defaultEnvironment.name}
            >
              {defaultEnvironment.name}
            </SelectItem>
          {/if}
          <AddEnvironmentDialog onSend={createNewEnvironment} />
        </SelectGroup>
      </SelectContent>
    </Select>
    {#if selectedCollectionEnvironment}
      <!-- content here -->
      <EditEnvironmentDialog
        environmentName={selectedCollectionEnvironment.name}
        bind:environmentVariables={selectedCollectionEnvironment.vars}
        onSend={updateCollection}
      />
    {/if}
  </span>
{/snippet}

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
  <div class="flex justify-between">
    <h1>{selectedRequest.name}</h1>
    {@render environmentSelect()}
  </div>
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
    <CodemirrorSingleLine
      class="col-span-3"
      placeholder="url"
      variables={selectedCollectionEnvironment.vars}
      bind:value={selectedRequest.url}
    />
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
