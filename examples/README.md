# Examples

Runnable teaching hosts for this product. Each card: when to use · command ·
success · look next. Copy `Cargo.toml` + `main.rs` (and the product mount
snippets in the host README) into your composite host.

## Canonical path

### `local-counter-host` — local Valence + `/counter` domain

**Teaches:** protected `/counter` session gate and `counter-app-worker` get →
increment → set on sqlite-mem Valence. Inventory names match the `counter`
`uf_app!` id/path (`/counter`), `RequireAuthenticated` on admin routes, and
`CounterAdmin`.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cargo run -p local-counter-host
```

**Success:** stdout prints `local_counter_host: OK — /counter deny/allow + get/increment/set`.

**Next step:** Mount `<CounterRoutes />` in a product host. Server fns resolve
Valence with `higgs::Higgs::from_request()` then `ctx.valence()`. Launcher
metadata comes from `uf_app!` → `uf_product::AppRegistration`. Copy table +
product mount `Cargo.toml`:
[`local-counter-host/README.md`](local-counter-host/README.md).

| Host | When to use | Command | Success | Look next |
|------|-------------|---------|---------|-----------|
| [`local-counter-host`](local-counter-host/) | Local full-stack domain host | `cargo run -p local-counter-host` | Deny/allow + get/incr/set | Product host with `CounterRoutes` |
