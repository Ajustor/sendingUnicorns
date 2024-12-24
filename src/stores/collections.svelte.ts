import type { CollectionConfig } from '../tauriApi'
import { requestStore } from './request.svelte'

let collections = $state<CollectionConfig[]>([])

export const collectionsStore = {
  get collections() {
    return collections
  },

  set collections(value: CollectionConfig[]) {
    collections = value
  },

  get collection() {
    for (const collection of collectionsStore.collections) {
      const request = collection.requests.find(({ id }) => id === requestStore.request.id)
      if (request) {
        return collection
      }
    }
    return null
  }
}
