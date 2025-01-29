<script lang="ts">
  import { Tabs, TabsList, TabsContent, TabsTrigger } from '@lib/components/ui/tabs'
  import { Codemirror } from '@lib/components/codemirror'

  type Props = {
    result: string
  }

  let { result }: Props = $props()
</script>

<Tabs value="raw" class="w-full overflow-hidden flex flex-col">
  <TabsList>
    <TabsTrigger value="raw">Result</TabsTrigger>
    <TabsTrigger value="preview">Preview</TabsTrigger>
  </TabsList>
  <TabsContent value="raw" class="w-full flex-grow">
    <Codemirror
      class="col-span-3 max-h-[73vh] h-full overflow-auto"
      placeholder="result"
      isSingleLine={false}
      variables={[]}
      maxHeight="100%"
      bind:value={result}
    />
  </TabsContent>
  <TabsContent class="h-full" value="preview">
    <iframe class="h-full w-full" srcdoc={result} title="Request's result"></iframe>
  </TabsContent>
</Tabs>
