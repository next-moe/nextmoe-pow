import { MESSAGES, type Messages, resolveLocale } from './i18n.ts'
import { solveQueued } from './pool.ts'
import { STYLES } from './styles.ts'
import { progressFraction, refreshDelay } from './timing.ts'

export type State = 'idle' | 'verifying' | 'solved' | 'error'

const TEMPLATE = `
<style>${STYLES}</style>
<div class="box" part="box" role="group">
  <p class="status" part="status"><span class="mark" part="mark" aria-hidden="true"></span><span class="text" role="status"></span></p>
  <button class="retry" part="retry" type="button" hidden></button>
  <div class="track" part="track" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow="0">
    <div class="bar" part="bar"></div>
  </div>
</div>`

export class NextmoePow extends HTMLElement {
  static formAssociated = true
  static observedAttributes = ['lang', 'challenge-url']

  #internals = this.attachInternals()
  #root = this.attachShadow({ mode: 'open' })
  #state: State = 'idle'
  #renewing = false
  #solution = ''
  #abort?: AbortController
  #run?: Promise<string>
  #refresh?: ReturnType<typeof setTimeout>
  #form: HTMLFormElement | null = null

  constructor() {
    super()
    this.#root.innerHTML = TEMPLATE
    this.#el('.retry').addEventListener('click', () => this.reset())
  }

  get solution() {
    return this.#solution
  }

  get state() {
    return this.#state
  }

  connectedCallback() {
    this.#bindForm(this.#internals.form)
    this.#render()
    if (!this.#solution) this.execute().catch(() => undefined)
  }

  disconnectedCallback() {
    this.#bindForm(null)
    this.#cancel()
  }

  formAssociatedCallback(form: HTMLFormElement | null) {
    this.#bindForm(form)
  }

  attributeChangedCallback(name: string, previous: string | null, next: string | null) {
    if (name === 'lang') this.#render()
    else if (previous !== next && this.isConnected) this.reset()
  }

  /** Resolves with the solution, starting a solve if none is running. */
  execute(): Promise<string> {
    if (this.#solution) return Promise.resolve(this.#solution)
    if (this.#run) return this.#run
    const run: Promise<string> = this.#verify().finally(() => {
      if (this.#run === run) this.#run = undefined
    })
    return (this.#run = run)
  }

  reset() {
    this.#cancel()
    this.#clearSolution()
    this.#setState('idle')
    if (this.isConnected) this.execute().catch(() => undefined)
  }

  async #verify() {
    const abort = (this.#abort = new AbortController())
    this.#setProgress(0)
    this.#setState('verifying')
    try {
      const { token, serverNow } = await this.#fetchChallenge(abort.signal)
      const result = await solveQueued(token, {
        signal: abort.signal,
        onProgress: (progress) => this.#setProgress(progressFraction(progress)),
      })
      abort.signal.throwIfAborted()
      this.#solution = result.solution
      this.#internals.setFormValue(result.solution)
      this.#renewing = false
      this.#setProgress(1)
      this.#setState('solved')
      this.#refresh = setTimeout(() => this.#expire(), refreshDelay(result.info.expiresAt, serverNow))
      this.#emit('pow-solved', { solution: result.solution })
      return result.solution
    } catch (error) {
      if (!abort.signal.aborted) {
        this.#setState('error')
        this.#emit('pow-error', { error })
      }
      throw error
    }
  }

  async #fetchChallenge(signal: AbortSignal) {
    const url = this.getAttribute('challenge-url')
    if (!url) throw new Error('nextmoe-pow: missing challenge-url')
    const res = await fetch(url, { signal, credentials: 'same-origin', headers: { accept: 'application/json' } })
    if (!res.ok) throw new Error(`nextmoe-pow: challenge request failed with ${res.status}`)
    const { token } = (await res.json()) as { token: string }
    return { token, serverNow: Date.parse(res.headers.get('date') ?? '') || Date.now() }
  }

  #expire() {
    this.#clearSolution()
    this.#renewing = true
    this.#emit('pow-expired', {})
    this.execute().catch(() => undefined)
  }

  #cancel() {
    this.#abort?.abort()
    this.#run = undefined
    clearTimeout(this.#refresh)
  }

  #clearSolution() {
    this.#solution = ''
    this.#internals.setFormValue(null)
  }

  #onSubmit = (event: SubmitEvent) => {
    if (this.#solution) return
    event.preventDefault()
    event.stopImmediatePropagation()
    const form = event.target as HTMLFormElement
    this.execute().then(
      () => form.requestSubmit(event.submitter),
      () => undefined,
    )
  }

  #bindForm(form: HTMLFormElement | null) {
    this.#form?.removeEventListener('submit', this.#onSubmit, true)
    this.#form = form
    form?.addEventListener('submit', this.#onSubmit, true)
  }

  #messages(): Messages {
    const inherited = this.closest('[lang]')?.getAttribute('lang')
    return MESSAGES[resolveLocale(this.getAttribute('lang'), inherited, document.documentElement.lang, navigator.language)]
  }

  #setState(state: State) {
    this.#state = state
    this.setAttribute('state', state)
    this.#render()
  }

  #setProgress(fraction: number) {
    this.#el('.bar').style.transform = `scaleX(${fraction})`
    this.#el('.track').setAttribute('aria-valuenow', String(Math.round(fraction * 100)))
  }

  #render() {
    const m = this.#messages()
    const text = { idle: '', verifying: this.#renewing ? m.expired : m.verifying, solved: m.verified, error: m.failed }
    this.#el('.box').setAttribute('aria-label', m.label)
    this.#el('.track').setAttribute('aria-label', m.label)
    this.#el('.track').hidden = this.#state === 'error'
    this.#el('.text').textContent = text[this.#state]
    const retry = this.#el('.retry')
    retry.textContent = m.retry
    retry.hidden = this.#state !== 'error'
  }

  #emit(type: string, detail: object) {
    this.dispatchEvent(new CustomEvent(type, { detail, bubbles: true, composed: true }))
  }

  #el<T extends HTMLElement = HTMLElement>(selector: string) {
    return this.#root.querySelector(selector) as T
  }
}
