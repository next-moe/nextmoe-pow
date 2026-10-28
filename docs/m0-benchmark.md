# M0 — algorithm benchmark

Status: **harness done; desktop and a flagship Android phone measured; mid-range and low-end Android are only inferred, so the parameters are provisional.** Every number is measured, except in the section marked "Inferred". Raw reports are in `bench/results/`.

## Acceptance

- Mid-range Android browser: median solve 1–2 s, P95 ≤ 4 s.
- Native server verification ≤ 5 ms per solution.
- WASM size budget: **≤ 40 KB gzip** for `solver_bg.wasm`. The build with both algorithms is 23.5 KB today; CI enforces the budget.

## How to run

```sh
pnpm install && pnpm build:wasm && pnpm build
(cd go && go build -o ../target/powdemo ./cmd/powdemo)
./target/powdemo -addr 0.0.0.0:8787 -root . -results bench/results
```

Open `http://<host>:8787/bench/?label=<tier>` on the device (a tier such as `midrange-android`, never a model name) and press **Run benchmark**. Add `auto=1` to start immediately and `n=<samples>` to change the sample count. A phone on USB can use `adb reverse tcp:8787 tcp:8787` and open `http://localhost:8787/bench/`. The page measures single-worker and pool throughput for each hash. It then calibrates `difficulty` so the expected solve takes `target` ms (1500 by default) and runs real solves, each verified by the Go server. The report is posted back to `bench/results/`. To pin a difficulty instead of calibrating it, pass `d.<config>=<n>`, for example `d.argon2id-1024-k4=900`.

The solve times include challenge set-up in every worker (for Argon2id, allocating its memory) but not the network round trip. Workers are created once per page, and their cold start is reported separately.

## Native (desktop: 16-core x86-64 with SHA extensions, 2026-10-27)

Rust, criterion (`cargo bench -p nextmoe-pow`), one hash:

| hash | time |
|---|---|
| SHA-256 (SHA-NI) | 34.2 ns |
| Argon2id t=1, 256 KiB | 55.7 µs |
| Argon2id t=1, 512 KiB | 104.5 µs |
| Argon2id t=1, 1 MiB | 210.6 µs |
| Argon2id t=1, 2 MiB | 412.6 µs |
| Argon2id t=1, 4 MiB | 842.0 µs |

Go, a full `Verify` including the MAC (`go test -bench Verify`, GOMAXPROCS=8):

| config | k=1 | k=2 | k=4 |
|---|---|---|---|
| SHA-256 | — | — | 1.95 µs at k=16 |
| Argon2id 256 KiB | 0.10 ms | 0.19 ms | 0.39 ms |
| Argon2id 512 KiB | 0.17 ms | 0.35 ms | 0.69 ms |
| Argon2id 1 MiB | 0.35 ms | 0.67 ms | 1.33 ms |
| Argon2id 2 MiB | 0.65 ms | 1.30 ms | 2.62 ms |
| Argon2id 4 MiB | 1.28 ms | 2.72 ms | 5.13 ms |

Go's Argon2id costs about 0.33 ms per MiB per sub-puzzle on this CPU, which is 1.6× slower than Rust's. A production host will likely be slower than this desktop, so leave headroom below the 5 ms line.

## Desktop browser (Chrome headless, same desktop, pool of 8 workers)

Throughput, baseline build (hashes/s):

| hash | 1 worker | 8 workers |
|---|---|---|
| SHA-256 | 8.70 M | 65.6 M |
| Argon2id 256 KiB | 10 486 | 79 451 |
| Argon2id 512 KiB | 5 045 | 30 900 |
| Argon2id 1 MiB | 2 319 | 20 554 |
| Argon2id 2 MiB | 1 408 | 10 307 |
| Argon2id 4 MiB | 711 | 5 457 |

A build with `+simd128` was about 14% faster for SHA-256 and 8–25% slower for Argon2id, so the default build stays scalar.

Real solves, 20 samples for each config, calibrated to 1.5 s:

| config | difficulty | median | P95 | max | Go verify, median |
|---|---|---|---|---|---|
| sha256 k=16 | 6 147 398 | 1443 ms | 2425 ms | 2804 ms | 14 µs |
| argon2id 256 KiB k=16 | 7 448 | 1628 ms | 2568 ms | 2701 ms | 2.48 ms |
| argon2id 512 KiB k=8 | 5 794 | 1169 ms | 1675 ms | 2742 ms | 2.30 ms |
| argon2id 1 MiB k=4 | 7 708 | 1502 ms | 2620 ms | 3045 ms | 1.91 ms |
| argon2id 1 MiB k=8 | 3 854 | 1470 ms | 2085 ms | 2128 ms | 3.66 ms |
| argon2id 2 MiB k=4 | 3 865 | 1091 ms | 3044 ms | 3206 ms | 3.33 ms |

