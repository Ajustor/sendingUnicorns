import { collectionsStore } from './collections.svelte'
import { requestStore } from './request.svelte'

export const collectionStore = {
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