use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::bench_run::{run_increment_timed, warmup, BenchOp, IncrementSamples};
use crate::engine::BenchEngine;
use crate::gates::{
    amplification_correlates, budget_breached, classify_delta, delta_ms, overall_verdict_label,
    AmplificationVerdict, DeltaVerdict,
};
use crate::report::{collect_env_flags, git_sha_short, TierReport};
use crate::stack::{BenchRuntime, StackOptions};
use crate::stats::Stats;

const FULL_STACK_ANCHOR_MS: f64 = 5000.0;

pub struct LadderOptions {
    pub max_tier: u8,
    pub budget_ms: f64,
    pub iterations: usize,
    pub warmup: usize,
    pub repeat: usize,
    pub data_dir: PathBuf,
    pub user_id: String,
    pub seed_value: i64,
    pub permission_cache: bool,
    pub boson_worker: bool,
    pub chronon_disable_worker: bool,
    pub chronon_no_jobs: bool,
    pub compare_tiers: bool,
}

pub struct LadderResult {
    pub reports: Vec<TierReport>,
    pub stopped_at_tier: u8,
    pub stop_reason: String,
}

pub async fn run_ladder(opts: &LadderOptions) -> Result<LadderResult> {
    let mut reports = Vec::new();
    let mut prev_increment_p95: Option<f64> = None;
    let mut prev_events_p95: Option<f64> = None;
    let mut prev_gauges_p95: Option<f64> = None;
    let mut prev_boson_p95: Option<f64> = None;
    let mut prev_retry_p95: Option<f64> = None;
    let mut stopped_at = 0u8;
    let mut stop_reason = String::from("exhausted tiers");

    for tier in 0..=opts.max_tier {
        stopped_at = tier;
        let mut run_p95s = Vec::new();
        let mut last_samples: Option<IncrementSamples> = None;

        for run_idx in 1..=opts.repeat {
            let stack_opts = StackOptions {
                tier,
                engine: BenchEngine::Rocksdb,
                store_isolation: crate::engine::BenchStoreIsolation::Shared,
                user_id: opts.user_id.clone(),
                seed_value: opts.seed_value,
                permission_cache: opts.permission_cache,
                data_dir: opts.data_dir.clone(),
                boson_worker: opts.boson_worker,
                chronon_disable_worker: opts.chronon_disable_worker,
                chronon_no_jobs: opts.chronon_no_jobs,
            };

            let runtime = BenchRuntime::boot(&stack_opts).await?;
            runtime.print_header(
                "increment",
                opts.iterations,
                opts.warmup,
                Some((run_idx, opts.repeat)),
            );
            warmup(&runtime, BenchOp::Increment, opts.warmup).await?;

            let samples = run_increment_timed(&runtime, opts.iterations, false).await?;
            let inc_p95 = Stats::summarize(samples.increment_total_ms.clone()).p95;
            run_p95s.push(inc_p95);
            last_samples = Some(samples);

            crate::bench_run::print_increment_results(
                &runtime,
                "increment",
                opts.iterations,
                opts.warmup,
                None,
                last_samples.as_ref().unwrap(),
                prev_increment_p95,
            );
        }

        let median_p95 = Stats::median(&run_p95s);
        let samples = last_samples.expect("at least one repeat");

        let delta_v = classify_delta(prev_increment_p95, median_p95);
        let events_p95 = Stats::summarize(samples.spectra_events_per_iter.clone()).p95;
        let gauges_p95 = Stats::summarize(samples.spectra_gauges_per_iter.clone()).p95;
        let boson_p95 = Stats::summarize(samples.boson_queued.clone()).p95;
        let retry_p95 = Stats::summarize(samples.db_retry_count.clone()).p95;
        let retry_sleep_p95 = Stats::summarize(samples.db_retry_total_sleep_ms.clone()).p95;

        let amp_v = amplification_correlates(
            delta_v,
            prev_events_p95,
            events_p95,
            prev_gauges_p95,
            gauges_p95,
            prev_boson_p95,
            boson_p95,
            prev_retry_p95,
            retry_p95,
            retry_sleep_p95,
        );

        let txt_path = opts
            .data_dir
            .join(format!("tier-{}-run-{}.txt", tier, opts.repeat));
        let json_path = opts
            .data_dir
            .join(format!("tier-{}-run-{}.json", tier, opts.repeat));

        let report = TierReport {
            tier,
            run: opts.repeat,
            repeat_total: opts.repeat,
            git_sha: git_sha_short(),
            op: "increment".to_string(),
            iterations: opts.iterations,
            warmup: opts.warmup,
            env: collect_env_flags(),
            increment_total: Stats::summarize(samples.increment_total_ms.clone()).into(),
            user_counter_get_ms: Stats::summarize(samples.user_counter_get_ms.clone()).into(),
            counter_get_ms: Stats::summarize(samples.counter_get_ms.clone()).into(),
            user_counter_commit_ms: Stats::summarize(samples.user_counter_commit_ms.clone()).into(),
            counter_commit_ms: Stats::summarize(samples.counter_commit_ms.clone()).into(),
            spectra_events_per_iter: Stats::summarize(samples.spectra_events_per_iter.clone())
                .into(),
            spectra_gauges_per_iter: Stats::summarize(samples.spectra_gauges_per_iter.clone())
                .into(),
            boson_queued: Stats::summarize(samples.boson_queued.clone()).into(),
            db_retry_count: Stats::summarize(samples.db_retry_count.clone()).into(),
            db_retry_total_sleep_ms: Stats::summarize(samples.db_retry_total_sleep_ms.clone())
                .into(),
            prev_tier: prev_increment_p95.map(|_| tier.saturating_sub(1)),
            delta_increment_p95_ms: delta_ms(prev_increment_p95, median_p95),
            delta_verdict: delta_v,
            amplification_verdict: amp_v,
            overall_verdict: overall_verdict_label(delta_v, amp_v).to_string(),
            artifact_txt: Some(txt_path.display().to_string()),
            artifact_json: Some(json_path.display().to_string()),
        };

        write_text_artifact(&txt_path, &report, median_p95, &run_p95s)?;
        report.write_json(&json_path)?;
        reports.push(report.clone());

        if opts.compare_tiers {
            print_compare_row(&report, median_p95);
        }

        if budget_breached(median_p95, opts.budget_ms, FULL_STACK_ANCHOR_MS) {
            stop_reason = format!(
                "budget breach: median increment p95={median_p95:.1}ms (budget={}ms)",
                opts.budget_ms
            );
            break;
        }

        if delta_v == DeltaVerdict::Conclusive
            && amp_v == AmplificationVerdict::InconclusiveMechanism
        {
            stop_reason = format!(
                "tier {tier}: conclusive latency delta without amplification — run drill-down before advancing"
            );
            break;
        }

        prev_increment_p95 = Some(median_p95);
        prev_events_p95 = Some(events_p95);
        prev_gauges_p95 = Some(gauges_p95);
        prev_boson_p95 = Some(boson_p95);
        prev_retry_p95 = Some(retry_p95);
    }

    println!("[counter-latency-bench] ladder stopped_at_tier={stopped_at} reason={stop_reason}");

    Ok(LadderResult {
        reports,
        stopped_at_tier: stopped_at,
        stop_reason,
    })
}

