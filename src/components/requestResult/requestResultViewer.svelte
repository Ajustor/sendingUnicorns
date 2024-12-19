<script lang="ts">
  import { Tabs, TabsList, TabsContent, TabsTrigger } from '@lib/components/ui/tabs'
  import { Codemirror } from '@lib/components/codemirror'

  type Props = {
    result: string
  }

  let { result }: Props = $props()
</script>

<Tabs class="h-dvh w-full">
  <TabsList>
    <TabsTrigger value="raw">Result</TabsTrigger>
    <TabsTrigger value="preview">Preview</TabsTrigger>
  </TabsList>
  <TabsContent value="raw" class="h-3/4 overflow-scroll">
    <Codemirror
      class="col-span-3"
      placeholder="result"
      isSingleLine={false}
      variables={[]}
      bind:value={result}
    />
  </TabsContent>
  <TabsContent class="h-full" value="preview">
    <iframe class="h-full w-full" srcdoc={result} title="Request's result"></iframe>
  </TabsContent>
</Tabs>
