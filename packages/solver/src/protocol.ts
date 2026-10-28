export type ToWorker =
  | { type: 'init'; wasm: string }
  | { type: 'challenge'; token: string }
  | { type: 'search'; sub: number; start: number; count: number }
  | { type: 'solution'; nonces: number[] }

export interface ChallengeInfo {
  alg: string
  k: number
  difficulty: number
  memoryKib: number
  iterations: number
  expiresAt: number
  site: string
  action: string
}

export type FromWorker =
  | { type: 'ready' }
  | { type: 'challenge'; info: ChallengeInfo }
  | { type: 'result'; sub: number; tried: number; found?: number; ms: number }
  | { type: 'solution'; solution: string }
  | { type: 'error'; message: string }
