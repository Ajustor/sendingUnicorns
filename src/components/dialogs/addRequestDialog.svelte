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
    SelectTrigger,
    SelectContent,
    SelectItem,
    SelectGroup
  } from '@lib/components/ui/select'
  import { Button, buttonVariants } from '@lib/components/ui/button'
  import { Label } from '@lib/components/ui/label'
  import { Input } from '@lib/components/ui/input'
  import { Plus } from 'lucide-svelte'
  import { Method } from '@enums/methods'

  type Props = { onSend: (name: string, url: string, method: Method) => void }

  let name = $state('')
  let url = $state('')
  let method: Method = $state(Method.GET)

  let selectedMethod = $derived(
    method
      ? {
          label: method,
          value: method
        }
      : undefined
  )

  let { onSend }: Props = $props()
</script>

<Dialog>
  <DialogTrigger class={`${buttonVariants()} gap-2 w-full`} title="Create a request">
    Create a request<Plus />
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Create a request</DialogTitle>
      <DialogDescription>Enter request informations</DialogDescription>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="name" class="text-right">Request's name</Label>
        <Input id="name" bind:value={name} placeholder="Val Jean Jean" class="col-span-3" />
      </div>
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="url" class="text-right">Request's url</Label>
        <Input
          id="url"
          bind:value={url}
          placeholder="https://www.youtube.com/watch?v=dQw4w9WgXcQ"
          class="col-span-3"
        />
      </div>
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="method" class="text-right">Request method</Label>
        <Select type="single" bind:value={method}>
          <SelectTrigger class="col-span-3">
            {selectedMethod?.label ?? 'Select method'}
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {#each Object.entries(Method) as [key, value]}
                <SelectItem typeof={typeof Method} {value} label={key}>{key}</SelectItem>
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
          onSend(name, url, method)
        }}
      >
        Save
      </Close>
    </DialogFooter>
  </DialogContent>
</Dialog>
