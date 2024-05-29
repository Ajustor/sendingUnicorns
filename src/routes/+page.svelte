<script lang="ts">
  import { Input } from '@lib/components/ui/input'

  type Props = {
    headerTitle: string
  }

  let { headerTitle }: Props = $props()

  let url = $state('')
</script>

<div id="main" class="p-4">
  <div class="p-2">
    <h1>{headerTitle}</h1>
    <Input placeholder="url" bind:value={url} class="w-full" type="url" />
  </div>
</div>

<style>
  #main {
    display: flex;
    flex-direction: column;
  }
</style>
