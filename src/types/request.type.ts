import type { Method } from '@enums/methods'

export type Request = {
  id: string
  url: string,
  name: string,
  method: Method
}