fn write_text_artifact(
    path: &Path,
    report: &TierReport,
    median_p95: f64,
    run_p95s: &[f64],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    out.push_str(&format!(
        "tier={} verdict={} median_increment_p95={:.1} run_p95s={:?}\n",
        report.tier, report.overall_verdict, median_p95, run_p95s
    ));
    out.push_str(&format!(
        "delta_increment_p95_ms={:?}\n",
        report.delta_increment_p95_ms
    ));
    std::fs::write(path, out)?;
    Ok(())
}

fn print_compare_row(report: &TierReport, median_p95: f64) {
    println!(
        "[counter-latency-bench] compare tier={} median_p95={:.1} delta={:?} verdict={}",
        report.tier, median_p95, report.delta_increment_p95_ms, report.overall_verdict
    );
}

pub async fn run_single_tier_report(
    stack_opts: &StackOptions,
    iterations: usize,
    warmup_iters: usize,
    repeat: usize,
    report_path: Option<&Path>,
    prev_p95: Option<f64>,
) -> Result<TierReport> {
    let mut run_p95s = Vec::new();
    let mut last_samples: Option<IncrementSamples> = None;

    for run_idx in 1..=repeat {
        let runtime = BenchRuntime::boot(stack_opts).await?;
        runtime.print_header(
            "increment",
            iterations,
            warmup_iters,
            Some((run_idx, repeat)),
        );
        crate::bench_run::warmup(&runtime, BenchOp::Increment, warmup_iters).await?;
        let samples = run_increment_timed(&runtime, iterations, false).await?;
        run_p95s.push(Stats::summarize(samples.increment_total_ms.clone()).p95);
        crate::bench_run::print_increment_results(
            &runtime,
            "increment",
            iterations,
            warmup_iters,
            None,
            &samples,
            prev_p95,
        );
        last_samples = Some(samples);
    }

    let samples = last_samples.unwrap();
    let median_p95 = Stats::median(&run_p95s);
    let delta_v = classify_delta(prev_p95, median_p95);

    let report = TierReport {
        tier: stack_opts.tier,
        run: repeat,
        repeat_total: repeat,
        git_sha: git_sha_short(),
        op: "increment".to_string(),
        iterations,
        warmup: warmup_iters,
        env: collect_env_flags(),
        increment_total: Stats::summarize(samples.increment_total_ms.clone()).into(),
        user_counter_get_ms: Stats::summarize(samples.user_counter_get_ms.clone()).into(),
        counter_get_ms: Stats::summarize(samples.counter_get_ms.clone()).into(),
        user_counter_commit_ms: Stats::summarize(samples.user_counter_commit_ms.clone()).into(),
        counter_commit_ms: Stats::summarize(samples.counter_commit_ms.clone()).into(),
        spectra_events_per_iter: Stats::summarize(samples.spectra_events_per_iter.clone()).into(),
        spectra_gauges_per_iter: Stats::summarize(samples.spectra_gauges_per_iter.clone()).into(),
        boson_queued: Stats::summarize(samples.boson_queued.clone()).into(),
        db_retry_count: Stats::summarize(samples.db_retry_count.clone()).into(),
        db_retry_total_sleep_ms: Stats::summarize(samples.db_retry_total_sleep_ms.clone()).into(),
        prev_tier: prev_p95.map(|_| stack_opts.tier.saturating_sub(1)),
        delta_increment_p95_ms: delta_ms(prev_p95, median_p95),
        delta_verdict: delta_v,
        amplification_verdict: AmplificationVerdict::NotApplicable,
        overall_verdict: overall_verdict_label(delta_v, AmplificationVerdict::NotApplicable)
            .to_string(),
        artifact_txt: report_path.map(|p| p.with_extension("txt").display().to_string()),
        artifact_json: report_path.map(|p| p.display().to_string()),
    };

    if let Some(path) = report_path {
        report.write_json(path)?;
        let txt = path.with_extension("txt");
        write_text_artifact(&txt, &report, median_p95, &run_p95s)?;
    }

    Ok(report)
}
