# counter-app

Leptos UI for the Counter example: pages, Higgs `#[server]` wrappers, Photon live
updates, and `CounterRoutes` registration via `uf_app!`.

Domain schemas, get/increment/set, and Chronon/Boson/Photon inventory live in
sibling [`counter-app-worker`](../counter-app-worker/).

## Documentation

- Workspace install / examples / verify: [repository README](../README.md)
- Discovery (Concern → API, Owns, examples ladder): `cargo doc -p counter-app --features ssr --open`
  (pin-dependent on Orbital / host graphs; prefer `counter-app-worker` when the UI graph is broken)
- Demo security posture: [`../SECURITY.md`](../SECURITY.md)

## Getting started

```rust,ignore
use counter_app::CounterRoutes;
use leptos_router::components::Routes;

view! {
    <Routes fallback=|| "not found">
        <CounterRoutes />
    </Routes>
}
```
