#[cfg(feature = "tier-boson")]
pub mod tier1_boson;
#[cfg(feature = "tier-spectra")]
pub mod tier2_spectra;
#[cfg(feature = "tier-photon")]
pub mod tier3_photon;
#[cfg(feature = "tier-spectra-composite")]
pub mod tier4_composite;
#[cfg(feature = "tier-chronon")]
pub mod tier5_chronon;
#[cfg(feature = "tier-soliton")]
pub mod tier6_rocksdb;

pub mod counting_sink;
pub mod tier0;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Result};
use valence::{DatabaseRouter, SDb, Valence};

use crate::engine::{self, BenchEngine, BenchStoreIsolation};

pub struct StackOptions {
    pub tier: u8,
    pub engine: BenchEngine,
    pub store_isolation: BenchStoreIsolation,
    pub user_id: String,
    pub seed_value: i64,
    pub permission_cache: bool,
    pub data_dir: PathBuf,
    pub boson_worker: bool,
    pub chronon_disable_worker: bool,
    pub chronon_no_jobs: bool,
}

impl StackOptions {
    pub fn max_tier_available() -> u8 {
        #[cfg(feature = "tier-full")]
        {
            return 6;
        }
        #[cfg(all(not(feature = "tier-full"), feature = "tier-soliton"))]
        {
            return 6;
        }
        #[cfg(all(
            not(feature = "tier-full"),
            not(feature = "tier-soliton"),
            feature = "tier-chronon"
        ))]
        {
            return 5;
        }
        #[cfg(all(
            not(feature = "tier-full"),
            not(feature = "tier-chronon"),
            feature = "tier-spectra-composite"
        ))]
        {
            return 4;
        }
        #[cfg(all(
            not(feature = "tier-full"),
            not(feature = "tier-spectra-composite"),
            feature = "tier-photon"
        ))]
        {
            return 3;
        }
        #[cfg(all(
            not(feature = "tier-full"),
            not(feature = "tier-photon"),
            feature = "tier-spectra"
        ))]
        {
            return 2;
        }
        #[cfg(all(
            not(feature = "tier-full"),
            not(feature = "tier-spectra"),
            feature = "tier-boson"
        ))]
        {
            return 1;
        }
        0
    }

    pub fn ensure_tier_available(&self) -> Result<()> {
        if self.tier > Self::max_tier_available() {
            bail!(
                "tier {} requires Cargo features (max available: {}). \
                 Use --features tier-full for the full ladder.",
                self.tier,
                Self::max_tier_available()
            );
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct BenchRuntime {
    pub tier: u8,
    pub valence: Valence,
    pub user_pk: String,
    pub router: Arc<DatabaseRouter>,
    pub db: SDb,
    pub permission_cache: bool,
    pub data_dir: PathBuf,
    pub boson_worker: bool,
    pub chronon_disable_worker: bool,
    pub chronon_no_jobs: bool,
    #[cfg(feature = "tier-boson")]
    pub boson: Option<Arc<boson_runtime::Boson>>,
    #[cfg(feature = "tier-photon")]
    pub photon_executor: Option<()>,
    #[cfg(feature = "tier-chronon")]
    pub chronon: Option<Arc<chronon::runtime::ChrononRuntime>>,
    #[cfg(feature = "tier-spectra")]
    pub spectra_recording: Option<spectra_core::RecordingSink>,
}

impl BenchRuntime {
    pub async fn boot(opts: &StackOptions) -> Result<Self> {
        opts.ensure_tier_available()?;
        tier0::assert_schemas_linked()?;

        let (db, router) = if opts.tier >= 6 {
            engine::db_and_router(opts.engine, &opts.data_dir, opts.store_isolation).await?
        } else {
            engine::db_and_router(opts.engine, &opts.data_dir, opts.store_isolation).await?
        };

        let (valence, user_pk) = tier0::seed_counters(
            router.clone(),
            &opts.user_id,
            opts.seed_value,
            opts.permission_cache,
        )
        .await?;

        let mut runtime = BenchRuntime {
            tier: opts.tier,
            valence,
            user_pk,
            router: router.clone(),
            db: db.clone(),
            permission_cache: opts.permission_cache,
            data_dir: opts.data_dir.clone(),
            boson_worker: opts.boson_worker,
            chronon_disable_worker: opts.chronon_disable_worker,
            chronon_no_jobs: opts.chronon_no_jobs,
            #[cfg(feature = "tier-boson")]
            boson: None,
            #[cfg(feature = "tier-photon")]
            photon_executor: None,
            #[cfg(feature = "tier-chronon")]
            chronon: None,
            #[cfg(feature = "tier-spectra")]
            spectra_recording: None,
        };

        if opts.tier >= 1 {
            #[cfg(feature = "tier-boson")]
            tier1_boson::boot(&mut runtime, opts).await?;
            #[cfg(not(feature = "tier-boson"))]
            bail!("tier 1 requires feature tier-boson");
        }

        if opts.tier >= 2 {
            #[cfg(feature = "tier-spectra")]
            tier2_spectra::boot(&mut runtime).await?;
            #[cfg(not(feature = "tier-spectra"))]
            bail!("tier 2 requires feature tier-spectra");
        }

        if opts.tier >= 3 {
            #[cfg(feature = "tier-photon")]
            tier3_photon::boot(&mut runtime).await?;
            #[cfg(not(feature = "tier-photon"))]
            bail!("tier 3 requires feature tier-photon");
        }

        if opts.tier >= 4 {
            #[cfg(feature = "tier-spectra-composite")]
            tier4_composite::boot(&mut runtime).await?;
            #[cfg(not(feature = "tier-spectra-composite"))]
            bail!("tier 4 requires feature tier-spectra-composite");
        }

        if opts.chronon_disable_worker {
            std::env::set_var("CHRONON_DISABLE_WORKER", "1");
        }

        if opts.tier >= 5 {
            #[cfg(feature = "tier-chronon")]
            tier5_chronon::boot(&mut runtime, opts).await?;
            #[cfg(not(feature = "tier-chronon"))]
            bail!("tier 5 requires feature tier-chronon");
        }

        Ok(runtime)
    }

    pub fn print_header(
        &self,
        op: &str,
        iterations: usize,
        warmup: usize,
        run: Option<(usize, usize)>,
    ) {
        let run_suffix = run
            .map(|(n, total)| format!(" run={n}/{total}"))
            .unwrap_or_default();
        println!(
            "[counter-latency-bench] tier={} op={} iterations={} warmup={} actor={} permission_cache={}{}",
            self.tier,
            op,
            iterations,
            warmup,
            tier0::actor_label(&self.user_pk),
            self.permission_cache,
            run_suffix,
        );
        if self.tier >= 1 {
            println!(
                "[counter-latency-bench]   boson_worker={}",
                self.boson_worker
            );
        }
        if self.tier >= 5 {
            println!(
                "[counter-latency-bench]   chronon_disable_worker={} chronon_no_jobs={}",
                self.chronon_disable_worker, self.chronon_no_jobs
            );
        }
        self.print_env_flags();
    }

    fn print_env_flags(&self) {
        for key in [
            "SPECTRA_CONSOLE",
            "SPECTRA_COMPOSITE_PERSIST",
            "SPECTRA_SYNC_HOT_PATH",
            "CHRONON_DISABLE_WORKER",
        ] {
            if let Ok(v) = std::env::var(key) {
                println!("[counter-latency-bench]   env {key}={v}");
            }
        }
    }

    pub async fn snapshot(&self) -> crate::snapshot::StackSnapshot {
        crate::snapshot::capture(self).await
    }
}

pub fn actor_label(user_id: &str) -> String {
    tier0::actor_label(user_id)
}
