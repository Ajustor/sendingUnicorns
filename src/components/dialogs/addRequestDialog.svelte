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
    SelectGroup,
    SelectValue
  } from '@lib/components/ui/select'
  import { Button } from '@lib/components/ui/button'
  import { Label } from '@lib/components/ui/label'
  import { Input } from '@lib/components/ui/input'
  import { Plus } from 'lucide-svelte'
  import { Method } from '@enums/methods'

  type Props = { onSend: (name: string, url: string, method: Method) => void }

  let name = $state('')
  let url = $state('')
  let method: Method = $state(Method.GET)

  let { onSend }: Props = $props()
</script>

<Dialog closeOnOutsideClick>
  <DialogTrigger>
    <Button title="Ajouter une requête">Ajouter une requête<Plus /></Button>
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Ajouter une requête</DialogTitle>
      <DialogDescription>Entrez les informations relatives à votre requête</DialogDescription>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="name" class="text-right">Nom de votre requête</Label>
        <Input id="name" bind:value={name} placeholder="Val Jean Jean" class="col-span-3" />
      </div>
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="url" class="text-right">L'url de votre requête</Label>
        <Input
          id="url"
          bind:value={url}
          placeholder="https://www.youtube.com/watch?v=dQw4w9WgXcQ"
          class="col-span-3"
        />
      </div>
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="method" class="text-right">la méthode de votre requête</Label>
        <Select
          onSelectedChange={({ value }) => {
            value && (method = value)
          }}
        >
          <SelectTrigger class="col-span-3">
            <SelectValue placeholder="Sélectionnez la méthode" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              {#each Object.entries(Method) as [key, value]}
                <SelectItem {value} label={key}>{key}</SelectItem>
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
        Sauvegarder
      </Close>
    </DialogFooter>
  </DialogContent>
</Dialog>
