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
  import { Input } from '@lib/components/ui/input'
  import { Pencil, Plus, Trash } from 'lucide-svelte'

  type Props = {
    onSend: (name: string) => void
    environmentVariables: [string, string][]
    environmentName: string
  }

  let name = $state('')

  let { onSend, environmentVariables = $bindable(), environmentName }: Props = $props()

  function addNewVar() {
    if (!environmentName) {
      return
    }
    environmentVariables = [...(environmentVariables ?? []), ['', '']]
  }

  function deleteVar(i: number) {
    if (!environmentName || !environmentVariables) {
      return
    }
    environmentVariables.splice(i, 1)
  }
</script>

<Dialog closeOnOutsideClick>
  <DialogTrigger>
    <Button disabled={!environmentName} title={`Edit ${environmentName}`} class="gap-2">
      <Pencil />
    </Button>
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Edit {environmentName}</DialogTitle>
      <DialogDescription>Enter environment informations</DialogDescription>
    </DialogHeader>
    {#if environmentVariables}
      <!-- content here -->
      {#each environmentVariables as envVar, i}
        <div class="flex items-center justify-center gap-3">
          <Input placeholder="key" bind:value={envVar[0]} />
          <Input placeholder="value" bind:value={envVar[1]} />
          <Button onclick={() => deleteVar(i)} class="gap-2" title="Delete variable">
            <Trash />
          </Button>
        </div>
      {/each}
    {/if}
    <Button onclick={addNewVar} class="mt-4 gap-2">
      Add a new variable <Plus />
    </Button>
    <DialogFooter>
      <Close
        data-dialog-close
        onclick={() => {
          onSend(name)
        }}
      >
        save
      </Close>
    </DialogFooter>
  </DialogContent>
</Dialog>
