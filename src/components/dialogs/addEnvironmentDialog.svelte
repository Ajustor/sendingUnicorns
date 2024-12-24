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

  import { Button } from '@lib/components/ui/button'
  import { Label } from '@lib/components/ui/label'
  import { Input } from '@lib/components/ui/input'
  import { Plus } from 'lucide-svelte'

  type Props = { onSend: (name: string) => void }

  let name = $state('')

  let { onSend }: Props = $props()
</script>

{#snippet dialogContent()}
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Create an environment</DialogTitle>
      <DialogDescription>Enter environment informations</DialogDescription>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="name" class="text-right">Environment's name</Label>
        <Input id="name" bind:value={name} placeholder="The prod" class="col-span-3" />
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
{/snippet}

<Dialog>
  <DialogTrigger>
    <Button title="Create an environment" class="gap-2">Create an environment<Plus /></Button>
  </DialogTrigger>
  {@render dialogContent()}
</Dialog>
