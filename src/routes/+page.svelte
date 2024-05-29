<script lang="ts">
  import { Input } from '@lib/components/ui/input'

  type Props = {
    headerTitle: string
  }

  let { headerTitle }: Props = $props()

  let url = $state('')
</script>

<div>
  <Input placeholder="url" bind:value={url} class="w-full" type="url" />
</div>
