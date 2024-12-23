import type { CollectionConfig } from '../tauriApi'

let collections = $state<CollectionConfig[]>([])

export const collectionsStore = {
  get collections() {
    return collections
  },

  set collections(value: CollectionConfig[]) {
    collections = value
  }
}
