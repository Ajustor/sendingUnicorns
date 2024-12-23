import { Method } from '@enums/methods'
import type { Request } from '../tauriApi'
import { useRune } from './sharedStore'

const defaultRequest: Request = $state({
  name: 'New request',
  url: '',
  method: Method.GET,
  id: 'no-id',
  options: {
    body: { form_data: [], json: '' },
    headers: [],
    params: []
  },
  pre_request_script: null,
  test: null
})

export const useRequest = () => useRune('request', defaultRequest)