The pool cold start (8 workers, WASM compile) took 22–25 ms. With 20 samples, P95 is close to the maximum and noisy. Use `n=50` or more before deciding.

## Flagship Android phone (8 cores, Android 17, Chrome 153, 2026-10-28)

This is an **upper bound, not the acceptance device**. The phone was on USB power with the screen on, ran the baseline build with a pool of 8 workers, and took 50 samples per config. The Go verifier ran on the desktop. There were three runs, and the first one was throttled.

| run | state | raw report |
|---|---|---|
| A | battery 41.6 → 39.4 °C, 26% charge; thermal status not logged | `20261028-005717-flagship-android-throttled.json` |
| B | pinned difficulty, started at 34.7 °C | `20261028-013241-flagship-android-pinned-d600.json` |
| C | full rerun, started cool at thermal status 0; status 1 (light throttling) from the second minute on, battery 36.1 → 39.9 °C | `20261028-014950-flagship-android-cool.json` |

**Correction:** the first write-up of run A treated it as this phone's normal speed. It was not. Run C's throughput phase, measured while the phone was still cool, is 1.9–3.7× faster than run A on every hash. Run A was throttled from start to finish.

Throughput (hashes/s), cool (run C) against throttled (run A):

| hash | 1 worker, cool | 8 workers, cool | 1 worker, throttled | 8 workers, throttled |
|---|---|---|---|---|
| SHA-256 | 6.49 M | 26.8 M | 1.83 M | 11.3 M |
| Argon2id 256 KiB | 7 844 | 32 843 | 2 142 | 13 312 |
| Argon2id 512 KiB | 4 111 | 16 951 | 1 121 | 6 963 |
| Argon2id 1 MiB | 1 627 | 8 085 | 571 | 3 455 |
| Argon2id 2 MiB | 675 | 3 585 | 286 | 1 642 |
| Argon2id 4 MiB | 442 | 1 490 | 139 | 785 |

Run B (cool, Argon2id 512 KiB, k=8, pinned D=600): median 294 ms, P95 500 ms, max 586 ms; 8 workers ran at 18 847/s. This agrees with run C's cool throughput, so solve time is linear in `k·D / rate`.

