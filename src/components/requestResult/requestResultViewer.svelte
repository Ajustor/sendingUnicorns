<script lang="ts">
  import { Tabs, TabsList, TabsContent, TabsTrigger } from '@lib/components/ui/tabs'
  import { ScrollArea } from '@lib/components/ui/scroll-area'
  import { Codemirror } from '@lib/components/codemirror'

  export let result: string
</script>

<Tabs class="h-dvh w-full">
  <TabsList>
    <TabsTrigger value="raw">Résultat</TabsTrigger>
    <TabsTrigger value="preview">Prévisualisation</TabsTrigger>
  </TabsList>
  <TabsContent value="raw">
    <ScrollArea>
      <Codemirror class="col-span-3" placeholder="url" variables={[]} bind:value={result} />
    </ScrollArea>
  </TabsContent>
  <TabsContent class="h-full" value="preview">
    <iframe class="h-full w-full" srcdoc={result} title="résultat de la requête"></iframe>
  </TabsContent>
</Tabs>
