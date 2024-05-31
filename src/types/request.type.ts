import type { Method } from '@enums/methods'

export type Request = {
  url: string,
  name: string,
  method: Method
}