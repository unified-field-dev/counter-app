mod bench_run;
mod bench_valence_factory;
mod engine;
mod experiments;
mod gates;
mod ladder;
mod ops;
mod report;
mod rootcause_ladder;
mod seed;
mod setup;
mod snapshot;
mod stack;
mod stats;

use counter_app_worker as _;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use ops::{run_increment_once, run_read_once, run_write_once};

use bench_run::{run_increment_timed, warmup, BenchOp};
use engine::{BenchEngine, BenchStoreIsolation};
use experiments::{ExperimentKind, ExperimentOptions};
use ladder::{run_ladder, run_single_tier_report, LadderOptions};
use stack::{BenchRuntime, StackOptions};
use stats::MetricReport;

fn default_stack_opts(args: &Args, permission_cache: bool) -> StackOptions {
    StackOptions {
        tier: args.tier,
        engine: args.engine,
        store_isolation: args.store_isolation,
        user_id: args.user_id.clone(),
        seed_value: args.seed_value,
        permission_cache,
        data_dir: args.data_dir.clone(),
        boson_worker: args.boson_worker,
        chronon_disable_worker: args.chronon_disable_worker,
        chronon_no_jobs: args.chronon_no_jobs,
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BenchOpArg {
    Read,
    Write,
    Increment,
}

impl From<BenchOpArg> for BenchOp {
    fn from(v: BenchOpArg) -> Self {
        match v {
            BenchOpArg::Read => BenchOp::Read,
            BenchOpArg::Write => BenchOp::Write,
            BenchOpArg::Increment => BenchOp::Increment,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "counter-latency-bench",
    about = "Tiered Valence Counter/UserCounter read/write latency harness"
)]
struct Args {
    #[arg(long, value_enum, default_value_t = BenchOpArg::Increment)]
    op: BenchOpArg,

    #[arg(long, default_value_t = 100)]
    iterations: usize,

    #[arg(long, default_value_t = 10)]
    warmup: usize,

    #[arg(long, default_value = "bench-user")]
    user_id: String,

    #[arg(long, default_value_t = 0)]
    seed_value: i64,

    #[arg(long, default_value_t = 0)]
    tier: u8,

    #[arg(long)]
    ladder: bool,

    #[arg(long, default_value_t = 200.0)]
    budget_ms: f64,

    #[arg(long, default_value_t = 3)]
    repeat: usize,

    #[arg(long)]
    compare_tiers: bool,

    #[arg(long)]
    report: Option<PathBuf>,

    #[arg(long, default_value = "profiling/counter-latency-bench/tiers")]
    data_dir: PathBuf,

    #[arg(long)]
    no_permission_cache: bool,

    #[arg(long)]
    boson_worker: bool,

    #[arg(long)]
    chronon_disable_worker: bool,

    #[arg(long)]
    chronon_no_jobs: bool,

    #[arg(long, default_value_t = false)]
    json: bool,

    /// Run A/B/C Spectra persist killswitch ladder (Arena B).
    #[arg(long)]
    rootcause_ladder: bool,

    /// Phase 0 isolation experiment (volume-ramp, load-ramp, rocksdb-floor, index-ab, explain, debug-release, contention, hot-key).
    #[arg(long, value_enum)]
    experiment: Option<ExperimentKind>,

    #[arg(long, value_enum, default_value_t = BenchEngine::Rocksdb)]
    engine: BenchEngine,

    #[arg(long, value_enum, default_value_t = BenchStoreIsolation::Shared)]
    store_isolation: BenchStoreIsolation,

    #[arg(long, default_value = "0,1000,10000,100000")]
    volume_sweep: String,

    #[arg(long, default_value = "1,2,4,8,16,32")]
    concurrency_sweep: String,

    #[arg(long, default_value = "1000,10000,100000")]
    overwrite_sweep: String,

    #[arg(long)]
    raw_surreal: bool,

    #[arg(long)]
    define_index: bool,

    #[arg(long)]
    soak_seconds: Option<u64>,

    #[arg(long)]
    http_url: Option<String>,

    #[arg(long, default_value = "/api/increment_counter")]
    http_path: String,

    #[arg(long, default_value_t = 10)]
    http_duration_secs: u64,

    /// Bell-curve peak offered RPS (load-bell / raw-floor-bell).
    #[arg(long, default_value_t = 1000.0)]
    bell_peak_rps: f64,

    /// Bell-curve floor offered RPS at start/end.
    #[arg(long, default_value_t = 1.0)]
    bell_floor_rps: f64,

    /// Bell-curve experiment duration in seconds.
    #[arg(long, default_value_t = 3600)]
    bell_duration_secs: u64,

    /// Max in-flight requests (open-loop overflow when exceeded).
    #[arg(long, default_value_t = 2000)]
    bell_max_inflight: usize,

    /// Per-bucket aggregation window in seconds (default 60 = per minute).
    #[arg(long, default_value_t = 60)]
    bell_bucket_secs: u64,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(Args::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[counter-latency-bench] error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<()> {
    let permission_cache = !args.no_permission_cache;

    if let Some(kind) = args.experiment {
        let opts = ExperimentOptions {
            kind,
            engine: args.engine,
            store_isolation: args.store_isolation,
            data_dir: args.data_dir.clone(),
            user_id: args.user_id.clone(),
            seed_value: args.seed_value,
            iterations: args.iterations,
            warmup: args.warmup,
            permission_cache,
            volume_sweep: experiments::parse_usize_list(&args.volume_sweep),
            concurrency_sweep: experiments::parse_usize_list(&args.concurrency_sweep),
            overwrite_sweep: experiments::parse_usize_list(&args.overwrite_sweep),
            define_index: args.define_index,
            raw_surreal: args.raw_surreal,
            soak_seconds: args.soak_seconds,
            http_url: args.http_url.clone(),
            http_path: args.http_path.clone(),
            http_duration_secs: args.http_duration_secs,
            bell_peak_rps: args.bell_peak_rps,
            bell_floor_rps: args.bell_floor_rps,
            bell_duration_secs: args.bell_duration_secs,
            bell_max_inflight: args.bell_max_inflight,
            bell_bucket_secs: args.bell_bucket_secs,
            budget_ms: args.budget_ms,
            report: args.report.clone(),
        };
        experiments::run(&opts).await?;
        return Ok(());
    }

    if args.rootcause_ladder {
        let report = rootcause_ladder::run_rootcause_ladder(
            args.iterations.max(1),
            args.warmup,
            args.data_dir.clone(),
            args.user_id.clone(),
        )
        .await?;
        rootcause_ladder::print_report(&report);
        let report_path = args
            .report
            .unwrap_or_else(|| args.data_dir.join("arena-b-rootcause-ladder.json"));
        rootcause_ladder::write_report(&report_path, &report)?;
        return Ok(());
    }

    if args.ladder {
        let ladder_max = if args.tier == 0 {
            StackOptions::max_tier_available()
        } else {
            args.tier.min(StackOptions::max_tier_available())
        };

        run_ladder(&LadderOptions {
            max_tier: ladder_max,
            budget_ms: args.budget_ms,
            iterations: args.iterations,
            warmup: args.warmup,
            repeat: args.repeat,
            data_dir: args.data_dir.clone(),
            user_id: args.user_id.clone(),
            seed_value: args.seed_value,
            permission_cache,
            boson_worker: args.boson_worker,
            chronon_disable_worker: args.chronon_disable_worker,
            chronon_no_jobs: args.chronon_no_jobs,
            compare_tiers: args.compare_tiers,
        })
        .await?;
        return Ok(());
    }

    let stack_opts = default_stack_opts(&args, permission_cache);
    stack_opts.ensure_tier_available()?;

    if args.report.is_some() || args.tier > 0 {
        let report_path = args.report.clone().or_else(|| {
            Some(
                args.data_dir
                    .join(format!("tier-{}-run-{}.json", args.tier, args.repeat)),
            )
        });
        run_single_tier_report(
            &stack_opts,
            args.iterations,
            args.warmup,
            args.repeat,
            report_path.as_deref(),
            None,
        )
        .await?;
        return Ok(());
    }

    // Tier 0 legacy path without report (supports read/write/increment).
    let runtime = BenchRuntime::boot(&stack_opts).await?;
    let op = BenchOp::from(args.op);
    let op_name = op.as_str().to_string();

    if !args.json {
        runtime.print_header(&op_name, args.iterations, args.warmup, None);
    }

    warmup(&runtime, op, args.warmup).await?;

    match op {
        BenchOp::Read => run_read_bench(&args, &runtime, op_name).await?,
        BenchOp::Write => run_write_bench(&args, &runtime, op_name).await?,
        BenchOp::Increment => {
            if args.tier >= 1 {
                let samples = run_increment_timed(&runtime, args.iterations, args.json).await?;
                if !args.json {
                    bench_run::print_increment_results(
                        &runtime,
                        &op_name,
                        args.iterations,
                        args.warmup,
                        None,
                        &samples,
                        None,
                    );
                }
            } else {
                run_increment_bench_legacy(&args, &runtime, op_name).await?;
            }
        }
    }

    Ok(())
}

async fn run_read_bench(args: &Args, runtime: &BenchRuntime, op_name: String) -> Result<()> {
    let mut user_get = Vec::with_capacity(args.iterations);
    let mut counter_get = Vec::with_capacity(args.iterations);
    let mut total = Vec::with_capacity(args.iterations);

    for _ in 0..args.iterations {
        let step = run_read_once(&runtime.valence, &runtime.user_pk).await?;
        if args.json {
            println!("{}", serde_json::to_string(&step)?);
        }
        user_get.push(step.user_counter_get_ms);
        counter_get.push(step.counter_get_ms);
        total.push(step.total_ms);
    }

    if !args.json {
        let mut report = MetricReport::new();
        report.push("user_counter_get_ms", user_get);
        report.push("counter_get_ms", counter_get);
        report.push("read_total_ms", total);
        report.print_summary(&format!("results op={op_name}"));
    }

    Ok(())
}

async fn run_write_bench(args: &Args, runtime: &BenchRuntime, op_name: String) -> Result<()> {
    let mut user_commit = Vec::with_capacity(args.iterations);
    let mut counter_commit = Vec::with_capacity(args.iterations);
    let mut total = Vec::with_capacity(args.iterations);

    for _ in 0..args.iterations {
        let step = run_write_once(&runtime.valence, &runtime.user_pk).await?;
        if args.json {
            println!("{}", serde_json::to_string(&step)?);
        }
        user_commit.push(step.user_counter_commit_ms);
        counter_commit.push(step.counter_commit_ms);
        total.push(step.total_ms);
    }

    if !args.json {
        let mut report = MetricReport::new();
        report.push("user_counter_commit_ms", user_commit);
        report.push("counter_commit_ms", counter_commit);
        report.push("write_total_ms", total);
        report.print_summary(&format!("results op={op_name}"));
    }

    Ok(())
}

async fn run_increment_bench_legacy(
    args: &Args,
    runtime: &BenchRuntime,
    op_name: String,
) -> Result<()> {
    let mut user_get = Vec::with_capacity(args.iterations);
    let mut counter_get = Vec::with_capacity(args.iterations);
    let mut user_commit = Vec::with_capacity(args.iterations);
    let mut counter_commit = Vec::with_capacity(args.iterations);
    let mut total = Vec::with_capacity(args.iterations);

    for _ in 0..args.iterations {
        let step = run_increment_once(&runtime.valence, &runtime.user_pk).await?;
        if args.json {
            println!("{}", serde_json::to_string(&step)?);
        }
        user_get.push(step.user_counter_get_ms);
        counter_get.push(step.counter_get_ms);
        user_commit.push(step.user_counter_commit_ms);
        counter_commit.push(step.counter_commit_ms);
        total.push(step.increment_total_ms);
    }

    if !args.json {
        let mut report = MetricReport::new();
        report.push("user_counter_get_ms", user_get);
        report.push("counter_get_ms", counter_get);
        report.push("user_counter_commit_ms", user_commit);
        report.push("counter_commit_ms", counter_commit);
        report.push("increment_total_ms", total);
        report.print_summary(&format!("results op={op_name}"));
    }

    Ok(())
}
