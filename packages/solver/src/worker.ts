import init, { Solver } from '../wasm/solver.js'
import type { FromWorker, ToWorker } from './protocol.ts'

const scope = self as unknown as DedicatedWorkerGlobalScope
let solver: Solver | undefined
let queue = Promise.resolve()

const post = (message: FromWorker) => scope.postMessage(message)

const handle = async (message: ToWorker) => {
  switch (message.type) {
    case 'init':
      await init({ module_or_path: message.wasm })
      post({ type: 'ready' })
      return
    case 'challenge': {
      solver?.free()
      const s = new Solver(message.token)
      solver = s
      const { alg, k, difficulty, memoryKib, iterations, expiresAt, site, action } = s
      post({ type: 'challenge', info: { alg, k, difficulty, memoryKib, iterations, expiresAt, site, action } })
      return
    }
    case 'search': {
      const started = performance.now()
      const found = solver!.search(message.sub, message.start, message.count)
      const tried = found === undefined ? message.count : found - message.start + 1
      post({ type: 'result', sub: message.sub, tried, found, ms: performance.now() - started })
      return
    }
    case 'solution':
      post({ type: 'solution', solution: solver!.solution(Float64Array.from(message.nonces)) })
  }
}

scope.onmessage = (event: MessageEvent<ToWorker>) => {
  queue = queue.then(() => handle(event.data)).catch((error: unknown) => post({ type: 'error', message: String(error) }))
}
