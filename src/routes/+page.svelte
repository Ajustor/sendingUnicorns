<script lang="ts">
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
    SelectGroup
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
    type Environment,
    type BodyTypes,
    type BodyTypesEnum
  } from '../tauriApi'
  import { debounce } from '@lib/utils'
  import AddEnvironmentDialog from '@components/dialogs/addEnvironmentDialog.svelte'
  import EditEnvironmentDialog from '@components/dialogs/editEnvironmentDialog.svelte'
  import Mustache from 'mustache'
  import { Codemirror } from '@lib/components/codemirror'
  import { Input } from '@lib/components/ui/input'
  import { BodyTypeEnum } from '@enums/bodyTypes'
  import { listen } from '@tauri-apps/api/event'
  import { collectionsStore } from '../stores/collections.svelte'
  import { requestStore } from '../stores/request.svelte'

  let defaultEnvironment: Environment = $state({
    name: 'default',
    id: 'nope',
    vars: []
  })

  let selectedEnvironmentId = $state('nope')
  let requestCollection: null | CollectionConfig = $derived(getCollection())
  let selectedCollectionEnvironment: Environment = $derived(selectEnvironment())
  let bodyType: BodyTypesEnum = $state(BodyTypeEnum.FORM_DATA)
  let needRedrawOfConfig = $state(false)

  let selectedEnvironment = $derived(
    selectedCollectionEnvironment?.name
      ? {
          label: selectedCollectionEnvironment.name,
          value: selectedCollectionEnvironment.id
        }
      : undefined
  )

  let selectedMethod = $derived(
    requestStore.request.method
      ? {
          label: requestStore.request.method.toUpperCase(),
          value: requestStore.request.method
        }
      : undefined
  )

  function getCollection() {
    for (const collection of collectionsStore.collections) {
      const request = collection.requests.find(({ id }) => id === requestStore.request.id)
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
    needRedrawOfConfig = true

    const environment = requestCollection.environments.find(
      ({ id }) => id === selectedEnvironmentId
    )

    if (environment) {
      needRedrawOfConfig = false
      return environment
    }
    needRedrawOfConfig = false

    return defaultEnvironment
  }

  const createNewEnvironment = async (name: string) => {
    if (!requestCollection) {
      return toast.error('Please select a collection to create a new environment')
    }

    if (!requestCollection?.environments) {
      requestCollection.environments = []
    }
    requestCollection.environments.push({ name, vars: [], id: 'nope' })

    updateCollection()
  }

  listen('save', () => {
    updateCollection()
  })

  const updateCollection = () => {
    if (!requestCollection) {
      return toast.error("Votre requête ne fait partie d'aucune collection", {
        description: "Merci de créer votre collection avant d'enregistrer votre requête"
      })
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

  let sendRequestPromise: Promise<string> | null = $state(null)
  let isSending = $state<boolean>(false)

  function getBody(hasEnvVars: boolean, envVars: Record<string, string>): BodyTypes {
    if (!requestStore.request) {
      throw toast.error('An error occured while try to retrieve body')
    }

    return {
      json: Mustache.render(
        requestStore.request.options.body.json,
        envVars,
        {},
        { escape: (s: string) => s }
      ),
      form_data: requestStore.request.options.body.form_data.reduce<[string, Options][]>(
        (acc, [key, { is_active, value }]) => {
          if (!key || !is_active) {
            return acc
          }
          return [
            ...acc,
            [
              hasEnvVars ? Mustache.render(key, envVars, {}, { escape: (s: string) => s }) : key,
              {
                is_active,
                value:
                  hasEnvVars && typeof value === 'string'
                    ? Mustache.render(value, envVars, {}, { escape: (s: string) => s })
                    : value
              }
            ]
          ]
        },
        []
      )
    }
  }

  const sendRequest = async () => {
    const hasEnvVars = !!selectedCollectionEnvironment?.vars.length
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
    const request = requestStore.request

    if (!request) {
      toast.error('An error occured with the request')
      return
    }

    if (!request?.url) {
      toast.error('Please write an url')
      return
    }

    isSending = true
    sendRequestPromise = new Promise(async (resolve, reject) => {
      const result = await commands.makeApiCall(
        request.method,
        hasEnvVars
          ? Mustache.render(request.url, envVars, {}, { escape: (s: string) => s })
          : request.url,
        {
          ...request.options,
          body: getBody(hasEnvVars, envVars),
          headers: request.options.headers.reduce<[string, string][]>(
            (acc, [key, { is_active, value }]) => {
              if (!key || !is_active) {
                return acc
              }
              return [
                ...acc,
                [
                  hasEnvVars
                    ? Mustache.render(key, envVars, {}, { escape: (s: string) => s })
                    : key,
                  hasEnvVars && typeof value === 'string'
                    ? Mustache.render(value, envVars, {}, { escape: (s: string) => s })
                    : `${value}`
                ]
              ]
            },
            []
          )
        },
        bodyType
      )

      if (result.status === 'error') {
        //toast.error('An error occured', { description: result.error })
        reject(result.error)
        return
      }

      resolve(result.data)
    })

    sendRequestPromise.finally(() => (isSending = false))
  }

  const saveParams = debounce((params: [string, Options][]) => {
    if (!requestStore.request) {
      return
    }
    requestStore.request.options.params = params
  }, 600)

  const setParamsInUrl = debounce((url: string) => {
    if (!requestStore.request) {
      return
    }
    const hasOptions = requestStore.request.options.params.some(([, { is_active }]) => is_active)
    requestStore.request.url = `${url}${
      hasOptions
        ? `?${requestStore.request.options.params
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
    if (!requestStore.request) {
      return
    }

    const [url, requestParams] = requestStore.request.url.split('?')

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
    if (!requestStore.request) {
      return
    }

    const [url] = requestStore.request.url.split('?')
    if (requestStore.request.options.params.length) {
      setParamsInUrl(url)
    }
  }

  function addNewHeader() {
    requestStore.request.options.headers.push(['', { is_active: true, value: '' }])
  }

  function addNewBodyField() {
    requestStore.request.options.body.form_data.push(['', { is_active: true, value: '' }])
  }

  function addNewParamField() {
    if (!requestStore.request) {
      return
    }
    requestStore.request.options.params = [
      ...requestStore.request.options.params,
      ['', { is_active: true, value: '' }]
    ]
  }

  function deleteBody(i: number) {
    requestStore.request.options.body.form_data.splice(i, 1)
  }

  function deleteHeader(i: number) {
    requestStore.request.options.headers.splice(i, 1)
  }
  function deleteParam(i: number) {
    requestStore.request.options.params.splice(i, 1)
  }
</script>

{#snippet configView()}
  {#if !needRedrawOfConfig}
    <!-- content here -->
    <RequestConfig
      variables={selectedCollectionEnvironment.vars}
      bind:bodyType
      {addNewHeader}
      {addNewBodyField}
      {addNewParamField}
      {deleteBody}
      {deleteHeader}
      {deleteParam}
      {setParamsToUrl}
    />
  {/if}
{/snippet}

{#snippet resultView()}
  {#await sendRequestPromise}
    sending unicorns...
  {:then result}
    {#if result !== null}
      <RequestResultViewer {result} />
    {/if}
  {:catch error}
    {error}
  {/await}
{/snippet}

{#snippet environmentSelect()}
  <span class="flex min-w-64 max-w-[50%] gap-2">
    <Select disabled={!requestCollection} type="single" bind:value={selectedEnvironmentId}>
      <SelectTrigger class="col-span-1">
        {selectedEnvironment?.label ?? 'Select your environment'}
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

<div id="main">
  <div class="flex justify-between">
    {#if requestStore.request}
      <Input bind:value={requestStore.request.name} class="border-0" />
    {/if}
    {@render environmentSelect()}
  </div>
  <div class="grid grid-cols-5 gap-2 p-2">
    {#if requestStore.request}
      <!-- content here -->
      <Select type="single" bind:value={requestStore.request.method}>
        <SelectTrigger class="col-span-1">
          {selectedMethod?.label ?? 'Select your method'}
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {#each Object.entries(Method) as [key, value]}
              <SelectItem {value} label={key}>{key}</SelectItem>
            {/each}
          </SelectGroup>
        </SelectContent>
      </Select>
      <Codemirror
        class="col-span-3"
        placeholder="url"
        variables={selectedCollectionEnvironment.vars}
        bind:value={requestStore.request.url}
      />
    {/if}
    <Button class="col-span-1 gap-2" onclick={sendRequest} disabled={isSending}>
      {#if isSending}
        <span class="loading loading-spinner loading-lg"></span>
      {:else}
        Send <Send />
      {/if}
    </Button>
  </div>

  <Tabs class="h-dvh w-full">
    <TabsList>
      <TabsTrigger value="config">Configuration</TabsTrigger>
      <TabsTrigger value="combo">Split view</TabsTrigger>
      <TabsTrigger value="result">Results</TabsTrigger>
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
    @apply w-full overflow-hidden p-4;
  }
</style>
