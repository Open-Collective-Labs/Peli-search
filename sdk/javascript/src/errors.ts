export class PeliSearchError extends Error {
  readonly status: number
  readonly body: unknown

  constructor(message: string, status: number, body?: unknown) {
    super(message)
    this.name = "PeliSearchError"
    this.status = status
    this.body = body
  }
}

export function isPeliSearchError(err: unknown): err is PeliSearchError {
  return err instanceof PeliSearchError
}
