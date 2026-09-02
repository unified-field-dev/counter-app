//! Photon topic for live counter updates — defined in `counter-app-worker`.
//!
//! The UI crate does not own the Photon event type. Under `ssr`, re-export
//! `CounterUpdated` from the worker so server functions can
//! `CounterUpdated { new_value }.publish()` after a successful mutate. The live
//! page's Photon-leptos `#[synced]` on `counter_get` (topic `counter.updated`)
//! then bumps the client `ws_trigger` and refetches.
//!
//! Publish from the host after Valence write succeeds when connected browsers
//! should see the new global value without polling. Skip publish when
//! `COUNTER_NOOP_PUBLISH` is set (load tests).

#[cfg(feature = "ssr")]
pub use crate::worker::events::CounterUpdated;
