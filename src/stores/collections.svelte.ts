import type { CollectionConfig } from '../tauriApi'
import { useRune } from './sharedStore'

export const useCollections = (defaultCollections: CollectionConfig[] | null = null) => useRune('collections', defaultCollections)
