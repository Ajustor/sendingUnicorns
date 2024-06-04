<script lang="ts">
  import { Tabs, TabsContent, TabsList, TabsTrigger } from '@lib/components/ui/tabs'
  import { Input } from '@lib/components/ui/input'
  import { Button } from '@lib/components/ui/button'
  import { Plus } from 'lucide-svelte'
  import type { RequestOptions } from '../../backApi'

  export let requestOptions: RequestOptions

  function addNewHeader() {
    requestOptions.headers?.push(['', ''])
  }
</script>

<Tabs>
  <TabsList>
    <TabsTrigger value="headers">Headers</TabsTrigger>
    <TabsTrigger value="params">Paramètres</TabsTrigger>
    <TabsTrigger value="body">Body</TabsTrigger>
  </TabsList>
  <TabsContent value="headers">
    {#each requestOptions.headers as [headerKey, headerValue]}
      <div class="flex">
        <Input placeholder="key" bind:value={headerKey} />
        <Input placeholder="value" bind:value={headerValue} />
      </div>
    {/each}
    <Button onclick={addNewHeader} class="gap-2">Ajouter un header <Plus /></Button>
  </TabsContent>
  <TabsContent value="params">params configuration goes here</TabsContent>
  <TabsContent value="body">body configuration goes here</TabsContent>
</Tabs>
