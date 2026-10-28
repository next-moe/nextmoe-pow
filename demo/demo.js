import '@nextmoe/pow-widget'

const verify = async (solution, action) => {
  const started = performance.now()
  const res = await fetch('/api/verify', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ solution, site: 'demo', action }),
  })
  const body = await res.json()
  return `${body.code} (server verify ${body.verify_us} µs, round trip ${(performance.now() - started).toFixed(0)} ms)`
}

let lastSolution = ''
const search = document.getElementById('search')
search.addEventListener('submit', async (event) => {
  event.preventDefault()
  lastSolution = new FormData(search).get('nextmoe-pow')
  document.getElementById('search-result').value = await verify(lastSolution, 'search')
  search.querySelector('nextmoe-pow').reset()
})
document.getElementById('replay').addEventListener('click', async () => {
  document.getElementById('search-result').value = lastSolution ? await verify(lastSolution, 'search') : 'submit once first'
})

const invisible = document.getElementById('invisible')
document.getElementById('run-invisible').addEventListener('click', async () => {
  const out = document.getElementById('invisible-result')
  out.value = 'working…'
  const solution = await invisible.execute()
  out.value = await verify(solution, 'invisible')
  invisible.reset()
})

document.getElementById('lang').addEventListener('change', (event) => {
  for (const el of document.querySelectorAll('nextmoe-pow')) el.setAttribute('lang', event.target.value)
})

for (const el of document.querySelectorAll('nextmoe-pow')) {
  el.addEventListener('pow-error', (event) => console.error('pow-error', event.detail.error))
}
