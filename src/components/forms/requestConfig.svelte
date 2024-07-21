<script lang="ts">
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@lib/components/ui/tabs'
  import { Button } from '@lib/components/ui/button'
  import { Plus, Trash } from 'lucide-svelte'
  import type { RequestOptions } from '../../tauriApi'
  import { Checkbox } from '@lib/components/ui/checkbox'
  import { CodemirrorSingleLine } from '@lib/components/codemirror'

  type Props = {
    variables: [string, string][]
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
    requestOptions = $bindable(),
    addNewHeader,
    addNewBodyField,
    addNewParamField,
    deleteBody,
    deleteHeader,
    setParamsToUrl,
    deleteParam
  }: Props = $props()
</script>

<Tabs>
  <TabsList>
    <TabsTrigger value="headers">Headers</TabsTrigger>
    <TabsTrigger value="params">Paramètres</TabsTrigger>
    <TabsTrigger value="body">Body</TabsTrigger>
  </TabsList>
  <TabsContent value="headers">
    {#each requestOptions.headers as header, i}
      <div class="flex items-center justify-center gap-3">
        <Checkbox bind:checked={header[1].is_active} />
        <CodemirrorSingleLine {variables} placeholder="key" bind:value={header[0]} />
        <CodemirrorSingleLine {variables} placeholder="value" bind:value={header[1].value} />
        <Button onclick={() => deleteHeader(i)} class="gap-2" title="Supprimer la valeur">
          <Trash />
        </Button>
      </div>
    {/each}
    <Button onclick={addNewHeader} class="mt-4 gap-2">Ajouter un header <Plus /></Button>
  </TabsContent>
  <TabsContent value="params">
    {#each requestOptions.params as param, i}
      <div class="flex items-center justify-center gap-3">
        <Checkbox on:click={setParamsToUrl} bind:checked={param[1].is_active} />
        <CodemirrorSingleLine
          {variables}
          onkeyup={setParamsToUrl}
          placeholder="key"
          bind:value={param[0]}
        />
        <CodemirrorSingleLine
          {variables}
          onkeyup={setParamsToUrl}
          placeholder="value"
          bind:value={param[1].value}
        />
        <Button onclick={() => deleteParam(i)} class="gap-2" title="Supprimer la valeur">
          <Trash />
        </Button>
      </div>
    {/each}
    <Button onclick={addNewParamField} class="mt-4 gap-2">
      Ajouter un élément dans les paramètres <Plus />
    </Button>
  </TabsContent>
  <TabsContent value="body">
    {#each requestOptions.body as body, i}
      <div class="flex items-center justify-center gap-3">
        <Checkbox bind:checked={body[1].is_active} />
        <CodemirrorSingleLine {variables} placeholder="key" bind:value={body[0]} />
        <CodemirrorSingleLine {variables} placeholder="value" bind:value={body[1].value} />
        <Button onclick={() => deleteBody(i)} class="gap-2" title="Supprimer la valeur">
          <Trash />
        </Button>
      </div>
    {/each}
    <Button onclick={addNewBodyField} class="mt-4 gap-2">
      Ajouter un élément dans le corps <Plus />
    </Button>
  </TabsContent>
</Tabs>
