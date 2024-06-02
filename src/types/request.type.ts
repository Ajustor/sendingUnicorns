import type { Method } from '@enums/methods'

export type Request = {
  id: string
  url: string,
  name: string,
  method: Method
}

export type RequestOptions = {
  body: Record<string, unknown>,
  headers: Record<string, string>,
  params: Record<string, unknown>
}