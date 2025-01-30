<script lang="ts">
  import { Tabs, TabsList, TabsContent, TabsTrigger } from '@lib/components/ui/tabs'
  import { Codemirror } from '@lib/components/codemirror'

  type Props = {
    result: string
  }

  let { result }: Props = $props()
</script>

<Tabs value="raw" class="w-full h-[79vh] overflow-hidden flex flex-col p-2">
  <TabsList class="w-fit">
    <TabsTrigger value="raw">Result</TabsTrigger>
    <TabsTrigger value="preview">Preview</TabsTrigger>
  </TabsList>
  <TabsContent value="raw" class="w-full h-full flex-grow overflow-hidden">
    <Codemirror
      class="col-span-3 overflow-auto"
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