Solves from run A (throttled throughout; difficulty calibrated to 1.5 s against run A's own throughput):

| config | difficulty | median | P95 | max | Go verify, median |
|---|---|---|---|---|---|
| sha256 k=16 | 1 057 460 | 1517 ms | 2080 ms | 2324 ms | 14 µs |
| argon2id 256 KiB k=16 | 1 248 | 1376 ms | 2008 ms | 2281 ms | 2.78 ms |
| argon2id 512 KiB k=8 | 1 306 | 1416 ms | 2307 ms | 2704 ms | 2.61 ms |
| argon2id 1 MiB k=4 | 1 296 | 1453 ms | 3149 ms | 3805 ms | 2.20 ms |
| argon2id 1 MiB k=8 | 648 | 1725 ms | 2779 ms | 4064 ms | 3.93 ms |
| argon2id 2 MiB k=4 | 616 | 1425 ms | 3452 ms | 4872 ms | 3.68 ms |

Run C's solves were calibrated against the cool throughput but ran after the phone had heated up. Their medians came out at 2.1–4.1 s instead of 1.5 s. Argon2id slowed the most: at 512 KiB with k=8, the last five samples took 3.0–4.9 s against 1.4–2.8 s for the first five. SHA-256 drifted much less.

What these runs show, for this phone only:

- **Burst and sustained speed differ by 1.9–3.7×.** A real user solves once, usually on a phone that is not hot, so the cool numbers are the normal case and the throttled ones are the worst case. The harness calibrates at the start and then solves for minutes, which mixes the two states. Compare like with like.
- The P95/median ratio follows k and does not depend on the device: in run A it was about 1.4–1.5 at k=16, about 1.6 at k=8, and 2.2–2.4 at k=4. With k=4, a median of 2 s would put P95 above 4 s. Only k ≥ 8 leaves margin across the whole 1–2 s median band.
- When cool, this phone's 8-worker pool runs at 0.4× the desktop pool for Argon2id 1 MiB (8 085/s against 20 554/s) and at 0.55× for Argon2id 512 KiB. The earlier "6× slower" figure came from throttled run A and is withdrawn.

## Inferred mid-range and low-end Android (not measured)

There is no mid-range device available, so this section **estimates** them. It scales the flagship phone's measured cool throughput by the ratio of public Geekbench 6 CPU scores. These are predictions, not measurements, and they must be replaced by a real run before the parameters are final.

Geekbench 6 (single / multi), from public aggregates and reviews, collected 2026-10-28:

| tier | GB6 single / multi | flagship ÷ tier |
|---|---|---|
| flagship (the measured phone's SoC) | ≈ 2 300 / 6 200 (reviews and user runs: 2 276–2 362 / 6 133–6 503) | 1 |
| mid-range (two current mid-range SoCs) | 1 023–1 025 / 2 690–2 962 | 2.09–2.30 |
| entry, 5G (2 big + 6 little cores) | ≈ 940 / 2 100 (user runs 921–983 / 1 570–2 193) | 2.45–2.95 |
| entry, 4G (2 big + 6 little cores) | ≈ 720 / 2 000 (user runs 714–729 / 1 700–2 052) | 3.10–3.19 |

The best-selling Android phones of Q1–Q2 2026 sit in the two entry tiers, so those tiers matter more than the mid-range one.

For each tier, the estimate takes the flagship phone's cool 8-worker rate and divides it by the single-core and multi-core ratios; those two results bound the range. P95 is the median times the P95/median ratio measured for that k. The last column applies a 2× throttling factor to the slow end; the flagship phone's measured cool-to-throttled factor was 1.9–3.7×.

Argon2id 512 KiB, t=1, k=8, **D=1000** (k·D = 8 000 attempts on average):

| device | median | P95 | P95 if throttled 2× |
|---|---|---|---|
| flagship (measured rate, cool) | ≈ 470 ms | ≈ 770 ms | — |
| mid-range | 0.99–1.09 s | 1.6–1.8 s | ≈ 3.5 s |
| entry, 5G | 1.16–1.39 s | 1.9–2.3 s | ≈ 4.5 s |
| entry, 4G | 1.46–1.51 s | 2.4–2.5 s | ≈ 4.9 s |

At D=1250, the mid-range estimate is 1.24–1.36 s, and the entry 4G tier is 1.83–1.89 s with a P95 of about 3.0 s. For comparison, at an equal mid-range median the other configs predict worse tails: Argon2id 1 MiB with k=4 at D=1000 gives an entry-4G P95 of 3.4–3.5 s, and SHA-256 with k=16 is tight but is the configuration GPUs favor most.

Why these estimates can be wrong:

- Geekbench is a CPU proxy, while Argon2id is memory-bound. Low-end chips have small caches and slower RAM, so their real gap may be wider than the scores suggest. The size of that effect is unknown until it is measured.
- The flagship scores come from third parties rather than from this phone, and they were measured under unknown thermal conditions.
- How well the worker pool scales on chips with six little cores has not been measured, and Geekbench multi-core does not model a fully parallel workload exactly.

**Provisional recommendation:** Argon2id, m = 512 KiB, t = 1, k = 8, D = 1000. It sits at the bottom of the median band on purpose, to leave room for estimation error. It verifies in 2.1–2.6 ms through HTTP on the desktop, and 0.69 ms per 4 sub-puzzles in the Go microbenchmark. Treat it as a starting value for `agent.anon.pow_difficulty`, to be confirmed on a real mid-range device.

## Observations from the desktop run

- In WASM the browser pays about 3.4× the native cost for SHA-256 (115 ns against 34 ns with SHA-NI) and about 1.7–2× for Argon2id at 1 MiB. Even on the same CPU, an attacker running natively gets that factor for free, and the gap is much larger for SHA-256 on GPUs (not measured here).
- The Go verify times measured through the HTTP path are about 1.3–1.6× the microbenchmark (allocation and GC; Go's Argon2 allocates `m` KiB per call). Argon2id at 1 MiB with k=8 and at 2 MiB with k=4 already passes 3 ms on this CPU.
- Argon2id at 1 MiB with k=4 is the strongest candidate on paper: about 1.9 ms to verify on this CPU, and memory-hard enough to blunt GPUs. Whether it passes depends on the Android numbers.

## Pending

- [x] Flagship Android phone, as an upper bound.
- [ ] Mid-range Android, Chrome, baseline build, `n=50`: fill in throughput and solves, and record the device tier, core count and Chrome version. Also run it with a pinned difficulty (`d.<config>=…`) so phones can be compared at the same work.
- [ ] Confirm the provisional parameters (Argon2id 512 KiB, k=8, D=1000) on that device, pinned with `d.argon2id-512-k8=1000`, and set a ceiling for `agent.anon.pow_max_difficulty`.
- [ ] Harness: calibrate right before each config, and log thermal state in the report, so throttled and cool samples are not mixed.
- [ ] Measure verify on the production host class.
