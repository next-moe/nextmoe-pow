import { NextmoePow } from './element.ts'

export { NextmoePow, type State } from './element.ts'
export { type Locale, MESSAGES } from './i18n.ts'
export { configure, warmUp } from './pool.ts'

if (!customElements.get('nextmoe-pow')) customElements.define('nextmoe-pow', NextmoePow)

declare global {
  interface HTMLElementTagNameMap {
    'nextmoe-pow': NextmoePow
  }
}
