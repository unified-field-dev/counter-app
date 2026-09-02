# counter-app-worker

Headless counter-example crate: Valence schemas, `#[chronon::script]` jobs,
`#[boson::task]` workers, `#[photon::topic]` definitions, side effects, and the
get/increment/set service — without Leptos.

Headless binaries (`chronon-server`, `boson-server`, `photon-server`, `server`)
link this crate so inventory matches the counter walkthrough. The UI crate
`counter-app` optionally re-exports it as `worker` under `ssr`.

## Documentation

- Workspace install / examples / verify: [repository README](../README.md)
- Discovery (Concern → API, Owns, examples ladder): `cargo doc -p counter-app-worker --open`
- Contract suite: `cargo test -p counter-app-worker --test counter_workflow_contract`
