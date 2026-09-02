# Contributing to Counter App

## Development setup

1. Clone [unified-field-dev/counter-app](https://github.com/unified-field-dev/counter-app)
2. Install Rust stable
3. From the repository root:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-counter-app
cargo fmt -p counter-app-worker -- --check
cargo test -p counter-app-worker --test counter_workflow_contract
```

Full gates: [`docs/VERIFICATION.md`](docs/VERIFICATION.md).

## Code of conduct

Participation is governed by [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Security
reports: [`SECURITY.md`](SECURITY.md).

## Pull requests

- Prefer small, focused PRs.
- Update [`README.md`](README.md) when public API or host mounting steps change.
