import assert from 'node:assert/strict'
import { test } from 'node:test'
import { resolveLocale } from '../src/i18n.ts'
import { progressFraction, refreshDelay } from '../src/timing.ts'

test('locale falls through candidates to English', () => {
  assert.equal(resolveLocale('zh-CN'), 'zh')
  assert.equal(resolveLocale(null, '', 'ja-JP', 'zh'), 'ja')
  assert.equal(resolveLocale('fr', undefined), 'en')
})

test('progress never goes backwards past solved sub-puzzles and stops short of done', () => {
  assert.equal(progressFraction({ solved: 0, k: 8, attempts: 0, expectedAttempts: 8000 }), 0)
  assert.equal(progressFraction({ solved: 2, k: 8, attempts: 100, expectedAttempts: 8000 }), 0.25)
  assert.equal(progressFraction({ solved: 7, k: 8, attempts: 20000, expectedAttempts: 8000 }), 0.95)
  assert.equal(progressFraction({ solved: 8, k: 8, attempts: 20000, expectedAttempts: 8000 }), 1)
})

test('refresh waits until shortly before expiry but never loops on a skewed clock', () => {
  assert.equal(refreshDelay(1_000_300, 1_000_000_000), 290_000)
  assert.equal(refreshDelay(1_000_300, 1_000_400_000), 30_000)
})
