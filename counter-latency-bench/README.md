# counter-latency-bench

Tiered Valence read/write latency harness for `Counter` / `UserCounter`. Exercises the same Valence operations as `user_counter_increment` in [`counter-app/src/counter/counter_example/server.rs`](../counter-app/src/counter/counter_example/server.rs), with optional cumulative platform layers (Boson, Spectra, Photon, Chronon, RocksDB).

## Quick start (Tier 0)

```bash
cargo run -p counter-latency-bench -- --op increment
```

## Tier ladder

| Tier | Adds | Feature flag |
|------|------|--------------|
| 0 | Valence + in-mem Surreal | (default) |
| 1 | Boson (`configure`, `build_local_without_worker`) | `tier-boson` |
| 2 | Spectra `RecordingSink` + event/gauge counts | `tier-spectra` |
| 3 | Photon runtime + continuum boot | `tier-photon` |
| 4 | Spectra composite sink (production hot path) | `tier-spectra-composite` |
| 5 | Chronon + counter-only default jobs | `tier-chronon` |
| 6 | RocksDB bench data dir | `tier-soliton` |

Full ladder: `--features tier-full`

## CLI

| Flag | Default | Description |
|------|---------|-------------|
| `--op read\|write\|increment` | `increment` | Operation to benchmark |
| `--tier 0-6` | `0` | Cumulative subsystems to boot |
| `--ladder` | off | Run tiers 0..N with evidence gates |
| `--budget-ms N` | `200` | Stop ladder when median p95 approaches N |
| `--iterations N` | `100` | Timed iterations after warmup |
| `--warmup N` | `10` | Discarded warmup iterations |
| `--repeat N` | `3` | Repeat full run; report median p95 |
| `--compare-tiers` | off | Print delta table with verdict per tier |
| `--report PATH` | — | Write tier summary JSON for EVIDENCE_LEDGER |
| `--data-dir PATH` | `profiling/counter-latency-bench/tiers/` | Artifacts root |
| `--boson-worker` | off | Tier 1 drill: spawn Boson worker thread |
| `--chronon-disable-worker` | off | Set `CHRONON_DISABLE_WORKER=1` |
| `--chronon-no-jobs` | off | Tier 5 drill: scheduler without job rows |
| `--no-permission-cache` | off | Disable Valence permission cache |
| `--json` | off | One JSON object per iteration |

### Gate capture (conclusive numbers)

```bash
cargo run -p counter-latency-bench --features tier-full -- \
  --tier 4 --op increment --iterations 100 --warmup 10 --repeat 3 \
  --report profiling/counter-latency-bench/tiers/tier-4-run.json
```

### Recursive ladder

```bash
cargo run -p counter-latency-bench --features tier-full -- \
  --ladder --budget-ms 200 --iterations 100 --warmup 10 --repeat 3
```

## Output

Every tier run emits min / p50 / p95 / max (ms) per sub-step, tagged `[counter-latency-bench]`:

```
[counter-latency-bench] tier=4 op=increment iterations=100 warmup=10 run=2/3
[counter-latency-bench]   user_counter_get_ms: ...
[counter-latency-bench]   increment_total_ms: ...
[counter-latency-bench]   spectra_events_per_iter: ...   # tier ≥ 2
[counter-latency-bench]   boson_queued: ...              # tier ≥ 1
[counter-latency-bench]   db_retry_count: ...            # tier ≥ 2 (via Spectra error log)
[counter-latency-bench] delta_vs_prev increment_total_p95=+XXXms verdict=conclusive|inconclusive
```

## Evidence ledger

Gate runs append to [`../profiling/counter-latency-bench/EVIDENCE_LEDGER.md`](../profiling/counter-latency-bench/EVIDENCE_LEDGER.md) with matching `tiers/tier-N-run-M.json` artifacts.

Conclusive rules: Δp95 thresholds, amplification correlation, drill-down flags — see EVIDENCE_LEDGER.md.

## Interpretation vs full-stack latency

| Bench increment p95 | Full-stack p95 | Conclusion |
|---------------------|----------------|------------|
| <50ms | >5000ms | Valence CRUD fast in isolation — gap is platform load |
| >1000ms at tier N | >5000ms | Tier N layer implicated with evidence row |
| Ladder tiers 0–6 all inconclusive | >5000ms | Gap outside reconstructed stack (Higgs, HTTP, Gluon jobs) |

Bookends: **3.9ms** (Tier 0) vs **~5000ms** (leptos5). See [`LANE_C_FINDINGS.md`](../profiling/counter-latency-bench/LANE_C_FINDINGS.md).

## Phase 0 isolation experiments

Use `--experiment <kind>` with `--engine rocksdb|mem` (default `rocksdb`).

| Experiment | `--experiment` | Notes |
|------------|----------------|-------|
| A volume ramp | `volume-ramp` | `--volume-sweep`, `--raw-surreal`, `--soak-seconds` |
| B load ramp | `load-ramp` | `--concurrency-sweep`, `--http-url` for full-stack |
| C RocksDB floor | `rocksdb-floor` | `--features bench-rocksdb-direct` for direct `rocksdb` crate |
| D index A/B | `index-ab` | `--define-index` |
| E EXPLAIN | `explain` | ownership pending-deletion query plan |
| F debug vs release | `debug-release` | re-run with `cargo run --release` |
| G contention | `contention` | `--features tier-chronon`, jobs off vs tier 5 on |
| H hot-key overwrite | `hot-key` | `--overwrite-sweep` |

Example:

```bash
cargo run -p counter-latency-bench -- \
  --experiment volume-ramp --engine rocksdb --raw-surreal \
  --volume-sweep 0,1000,10000,100000 --iterations 50 --warmup 5
```

Live-run snapshots: [`../scripts/perf-snapshot.sh`](../scripts/perf-snapshot.sh). Validation protocol: [`../profiling/counter-latency-bench/VALIDATION.md`](../profiling/counter-latency-bench/VALIDATION.md).

## Manual perf (when tier conclusive)

```bash
RUSTFLAGS="-C force-frame-pointers=yes" cargo run -p counter-latency-bench --features tier-full -- \
  --tier N --op increment --iterations 100 --warmup 10
perf record -g -p $(pgrep -f counter-latency-bench) sleep 30 && perf report
```
