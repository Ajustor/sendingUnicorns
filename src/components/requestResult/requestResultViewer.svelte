<script lang="ts">
  import { Tabs, TabsList, TabsContent, TabsTrigger } from '@lib/components/ui/tabs'
  import { ScrollArea } from '@lib/components/ui/scroll-area'

  export let result: string
</script>

<Tabs class="h-dvh w-full">
  <TabsList>
    <TabsTrigger value="raw">Résultat</TabsTrigger>
    <TabsTrigger value="preview">Prévisualisation</TabsTrigger>
  </TabsList>
  <TabsContent value="raw">
    <ScrollArea>
      <p class="max-h-dvh max-w-prose">
        {result}
      </p>
    </ScrollArea>
  </TabsContent>
  <TabsContent class="h-full" value="preview">
    <iframe class="h-full w-full" srcdoc={result} title="résultat de la requête"></iframe>
  </TabsContent>
</Tabs>
