//! Photon event types for the counter system.
//!
//! Topics are declared with `#[photon::topic]` so inventory registration runs when
//! this crate is linked into a Photon host. UI server functions call `.publish()`
//! after a successful global write; photon-leptos clients subscribe for live
//! refetch. Topics live in the worker (no Leptos); the UI crate only consumes them.

/// Published when the global counter value changes (increment or set).
///
/// Unlike keyed topics such as `user.notifications`, this topic is unkeyed —
/// every connected client receives each event. Field `new_value` is the
/// persisted singleton after the write.
///
/// # Examples
///
/// ```rust,ignore
/// use counter_app_worker::events::CounterUpdated;
///
/// CounterUpdated { new_value: 42 }.publish().await?;
/// ```
// Photon inventory topic — wire name must match UI `#[synced(topic = "counter.updated")]`.
#[photon::topic(name = "counter.updated")]
pub struct CounterUpdated {
    /// The counter's value after the increment or set that triggered this event.
    pub new_value: usize,
}
