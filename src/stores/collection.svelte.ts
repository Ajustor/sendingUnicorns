import type { CollectionConfig } from '../tauriApi'
import { useRune } from './sharedStore'

export const useCollection = (defaultCollection: CollectionConfig | null = null) => useRune('collection', defaultCollection)