# local-counter-host

Canonical **local** host for the counter product: in-memory Valence +
`counter-app-worker` get/increment/set under protected **`/counter`**.

Production Leptos hosts mount `CounterRoutes` at **`/counter`** and resolve
request Valence via `higgs::Higgs::from_request()` then `ctx.valence()`. This
example proves the same path + auth + domain contract without the SSR/WASM /
Orbital graph. The oneshot path `/counter` matches the Orbital app id/path
(`counter` / `/counter`).

| | |
|---|---|
| **When to use** | First smoke of counter domain wiring in a local host |
| **Command** | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-counter-app cargo run -p local-counter-host` |
| **Success** | Stdout: `local_counter_host: OK — /counter deny/allow + get/increment/set` |
| **Look next** | Mount [`CounterRoutes`](../../counter-app/) ; register Chronon/Boson from `counter-app-worker` |

**Open first:** [`src/main.rs`](src/main.rs)

## Copy into your host

| File | What to take |
|------|----------------|
| This [`Cargo.toml`](Cargo.toml) | Axum oneshot shape + `counter-app-worker` (get/increment/set smoke) |
| Product mount `Cargo.toml` (below) | `counter-app` + worker with `ssr` / `hydrate` |
| [`src/main.rs`](src/main.rs) | Session gate on `/counter`, Valence mem router, get → increment → set |
| Leptos sketch (below) | `<CounterRoutes />` under `/counter` |

### Product mount dependencies

```toml
[dependencies]
counter-app = { git = "https://github.com/unified-field-dev/counter-app", package = "counter-app", rev = "REPLACE_WITH_PIN", default-features = false }
counter-app-worker = { git = "https://github.com/unified-field-dev/counter-app", package = "counter-app-worker", rev = "REPLACE_WITH_PIN" }
uf-product = { /* your pin */, default-features = false }
uf-integrations = { /* your pin */, default-features = false }

[features]
ssr = [
    "counter-app/ssr",
    "uf-product/ssr",
    "uf-integrations/ssr",
]
hydrate = [
    "counter-app/hydrate",
    "uf-product/hydrate",
    "uf-integrations/hydrate",
]
```

### Leptos mount sketch

```rust,ignore
use counter_app::CounterRoutes;
use leptos_router::components::Routes;

view! {
    <Routes fallback=|| "not found">
        <CounterRoutes />
    </Routes>
}
```

Domain service (Leptos-free):

```rust,ignore
use counter_app_worker::service::{get_global, increment_global, set_global};

let empty = get_global(&valence).await?;
let after_incr = increment_global(7, &valence).await?;
let after_set = set_global(42, &valence).await?;
```

Request-scoped Valence in a Higgs server fn:

```rust,ignore
use leptos::prelude::ServerFnError;

#[higgs_macros::server]
pub async fn my_endpoint() -> Result<(), ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    let db = ctx
        .valence()
        .map_err(|e| ServerFnError::new(format!("Failed to build Valence: {e}")))?;
    let value = counter_app_worker::service::get_global(&db).await?;
    println!("global={value}");
    Ok(())
}
```

Launcher metadata for product hosts comes from `uf_app!` →
`uf_product::AppRegistration` / `AppRegistry::auto_discover()`. Inventory names
match `counter` / `/counter`. Admin pages use `RequireAuthenticated` + email
verification; the permission manifest includes `CounterAdmin`. Register
Chronon/Boson/Photon pieces from `counter-app-worker` in host bootstrap
alongside the routes.

For shell chrome (layout, fonts, Axum + Leptos boot), copy
[`shell-chrome-host`](https://github.com/unified-field-dev/unified-field-product/tree/main/examples/shell-chrome-host)
from unified-field-product, then mount `CounterRoutes`.

## Run (documented gate)

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cargo check -p local-counter-host
cargo run -p local-counter-host
```

**Success:** stdout prints `local_counter_host: OK — /counter deny/allow + get/increment/set`.

## Hydrate / browser

Out of gate for this host. Full counter UI needs a product binary with
`cargo-leptos`, `wasm32`, session chrome, Higgs, and a working Orbital /
`uf-product` graph. Prefer the oneshot above.
