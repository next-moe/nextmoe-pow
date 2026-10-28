import assert from 'node:assert/strict'
import { after, before, test } from 'node:test'
import { type Browser, type Page, chromium, firefox } from 'playwright'

const base = process.env.POW_DEMO_URL ?? 'http://localhost:8787'
const engine = process.env.POW_BROWSER === 'firefox' ? firefox : chromium
let browser: Browser

before(async () => {
  browser = await engine.launch(engine === chromium ? { channel: 'chrome' } : {})
})
after(() => browser.close())

const open = async () => {
  const page = await browser.newPage()
  const errors: string[] = []
  page.on('pageerror', (e) => errors.push(e.message))
  await page.goto(`${base}/demo/`)
  return { page, errors }
}

const solved = (page: Page, selector: string) =>
  page.waitForFunction((s) => document.querySelector(s)?.getAttribute('state') === 'solved', selector, { timeout: 30_000 })

test('auto mode solves, submits with the form and is verified once', async () => {
  const { page, errors } = await open()
  await solved(page, '#search nextmoe-pow')
  const widget = page.locator('#search nextmoe-pow')
  assert.equal(await widget.locator('[role=status]').textContent(), 'Verified')
  assert.equal(await widget.locator('[role=progressbar]').getAttribute('aria-valuenow'), '100')
  await page.click('#search button:not([type])')
  await page.waitForFunction(() => document.getElementById('search-result')?.textContent !== '')
  assert.match(await page.locator('#search-result').textContent() ?? '', /^ok /)
  await page.click('#replay')
  await page.waitForFunction(() => document.getElementById('search-result')?.textContent?.startsWith('replayed'))
  assert.deepEqual(errors, [])
  await page.close()
})

test('submitting before the solve finishes waits and then submits', async () => {
  const { page } = await open()
  await page.click('#search button:not([type])')
  await page.waitForFunction(() => document.getElementById('search-result')?.textContent !== '', null, { timeout: 30_000 })
  assert.match(await page.locator('#search-result').textContent() ?? '', /^ok /)
  await page.close()
})

test('invisible mode renders nothing and solves on demand', async () => {
  const { page } = await open()
  assert.equal(await page.locator('#invisible').isVisible(), false)
  await page.click('#run-invisible')
  await page.waitForFunction(() => document.getElementById('invisible-result')?.textContent?.startsWith('ok'), null, { timeout: 30_000 })
  await page.close()
})

test('language follows the lang attribute', async () => {
  const { page } = await open()
  await solved(page, '.themed')
  await page.selectOption('#lang', 'zh')
  assert.equal(await page.locator('.themed [role=status]').textContent(), '验证完成')
  await page.selectOption('#lang', 'ja')
  assert.equal(await page.locator('.themed [role=status]').textContent(), '確認しました')
  await page.close()
})

test('a failing challenge endpoint shows an error with a working retry button', async () => {
  const { page } = await open()
  await page.evaluate(() => {
    const el = document.createElement('nextmoe-pow')
    el.id = 'broken'
    el.setAttribute('challenge-url', '/api/missing')
    document.body.append(el)
  })
  await page.waitForFunction(() => document.getElementById('broken')?.getAttribute('state') === 'error')
  const retry = page.locator('#broken .retry')
  assert.equal(await retry.textContent(), 'Retry')
  await page.evaluate(() =>
    document.getElementById('broken')!.setAttribute('challenge-url', '/api/challenge?alg=2&m=512&t=1&k=8&d=50&site=demo&action=x'),
  )
  await solved(page, '#broken')
  await page.close()
})
