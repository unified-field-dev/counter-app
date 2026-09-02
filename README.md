# Counter App

[![CI](https://github.com/unified-field-dev/counter-app/actions/workflows/ci.yml/badge.svg)](https://github.com/unified-field-dev/counter-app/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[GitHub](https://github.com/unified-field-dev/counter-app) · `cargo doc -p counter-app-worker --open`

## About

Counter App is a Unified Field **example product**: Valence schemas, Chronon
scripts, Boson side effects, Photon topics, and a Leptos UI at `/counter`. Use
it as a readable reference when composing those families into your own host.

- **UI (`counter-app`)** — pages, Higgs `#[server]` wrappers, Photon live
  updates, `CounterRoutes`
- **Worker (`counter-app-worker`)** — Valence models, get/increment/set service,
  Chronon/Boson/Photon inventory (no Leptos)
- **Spectra (`counter-app-spectra-topics`)** — typed recorders for request and
  server-error metrics

Crate-root rustdoc owns **Features** and primary-task guides. Open docs with:

```bash
export CARGO_TARGET_DIR=target-counter-app
cargo doc -p counter-app-worker -p counter-app-spectra-topics --no-deps
cargo doc -p counter-app --features ssr --no-deps
```

Prefer `cargo doc -p counter-app-worker --open` for the domain contract. UI
rustdoc needs the `ssr` feature and is pin-dependent on Orbital / host graphs.

## Getting started

```toml
[dependencies]
# Pin tag or rev — do not use branch = "main".
counter-app = { git = "https://github.com/unified-field-dev/counter-app", package = "counter-app", rev = "REPLACE_WITH_PIN", default-features = false }
counter-app-worker = { git = "https://github.com/unified-field-dev/counter-app", package = "counter-app-worker", rev = "REPLACE_WITH_PIN" }
```

```rust,ignore
use counter_app::CounterRoutes;
use leptos_router::components::Routes;

view! {
    <Routes fallback=|| "not found">
        <CounterRoutes />
    </Routes>
}
```

Register Chronon/Boson/Photon pieces from `counter-app-worker` in host bootstrap,
then mount the routes above. Full Leptos SSR hosts live outside this repository;
use the local teaching host for the domain contract.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cargo test -p counter-app-worker --test counter_workflow_contract
```

## Workspace

| Crate | Role |
|-------|------|
| [`counter-app`](counter-app/) | Leptos UI + `CounterRoutes` + app registration |
| [`counter-app-worker`](counter-app-worker/) | Valence schemas, service, Chronon/Boson/Photon |
| [`counter-app-spectra-topics`](counter-app-spectra-topics/) | Spectra topic-name helpers |
| [`local-counter-host`](examples/local-counter-host/) | Teaching host: deny/allow + get/incr/set |

## Examples

| Host | When to use | Command | Success | Look next |
|------|-------------|---------|---------|-----------|
| [`local-counter-host`](examples/local-counter-host/) | Local Valence + `/counter` domain | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-counter-app cargo run -p local-counter-host` | Deny/allow + get/incr/set | Mount `CounterRoutes` |

Copy table + product mount `Cargo.toml`:
[`examples/local-counter-host/README.md`](examples/local-counter-host/README.md).
More examples: [`examples/README.md`](examples/README.md).

## Security

Demo write posture, anon rate limits, and WebSocket notes:
[`SECURITY.md`](SECURITY.md). Report vulnerabilities privately — do not open a
public issue for security-sensitive reports.

## Verify

GitHub Actions (`.github/workflows/ci.yml`) runs Layer 1 (fmt, clippy, contract
tests, teaching host, rustdoc) **and** Layer 2 Playwright (`e2e` job) from
[`docs/VERIFICATION.md`](docs/VERIFICATION.md).

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cargo fmt -p counter-app-worker -p local-counter-host -- --check
cargo clippy -p counter-app-worker --all-targets -- -D warnings
cargo clippy -p local-counter-host --all-targets -- -D warnings
cargo test -p counter-app-worker --test workspace_members --test product_surface
cargo test -p counter-app-worker --test counter_workflow_contract
cargo check -p local-counter-host
cargo run -p local-counter-host
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc \
  -p counter-app-worker -p counter-app-spectra-topics --no-deps
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc \
  -p counter-app --features ssr --no-deps
```

Teaching host success line:
`local_counter_host: OK — /counter deny/allow + get/increment/set`.
Contribute: [`CONTRIBUTING.md`](CONTRIBUTING.md).

## FAQ

**Is it a standalone server?** No. `counter-app` mounts into a composite host
that already wires Valence, session chrome, and Higgs. `local-counter-host`
exercises the domain contract without the full SSR/WASM graph.

**Do I need the UI crate?** No. Headless Chronon/Boson/Photon binaries can
depend on `counter-app-worker` alone. Mount `CounterRoutes` when you want the
Leptos pages.

**How do server fns get Valence?** Call `higgs::Higgs::from_request()` then
`ctx.valence()` (same pattern as other uf-apps). App launcher metadata comes
from `uf_app!` → `uf_product::AppRegistration`.

**Why are some writes public?** Demo defaults for anon increment — see [`SECURITY.md`](SECURITY.md).
`counter_set` requires `CounterAdmin`; hosts must wire Gauge and grant admins.

## License

MIT. See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md), and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
