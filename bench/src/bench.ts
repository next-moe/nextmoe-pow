import { type Pool, createPool, defaultPoolSize } from '@nextmoe/pow-solver'

interface Hash {
  id: string
  alg: number
  m: number
  t: number
}

interface SolveConfig {
  id: string
  hash: string
  k: number
}

interface Rate {
  hash: string
  workers: number
  perSecond: number
}

interface Sample {
  ms: number
  attempts: number
  verifyUs: number
  code: string
  at: number
}

const HASHES: Hash[] = [
  { id: 'sha256', alg: 1, m: 0, t: 0 },
  ...[256, 512, 1024, 2048, 4096].map((m) => ({ id: `argon2id-${m}`, alg: 2, m, t: 1 })),
]

const SOLVES: SolveConfig[] = [
  { id: 'sha256-k16', hash: 'sha256', k: 16 },
  { id: 'argon2id-256-k16', hash: 'argon2id-256', k: 16 },
  { id: 'argon2id-512-k8', hash: 'argon2id-512', k: 8 },
  { id: 'argon2id-1024-k4', hash: 'argon2id-1024', k: 4 },
  { id: 'argon2id-1024-k8', hash: 'argon2id-1024', k: 8 },
  { id: 'argon2id-2048-k4', hash: 'argon2id-2048', k: 4 },
]

const params = new URLSearchParams(location.search)
const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T
const labelInput = $<HTMLInputElement>('label')
const samplesInput = $<HTMLInputElement>('samples')
const targetInput = $<HTMLInputElement>('target')
const simdInput = $<HTMLInputElement>('simd')
const runButton = $<HTMLButtonElement>('run')
const status = $('status')
const log = $('log')

labelInput.value = params.get('label') ?? ''
samplesInput.value = params.get('n') ?? samplesInput.value
targetInput.value = params.get('target') ?? targetInput.value
simdInput.checked = params.get('simd') === '1'

const nav = navigator as Navigator & { deviceMemory?: number }
// Reports are committed, and the repository names no device models; some user agents carry one, so it is not recorded.
const device = {
  hardwareConcurrency: navigator.hardwareConcurrency,
  deviceMemory: nav.deviceMemory,
  poolSize: defaultPoolSize(),
}
$('device').textContent = `${device.hardwareConcurrency} threads · pool ${device.poolSize}`

const say = (line: string) => {
  status.textContent = line
  log.textContent += `${line}\n`
}

const quantile = (sorted: number[], q: number) => sorted[Math.min(sorted.length - 1, Math.ceil(q * sorted.length) - 1)]!

const summarize = (values: number[]) => {
  const sorted = [...values].sort((a, b) => a - b)
  return {
    median: quantile(sorted, 0.5),
    p95: quantile(sorted, 0.95),
    min: sorted[0]!,
    max: sorted.at(-1)!,
    mean: values.reduce((a, b) => a + b, 0) / values.length,
  }
}

const challenge = async (hash: Hash, k: number, d: number, action: string) => {
  const q = new URLSearchParams({ alg: String(hash.alg), m: String(hash.m), t: String(hash.t), k: String(k), d: String(d), site: 'bench', action })
  const res = await fetch(`/api/challenge?${q}`)
  const body = (await res.json()) as { token?: string; error?: string }
  if (!body.token) throw new Error(body.error)
  return body.token
}

const measureRate = async (pool: Pool, hash: Hash, ms: number) => {
  const token = await challenge(hash, 1, 0xffffffff, 'rate')
  const points: Array<[number, number]> = []
  await pool
    .solve(token, { signal: AbortSignal.timeout(ms), onProgress: (p) => points.push([performance.now(), p.attempts]) })
    .catch((error: unknown) => {
      if (!(error instanceof DOMException)) throw error
    })
  const first = points[Math.min(points.length - 1, pool.size)]!
  const last = points.at(-1)!
  return ((last[1] - first[1]) / (last[0] - first[0])) * 1000
}

