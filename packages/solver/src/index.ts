import type { ChallengeInfo, FromWorker, ToWorker } from './protocol.ts'

export type { ChallengeInfo }

export interface Progress {
  solved: number
  k: number
  attempts: number
  expectedAttempts: number
}

export interface SolveOptions {
  onProgress?: (progress: Progress) => void
  signal?: AbortSignal
}

export interface SolveResult {
  solution: string
  info: ChallengeInfo
  attempts: number
  ms: number
}

export interface PoolOptions {
  size?: number
  wasmUrl?: string | URL
  workerUrl?: string | URL
}

const BATCH_MS = 40

interface Channel {
  call: (message: ToWorker) => Promise<FromWorker>
  terminate: () => void
}

const openChannel = (url: string | URL): Channel => {
  const worker = new Worker(url, { type: 'module' })
  const waiting: Array<(message: FromWorker) => void> = []
  worker.onmessage = (event: MessageEvent<FromWorker>) => waiting.shift()?.(event.data)
  worker.onerror = (event) => {
    for (const resolve of waiting.splice(0)) resolve({ type: 'error', message: event.message })
  }
  return {
    call: (message) =>
      new Promise((resolve, reject) => {
        waiting.push((reply) => (reply.type === 'error' ? reject(new Error(reply.message)) : resolve(reply)))
        worker.postMessage(message)
      }),
    terminate: () => worker.terminate(),
  }
}

export const defaultPoolSize = () => Math.max(1, Math.min(navigator.hardwareConcurrency || 4, 8))

/** A warm set of solver workers; create it early so the WASM is compiled before a challenge arrives. */
export const createPool = (options: PoolOptions = {}) => {
  const wasm = String(options.wasmUrl ?? new URL('../wasm/solver_bg.wasm', import.meta.url))
  const workerUrl = options.workerUrl ?? new URL('./worker.js', import.meta.url)
  const channels = Array.from({ length: options.size ?? defaultPoolSize() }, () => openChannel(workerUrl))
  const ready = Promise.all(channels.map((c) => c.call({ type: 'init', wasm })))
  let busy = false

  const solve = async (token: string, { onProgress, signal }: SolveOptions = {}): Promise<SolveResult> => {
    if (busy) throw new Error('pool is already solving')
    busy = true
    try {
      await ready
      const started = performance.now()
      const replies = await Promise.all(channels.map((c) => c.call({ type: 'challenge', token })))
      const info = (replies[0] as Extract<FromWorker, { type: 'challenge' }>).info
      const subs = Array.from({ length: info.k }, () => ({ next: 0, active: 0, nonce: undefined as number | undefined }))
      let solved = 0
      let attempts = 0
      const expectedAttempts = info.k * info.difficulty
      const report = () => onProgress?.({ solved, k: info.k, attempts, expectedAttempts })

      const run = async (channel: Channel) => {
        let count = info.alg === 'sha256' ? 20_000 : 1
        while (solved < info.k) {
          signal?.throwIfAborted()
          const open = subs.filter((s) => s.nonce === undefined)
          const sub = open.reduce((a, b) => (b.active < a.active ? b : a))
          const index = subs.indexOf(sub)
          const start = sub.next
          sub.next += count
          sub.active++
          const reply = (await channel.call({ type: 'search', sub: index, start, count })) as Extract<
            FromWorker,
            { type: 'result' }
          >
          sub.active--
          attempts += reply.tried
          if (reply.found !== undefined && sub.nonce === undefined) {
            sub.nonce = reply.found
            solved++
          }
          report()
          count = Math.max(1, Math.round((reply.tried / Math.max(reply.ms, 1)) * BATCH_MS))
        }
      }

      const failure = (await Promise.allSettled(channels.map(run))).find((r) => r.status === 'rejected')
      if (failure) throw failure.reason
      const reply = await channels[0]!.call({ type: 'solution', nonces: subs.map((s) => s.nonce!) })
      const { solution } = reply as Extract<FromWorker, { type: 'solution' }>
      return { solution, info, attempts, ms: performance.now() - started }
    } finally {
      busy = false
    }
  }

  return {
    size: channels.length,
    ready: ready.then(() => undefined),
    solve,
    destroy: () => channels.forEach((c) => c.terminate()),
  }
}

export type Pool = ReturnType<typeof createPool>
