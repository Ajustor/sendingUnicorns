<script lang="ts">
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@lib/components/ui/tabs'
  import { Button } from '@lib/components/ui/button'
  import { Plus, Trash } from 'lucide-svelte'
  import { type RequestOptions, type BodyTypesEnum } from '../../tauriApi'
  import { Checkbox } from '@lib/components/ui/checkbox'
  import { Codemirror } from '@lib/components/codemirror'
  import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger,
    SelectValue
  } from '@lib/components/ui/select'
  import { json } from '@codemirror/lang-json'
  import { BodyTypeEnum } from '@enums/bodyTypes'

  type Props = {
    variables: [string, string][]
    bodyType: BodyTypesEnum
    requestOptions: RequestOptions
    addNewHeader: () => void
    addNewBodyField: () => void
    addNewParamField: () => void
    setParamsToUrl: () => void
    deleteBody: (i: number) => void
    deleteHeader: (i: number) => void
    deleteParam: (i: number) => void
  }

  let {
    variables,
    bodyType = $bindable(),
    requestOptions = $bindable(),
    addNewHeader,
    addNewBodyField,
    addNewParamField,
    deleteBody,
    deleteHeader,
    setParamsToUrl,
    deleteParam
  }: Props = $props()

  let selectedBodyType = $derived(
    bodyType
      ? {
          label: bodyType,
          value: bodyType
        }
      : undefined
  )

  let localVariables = $derived(variables)
</script>

{#snippet bodyTypeSelector()}
  <Select
    selected={selectedBodyType}
    onSelectedChange={(v) => {
      v && (bodyType = v.value)
    }}
  >
    <SelectTrigger class="col-span-1">
      <SelectValue placeholder="Select environment" />
    </SelectTrigger>
    <SelectContent>
      <SelectGroup>
        <SelectItem
          class="flex justify-between"
          value={BodyTypeEnum.FORM_DATA}
          label={BodyTypeEnum.FORM_DATA}
        >
          Form data
        </SelectItem>
        <SelectItem
          class="flex justify-between"
          value={BodyTypeEnum.JSON}
          label={BodyTypeEnum.JSON}
        >
          Json
        </SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
{/snippet}

<Tabs>
  <TabsList>
    <TabsTrigger value="headers">Headers</TabsTrigger>
    <TabsTrigger value="params">Params</TabsTrigger>
    <TabsTrigger value="body">Body</TabsTrigger>
  </TabsList>
  <TabsContent value="headers">
    {#each requestOptions.headers as header, i}
      <div class="flex items-center justify-center gap-3">
        <Checkbox bind:checked={header[1].is_active} />
        <Codemirror variables={localVariables} placeholder="key" bind:value={header[0]} />
        <Codemirror
          variables={localVariables}
          placeholder="value"
          bind:value={header[1].value as string}
        />
        <Button onclick={() => deleteHeader(i)} class="gap-2" title="Delete">
          <Trash />
        </Button>
      </div>
    {/each}
    <Button onclick={addNewHeader} class="mt-4 gap-2">Add header <Plus /></Button>
  </TabsContent>
  <TabsContent value="params">
    {#each requestOptions.params as param, i}
      <div class="flex items-center justify-center gap-3">
        <Checkbox on:click={setParamsToUrl} bind:checked={param[1].is_active} />
        <Codemirror
          variables={localVariables}
          onkeyup={setParamsToUrl}
          placeholder="key"
          bind:value={param[0]}
        />
        <Codemirror
          variables={localVariables}
          onkeyup={setParamsToUrl}
          placeholder="value"
          bind:value={param[1].value as string}
        />
        <Button onclick={() => deleteParam(i)} class="gap-2" title="Delete">
          <Trash />
        </Button>
      </div>
    {/each}
    <Button onclick={addNewParamField} class="mt-4 gap-2">
      Add param <Plus />
    </Button>
  </TabsContent>
  <TabsContent value="body">
    {@render bodyTypeSelector()}
    {#if bodyType === BodyTypeEnum.JSON}
      <!-- content here -->
      <Codemirror
        variables={localVariables}
        language={json()}
        isSingleLine={false}
        bind:value={requestOptions.body.json}
        placeholder="Json body"
      />
    {:else if bodyType === BodyTypeEnum.FORM_DATA}
      <!-- else content here -->
      {#each requestOptions.body.form_data as body, i}
        <div class="flex items-center justify-center gap-3">
          <Checkbox bind:checked={body[1].is_active} />
          <Codemirror variables={localVariables} placeholder="key" bind:value={body[0]} />
          <Codemirror
            variables={localVariables}
            placeholder="value"
            bind:value={body[1].value as string}
          />
          <Button onclick={() => deleteBody(i)} class="gap-2" title="Delete">
            <Trash />
          </Button>
        </div>
      {/each}
      <Button onclick={addNewBodyField} class="mt-4 gap-2">
        Add body element <Plus />
      </Button>
    {/if}
  </TabsContent>
</Tabs>