const row = (cells: Array<string | number>, head = false) =>
  `<tr>${cells.map((c) => `<${head ? 'th' : 'td'}>${typeof c === 'number' ? c.toLocaleString('en', { maximumFractionDigits: 1 }) : c}</${head ? 'th' : 'td'}>`).join('')}</tr>`

const run = async () => {
  runButton.disabled = true
  log.textContent = ''
  const samples = Number(samplesInput.value)
  const targetMs = Number(targetInput.value)
  const wasmUrl = simdInput.checked ? '/packages/solver/wasm-simd/solver_bg.wasm' : undefined
  const only = params.get('only')?.split(',')

  const coldStarted = performance.now()
  const pool = createPool({ wasmUrl })
  await pool.ready
  const coldStartMs = performance.now() - coldStarted
  const single = createPool({ wasmUrl, size: 1 })
  await single.ready
  say(`pool of ${pool.size} ready in ${coldStartMs.toFixed(0)} ms`)

  const rates: Rate[] = []
  const rateTable = $('rates')
  rateTable.innerHTML = row(['hash', '1 worker /s', `${pool.size} workers /s`, 'scaling'], true)
  const solveHashes = new Set(SOLVES.filter((s) => !only || only.includes(s.id)).map((s) => s.hash))
  for (const hash of HASHES.filter((h) => !only || solveHashes.has(h.id))) {
    say(`throughput ${hash.id}`)
    const one = await measureRate(single, hash, 2500)
    const many = await measureRate(pool, hash, 2500)
    rates.push({ hash: hash.id, workers: 1, perSecond: one }, { hash: hash.id, workers: pool.size, perSecond: many })
    rateTable.innerHTML += row([hash.id, one, many, `${(many / one).toFixed(2)}×`])
  }
  single.destroy()

  const solves: Array<{ config: SolveConfig; difficulty: number; samples: Sample[] }> = []
  const solveTable = $('solves')
  solveTable.innerHTML = row(['config', 'difficulty', 'median ms', 'p95 ms', 'max ms', 'verify µs (med)', 'failures'], true)
  for (const config of SOLVES.filter((s) => !only || only.includes(s.id))) {
    const hash = HASHES.find((h) => h.id === config.hash)!
    const rate = rates.find((r) => r.hash === hash.id && r.workers === pool.size)!.perSecond
    const difficulty = Number(params.get(`d.${config.id}`)) || Math.max(1, Math.round((rate * targetMs) / 1000 / config.k))
    const result: Sample[] = []
    for (let i = 0; i < samples; i++) {
      say(`${config.id} sample ${i + 1}/${samples}`)
      const token = await challenge(hash, config.k, difficulty, config.id)
      const solved = await pool.solve(token)
      const verify = await fetch('/api/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ solution: solved.solution, site: 'bench', action: config.id }),
      }).then((r) => r.json() as Promise<{ code: string; verify_us: number }>)
      result.push({ ms: solved.ms, attempts: solved.attempts, verifyUs: verify.verify_us, code: verify.code, at: Date.now() })
    }
    solves.push({ config, difficulty, samples: result })
    const time = summarize(result.map((s) => s.ms))
    const verifyUs = summarize(result.map((s) => s.verifyUs)).median
    const failures = result.filter((s) => s.code !== 'ok').length
    solveTable.innerHTML += row([config.id, difficulty, time.median, time.p95, time.max, verifyUs, failures])
  }
  pool.destroy()

  const report = {
    label: labelInput.value || 'unlabelled',
    simd: simdInput.checked,
    targetMs,
    device,
    coldStartMs,
    rates,
    solves: solves.map((s) => ({ ...s, summary: summarize(s.samples.map((x) => x.ms)) })),
    finishedAt: new Date().toISOString(),
  }
  const saved = await fetch('/api/results', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(report) })
  say(saved.ok ? `done, saved as ${((await saved.json()) as { file: string }).file}` : `done, saving failed: ${saved.status}`)
  document.body.dataset.done = '1'
  runButton.disabled = false
}

runButton.onclick = () => run().catch((error: unknown) => say(`error: ${String(error)}`))
if (params.get('auto') === '1') runButton.click()
