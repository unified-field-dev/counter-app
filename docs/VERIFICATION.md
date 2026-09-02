# counter-app verification

Re-run after code or doc changes. This workspace is the Counter product
(`counter-app` Leptos UI + `counter-app-worker` schemas/jobs/service) plus
product crates. Layer 1 covers the product-local counter service that backs
`CounterRoutes` server functions (`counter_get`, `counter_increment`,
`counter_set`, `user_counter_get`, `user_counter_increment`), Chronon/Boson
script cores, high-score ordering, sibling-source UI needles (smoke), and
helper-crate contracts. Layer 2 is the `counter-ui-e2e` Leptos + Playwright
lab (Photon WS, seed API, validating browser scenarios). Cloud fleets stay
out of scope. Valence owns persistence primitives; this repo verifies the
counter mapping / validation layer in `counter-app-worker::service`.

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
```

## Teaching host

Axum oneshot under [`examples/local-counter-host`](../examples/local-counter-host/).
Copy table + product mount sketches live
in that host README.

```bash
cargo check -p local-counter-host
cargo run -p local-counter-host
```

Success line: `local_counter_host: OK — /counter deny/allow + get/increment/set`.
Hydrate/browser is out of gate for the oneshot (`cargo-leptos` + `wasm32` +
Orbital / `uf-product` belong to a composite product host).

## Layer 1 — Unit + integration (CI)

GitHub Actions (`.github/workflows/ci.yml`) covers this Layer 1 subset plus the
teaching host and worker / helper rustdoc gates below. It does not build
`counter-app` (Leptos UI).

Sibling-source UI needles (smoke — greps routes/testids/auth without compiling
Orbital). Do not treat these as primary happy/sad coverage:

```bash
cargo test -p counter-app-worker --test workspace_members --test product_surface
```

Backend contracts (preferred path; no UI graph):

```bash
cargo fmt -p counter-app-worker -p local-counter-host -- --check
cargo clippy -p counter-app-worker --all-targets -- -D warnings
cargo clippy -p local-counter-host --all-targets -- -D warnings
cargo test -p counter-app-worker --test counter_workflow_contract
cargo test -p counter-app-worker --test high_scores_contract
cargo test -p counter-app-worker --test scripts_contract
```

CI clippy uses `-- -D warnings` on the same packages (job clears global `RUSTFLAGS`
only when needed; deny is via the clippy flag).

### leptos-lints (local; hydrate UI)

Needs `cargo-dylint` / `dylint-link` 6.0.1 and toolchain `nightly-2025-05-14`
(see `leptos-lints@v0.1.2`). Workspace `[workspace.metadata.dylint]` pins the
library; rustc deny names are declared under `[workspace.lints.rust]`.

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview

export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"

cargo dylint --all -p counter-app --no-deps -- --features hydrate
```

Hard CI job deferred: `counter-app` hydrate still depends on the Orbital / host
graph (same pin risk as UI compile in Layer 1). Run locally when that graph is
green.

`counter-app` (Leptos UI + Higgs `#[server]` wrappers) remains pin-dependent on
the Orbital / host graph. Prefer worker + helper crates for CI contract gates;
treat UI-crate compile failures as a separate host product issue, not a counter
service gap. Existing UI-crate binaries (`app_metadata_test`, `server_fn_paths`,
`error_mapping`, …) are optional when the UI graph compiles.

Workspace notes:

- `counter-latency-bench` is excluded from the workspace (Surreal-era harness).
- Axum Photon WS lives in `photon-axum` (photon-leptos); former `photon-wiring-axum`
  kit is archived at https://github.com/deathbreakfast/photon-wiring-axum-archive.

## Layer 2 — E2E (`counter-ui-e2e`)

Required for teaching-surface correctness. GitHub Actions job `e2e` runs this
Layer 2 gate on PRs and pushes to `main` / `master` (see `.github/workflows/ci.yml`).
Lab host under
[`examples/counter-ui-e2e`](../examples/counter-ui-e2e/): Axum + Leptos
SSR/hydrate, in-memory Valence, Gauge manifest sync, `counter_admin` permission
group membership for the owner seed user, Photon WS, `POST /api/test/seed-data`. Scenario
catalog and run notes live in that package README.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cd examples/counter-ui-e2e/end2end && npm ci && npx playwright install chromium
cd ../../..
cargo leptos end-to-end --project counter-ui-e2e
```

Host: `127.0.0.1:3000`. Do not Ctrl-C; wait for Playwright to exit.

L5 `unified-field-embedded` still ships a one-click composition smoke
(`counter-click-demo.spec.ts`). Product feature depth stays in `counter-ui-e2e`.

## Layer 3 — Cloud + performance

**Waived.** L3-local product; no cloud resources. `counter-latency-bench` is
excluded from the workspace (pre-router Surreal harness). Correctness is
in-process against an embedded SQLite `:memory:` Valence for Layer 1, plus the
Layer 2 lab host above.

## Rustdoc

Workspace `Cargo.toml` allows `broken_intra_doc_links` by default. Honest local
deny for worker, Spectra helpers, and the UI crate (`ssr`):

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc \
  -p counter-app-worker -p counter-app-spectra-topics --no-deps
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc \
  -p counter-app --features ssr --no-deps
```

For guide-contract audits, point `CARGO_TARGET_DIR` at the same tree as
`uf-docs-guide-contracts/workspaces/counter-app/doc-guide-spec.toml` `doc_root`
(typically `uf-docs-data/target-counter-app`).

`counter-app` still uses `#![allow(missing_docs)]` on macro-heavy UI surfaces
(`uf_app!`, `orbital_routes_extract`). Hand-written items carry teaching rustdoc.

## Notes

- Prefer `cargo test -p counter-app-worker` (named binaries above) for backend
  CI. Treat `product_surface` / `workspace_members` as smoke gates only.
- Layer 2 scenarios are listed in `examples/counter-ui-e2e/README.md`.
- Tests may `unwrap`/`expect`; production paths map failures to
  `CounterServiceError` / `ServerFnError` (no ordinary-path unwrap).
- Sad-path assertions check message content — stronger than `is_err()` alone.
- Happy-path tests are named `*_happy_path` so audits detect them.
- `CounterRoutes` pages call the `#[server]` fns; those fns wrap `service::*`.
