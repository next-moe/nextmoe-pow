# `<nextmoe-pow>` widget

A framework-free Web Component (`@nextmoe/pow-widget`). It fetches a challenge, solves it in Web Workers and puts the solution into its form. It does not depend on KunUI, and it styles itself only through CSS custom properties.

```html
<script type="module">import '@nextmoe/pow-widget'</script>

<form method="post" action="/search">
  <input name="q" />
  <nextmoe-pow name="pow" challenge-url="/api/pow/challenge?action=agent.search"></nextmoe-pow>
  <button>Search</button>
</form>
```

The server reads the `pow` field and passes it to `pow.VerifyOnce`.

## Behavior

- It starts solving as soon as it is connected. One worker pool is shared by every widget on the page, and solves run one at a time.
- It is form-associated: the solution is submitted with the form under `name`, and a form reset keeps it.
- If the form is submitted before the solve finishes, the widget holds the submit, waits for the solution and then submits again with the same submitter. Its listener runs in the capture phase on the form, so framework submit handlers see only the submit that carries the solution.
- Shortly before the challenge expires, it clears the value and solves a fresh challenge. Expiry is computed from the challenge response's `Date` header, so a client clock that is minutes off cannot cause a refresh loop.
- Code that sends the request itself instead of submitting a form calls `await el.execute()`.

## API

| attribute | |
|---|---|
| `challenge-url` | GET endpoint returning `{"token": "<challenge token>"}`. Changing it restarts the widget. |
| `name` | form field name |
| `mode` | `auto` (default; visible status and progress) or `invisible` (no UI, same behavior) |
| `lang` | `en`, `zh` or `ja`. Otherwise the nearest `[lang]` ancestor, then `<html lang>`, then the browser language. |

| member | |
|---|---|
| `execute(): Promise<string>` | the current solution, or the one being worked on |
| `reset()` | drop the solution and solve a new challenge |
| `solution`, `state` | `state` is `idle`, `verifying`, `solved` or `error`, and is also reflected as an attribute |

Events bubble and are composed: `pow-solved` (`detail.solution`), `pow-error` (`detail.error`), `pow-expired`.

`configure({ size, wasmUrl, workerUrl })` (exported) overrides the pool before the first widget starts; `warmUp()` starts the workers early.

## Styling

Custom properties, each with light and dark defaults: `--nextmoe-pow-background`, `-foreground`, `-muted`, `-border`, `-track`, `-accent`, `-success`, `-danger`, `-radius`, `-padding`, `-width`, `-font-family`, `-font-size`. Each site maps them to its own tokens. Shadow parts: `box`, `status`, `mark`, `retry`, `track`, `bar`.

## Accessibility

The state text is a `role="status"` live region and is announced on state changes only, not on every progress tick. Progress is a labelled `role="progressbar"`. On failure the widget shows a real `<button>` for retry. The progress animation is disabled under `prefers-reduced-motion`. The user has nothing to solve or click.

## Measured

- `packages/widget/test/e2e`: 5 end-to-end tests against `go/cmd/powdemo`, passing in Chrome and Firefox (2026-10-28).
- A flagship Android phone in Chrome, cool (thermal status 0), demo page, provisional parameters (Argon2id 512 KiB, k=8, D=1000), 20 reloads. Time from navigation to the first `pow-solved`, including page load, worker start-up, WASM compile and the challenge request: median 786 ms, P95 1052 ms, max 1078 ms. Go verified these solutions in 1.9–3.0 ms through HTTP on the 16-core desktop.

## Demo

```sh
pnpm install && pnpm build:wasm && pnpm build
(cd go && go build -o ../target/powdemo ./cmd/powdemo)
./target/powdemo -root .   # open http://localhost:8787/demo/
```
