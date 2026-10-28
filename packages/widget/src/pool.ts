import { type Pool, type PoolOptions, type SolveOptions, type SolveResult, createPool } from '@nextmoe/pow-solver'

let options: PoolOptions = {}
let pool: Pool | undefined
let queue: Promise<unknown> = Promise.resolve()

/** Set worker/WASM URLs or the pool size; takes effect only before the first widget starts. */
export const configure = (next: PoolOptions) => {
  options = next
}

export const warmUp = () => (pool ??= createPool(options)).ready

/** Every widget on the page shares one pool, which solves one challenge at a time. */
export const solveQueued = (token: string, solveOptions: SolveOptions): Promise<SolveResult> => {
  const run = queue.then(() => (pool ??= createPool(options)).solve(token, solveOptions))
  queue = run.catch(() => undefined)
  return run
}
