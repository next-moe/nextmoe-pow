import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { Solver, initSync } from '../wasm/solver.js'

initSync({ module: readFileSync(new URL('../wasm/solver_bg.wasm', import.meta.url)) })
const vectors = JSON.parse(readFileSync(new URL('../../../testdata/vectors.json', import.meta.url), 'utf8'))

const hex = (bytes: Uint8Array) => Buffer.from(bytes).toString('hex')

test('challenge fields decode', () => {
  for (const c of vectors.challenges) {
    const s = new Solver(c.token)
    assert.deepEqual(
      { alg: s.alg, k: s.k, difficulty: s.difficulty, expiresAt: s.expiresAt, site: s.site, action: s.action },
      { alg: c.alg.name, k: c.k, difficulty: c.difficulty, expiresAt: c.expires_at, site: c.site, action: c.action },
    )
    assert.equal(s.memoryKib, c.alg.memory_kib ?? 0)
    assert.equal(s.iterations, c.alg.iterations ?? 0)
  }
})

test('sub-puzzle hashes', () => {
  for (const h of vectors.hashes) {
    assert.equal(hex(new Solver(h.token).hash(h.sub, BigInt(h.nonce))), h.hash)
  }
})

test('reference solutions are reproduced', () => {
  for (const c of vectors.verify.filter((v: { name: string }) => /^[\w-]+\/ok$/.test(v.name) && !v.name.startsWith('rotated'))) {
    const token = c.solution.slice(0, c.solution.lastIndexOf('.'))
    const s = new Solver(token)
    const nonces = Array.from({ length: s.k }, (_, sub) => s.search(sub, 0, 2 ** 31)!)
    assert.equal(s.solution(Float64Array.from(nonces)), c.solution, c.name)
  }
})

test('malformed tokens are rejected', () => {
  assert.throws(() => new Solver('not a token'), /malformed/)
})
