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
  import { Button, buttonVariants } from '@lib/components/ui/button'
  import { Label } from '@lib/components/ui/label'
  import { Input } from '@lib/components/ui/input'
  import { Plus } from 'lucide-svelte'

  type Props = { onSend: (name: string) => void }

  let name = $state('')

  let { onSend }: Props = $props()
</script>

<Dialog>
  <DialogTrigger class={`${buttonVariants()} gap-2 w-full`} title="Create a collection">
    Create a collection<Plus />
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Create a collection</DialogTitle>
      <DialogDescription>Enter collection informations</DialogDescription>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="name" class="text-right">Collection's name</Label>
        <Input id="name" bind:value={name} placeholder="Val Jean Jean" class="col-span-3" />
      </div>
    </div>
    <DialogFooter>
      <Close
        data-dialog-close
        onclick={() => {
          onSend(name)
        }}
      >
        Save
      </Close>
    </DialogFooter>
  </DialogContent>
</Dialog>
