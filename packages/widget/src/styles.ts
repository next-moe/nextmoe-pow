export const STYLES = `
:host {
  --_bg: var(--nextmoe-pow-background, #ffffff);
  --_fg: var(--nextmoe-pow-foreground, #18181b);
  --_muted: var(--nextmoe-pow-muted, #71717a);
  --_border: var(--nextmoe-pow-border, #e4e4e7);
  --_track: var(--nextmoe-pow-track, #f4f4f5);
  --_accent: var(--nextmoe-pow-accent, #2563eb);
  --_success: var(--nextmoe-pow-success, #16a34a);
  --_danger: var(--nextmoe-pow-danger, #dc2626);
  display: block;
  max-width: var(--nextmoe-pow-width, 320px);
  font-family: var(--nextmoe-pow-font-family, system-ui, sans-serif);
  font-size: var(--nextmoe-pow-font-size, 14px);
  line-height: 1.4;
  color: var(--_fg);
}
@media (prefers-color-scheme: dark) {
  :host {
    --_bg: var(--nextmoe-pow-background, #18181b);
    --_fg: var(--nextmoe-pow-foreground, #f4f4f5);
    --_muted: var(--nextmoe-pow-muted, #a1a1aa);
    --_border: var(--nextmoe-pow-border, #3f3f46);
    --_track: var(--nextmoe-pow-track, #27272a);
    --_accent: var(--nextmoe-pow-accent, #60a5fa);
    --_success: var(--nextmoe-pow-success, #4ade80);
    --_danger: var(--nextmoe-pow-danger, #f87171);
  }
}
:host([mode='invisible']), [hidden] { display: none !important; }
.box {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 8px 12px;
  padding: var(--nextmoe-pow-padding, 12px 14px);
  background: var(--_bg);
  border: 1px solid var(--_border);
  border-radius: var(--nextmoe-pow-radius, 8px);
}
.status { margin: 0; display: flex; align-items: center; gap: 8px; }
.mark { flex: none; width: 16px; height: 16px; border-radius: 50%; border: 2px solid var(--_muted); box-sizing: border-box; }
:host([state='solved']) .mark { border-color: var(--_success); background: var(--_success); }
:host([state='error']) .mark { border-color: var(--_danger); background: var(--_danger); }
.track { grid-column: 1 / -1; height: 4px; border-radius: 2px; background: var(--_track); overflow: hidden; }
.bar { height: 100%; width: 100%; background: var(--_accent); transform-origin: left; transform: scaleX(0); transition: transform 120ms linear; }
:host([state='solved']) .bar { background: var(--_success); }
.retry {
  font: inherit;
  color: var(--_accent);
  background: transparent;
  border: 1px solid currentColor;
  border-radius: var(--nextmoe-pow-radius, 8px);
  padding: 2px 10px;
  cursor: pointer;
}
.retry:focus-visible { outline: 2px solid var(--_accent); outline-offset: 2px; }
@media (prefers-reduced-motion: reduce) { .bar { transition: none; } }
`
