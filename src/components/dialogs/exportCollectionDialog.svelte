<script lang="ts">
  import {
    Close,
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger
  } from '@lib/components/ui/dialog'
  import {
    Select,
    SelectContent,
    SelectGroup,
    SelectItem,
    SelectTrigger
  } from '@lib/components/ui/select'

  type Props = { onSend: (name: string) => void; collections: string[]; open: boolean }

  let selectedCollection = $state('')

  let { onSend, collections, open = $bindable() }: Props = $props()
</script>

{#snippet dialogContent()}
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Export collection</DialogTitle>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <!-- content here -->
      <div class="grid grid-cols-4 items-center gap-4">
        <Select type="single" bind:value={selectedCollection}>
          <SelectTrigger class="col-span-3">
            {selectedCollection ?? 'Select collection'}
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {#each collections as collection}
                <SelectItem value={collection} label={collection}>{collection}</SelectItem>
              {/each}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>
    </div>
    <DialogFooter>
      <Close
        data-dialog-close
        onclick={() => {
          onSend(selectedCollection)
        }}
      >
        Export
      </Close>
    </DialogFooter>
  </DialogContent>
{/snippet}

<Dialog bind:open>
  {@render dialogContent()}
</Dialog>
