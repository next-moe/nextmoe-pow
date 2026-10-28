# Flutter (`flutter/`, package `nextmoe_pow`)

A Flutter FFI plugin. `crates/core` is linked natively through flutter_rust_bridge, and cargokit builds the Rust code for Android, iOS, macOS, Linux and Windows during the app's normal build. It solves on native threads instead of WASM and uses the same protocol and vectors as the web solver.

Until the core crate is published, depend on it from git, because the plugin's Rust crate references `crates/core` by path:

```yaml
dependencies:
  nextmoe_pow:
    git: { url: https://github.com/next-moe/nextmoe-pow, path: flutter }
```

The build needs a Rust toolchain (rustup), plus the Android NDK for Android.

## Use

```dart
await initNextmoePow(); // once, before the first solve

final controller = PowController(fetchChallenge: () async => (await api.powChallenge()).token);

// Visible: starts solving when first built.
NextmoePow(controller: controller)

// Before the protected request, with or without the widget on screen:
final solution = await controller.execute();
await api.search(query, pow: solution);
controller.reset(); // a solution is single-use; start on the next one
```

- `PowController` fetches through the app's own HTTP client, solves, and renews about 10 s before expiry. A caller waiting in `execute()` across a `reset()` receives the next solution.
- Without a widget, the controller alone is the invisible mode.
- `NextmoePow` depends on the widgets layer only, with no Material. The default `NextmoePowStyle.light`/`.dark` follow the platform brightness; pass a `NextmoePowStyle` to map the colors to the app's tokens. There are no gradients.
- Languages are en, zh and ja. The `locale` argument wins, then `Localizations`, then the platform locale.
- Accessibility: the status text is a live region, and the progress bar has a semantic value. The retry button works with focus and keyboard, and the bar does not animate when the platform disables animations.
- For low-level use, `challengeInfo(token:)` decodes a token, and `solve(token:, threads:)` is a stream of progress events whose last event carries `solution`. Cancelling the subscription stops the native threads at their next progress report.

## Develop

```sh
cd flutter && flutter_rust_bridge_codegen generate   # after changing rust/src/api.rs
scripts/flutter-check.sh                              # analyze + tests, including native solve → Go verify
```

`test/native_test.dart` loads `target/release/libnextmoe_pow_flutter.so`. It decodes the shared vectors, and when `POW_DEMO_URL` is set, it solves a fresh challenge natively and checks that `go/cmd/powdemo` returns `ok` and then `replayed`.

`example/` is a demo app. Build it with `--dart-define=POW_DEMO_URL=…`, and add `--dart-define=AUTO_BENCH=true` to run 20 verified solves at start-up and log a `POWBENCH` line.

## Measured (2026-10-28)

A flagship Android phone, cool (thermal status 0), release build, arm64. Provisional parameters: Argon2id 512 KiB, k=8, D=1000, threads = cores. Each of the 20 solves was verified by powdemo over `adb reverse`, with 0 failures. Solve time from token to solution: median 317 ms, P95 496 ms, max 1182 ms. On the same phone the browser demo's median was 786 ms, but that figure also includes page load and worker start-up, so the two are not directly comparable.

That run found a bug that the widget tests had missed: the progress bar collapsed to 0 px on the device. It is fixed, and a test now checks the bar's size.
