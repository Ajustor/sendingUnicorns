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

<Dialog closeOnOutsideClick>
  <DialogTrigger>
    <Button title="Ajouter un environnement" class="gap-2">Ajouter un environnement<Plus /></Button>
  </DialogTrigger>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Ajouter un environnement</DialogTitle>
      <DialogDescription>Entrez les informations relatives à votre environnement</DialogDescription>
    </DialogHeader>
    <div class="grid gap-4 py-4">
      <div class="grid grid-cols-4 items-center gap-4">
        <Label for="name" class="text-right">Nom de votre environnement</Label>
        <Input id="name" bind:value={name} placeholder="la prod" class="col-span-3" />
      </div>
    </div>
    <DialogFooter>
      <Close
        data-dialog-close
        onclick={() => {
          onSend(name)
        }}
      >
        Sauvegarder
      </Close>
    </DialogFooter>
  </DialogContent>
</Dialog>
