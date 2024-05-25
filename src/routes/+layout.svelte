<script lang="ts">
  import '../app.css'
  import { ModeWatcher } from 'mode-watcher'
  import { Toaster } from '$lib/components/ui/sonner'
  import { Button } from '@lib/components/ui/button'
  import { Menu } from 'lucide-svelte'

  import { Sheet, SheetTrigger } from '@lib/components/ui/sheet'
</script>

<ModeWatcher />
<Toaster />

<nav>
  <Sheet>
    <SheetTrigger><Button variant="link"><Menu /></Button></SheetTrigger>
  </Sheet>
</nav>

<slot></slot>

<style></style>
