import { Method } from '@enums/methods'
import type { Request } from '../tauriApi'

let request = $state<Request>({
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
  test: null,
})

export const requestStore = {
  get request() {
    return request
  },

  set request(value: Request) {
    console.log('set data', value)
    request = value
  }
}