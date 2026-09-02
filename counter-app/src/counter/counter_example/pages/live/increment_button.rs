//! Batched increment control for the live counter page.
//!
//! Owns the idle / max flush timers and dispatches Higgs `IncrementCounter`
//! through a Leptos `ServerAction`. See [`super::batch`] for the timing policy.

use std::time::Duration;

use crate::counter::counter_example::server::IncrementCounter;
use leptos::leptos_dom::helpers::TimeoutHandle;
use leptos::prelude::*;
use orbital::primitives::{Button, ButtonType, MessageBar, MessageBarIntent};

use super::batch::{IDLE_FLUSH_MS, MAX_FLUSH_MS};

fn clear_timer(store: StoredValue<Option<TimeoutHandle>>) {
    store.update_value(|handle| {
        if let Some(h) = handle.take() {
            h.clear();
        }
    });
}

/// Increment control with local pending batching and Orbital error feedback.
///
/// Wire this under [`super::CounterExamplePage`] with a shared
/// `ServerAction<IncrementCounter>`. Each click bumps `pending_local` so the
/// displayed count moves immediately; a single Higgs `increment_counter` /
/// user-increment dispatch runs after [`super::batch::IDLE_FLUSH_MS`] idle or
/// [`super::batch::MAX_FLUSH_MS`] since the first click in the batch.
///
/// Pass `in_flight` so a second batch arms timers only after the previous
/// server action settles. Failures render an Orbital `MessageBar`.
#[component]
pub fn IncrementButton(
    increment_action: ServerAction<IncrementCounter>,
    display_global: Signal<usize>,
    pending_local: RwSignal<usize>,
    in_flight: RwSignal<usize>,
    error_message: Signal<Option<String>>,
) -> impl IntoView {
    let idle_timer = StoredValue::new(None::<TimeoutHandle>);
    let max_timer = StoredValue::new(None::<TimeoutHandle>);
    let prev_in_flight = StoredValue::new(0usize);

    let flush = move || {
        let pending = pending_local.get_untracked();
        if pending == 0 || in_flight.get_untracked() > 0 {
            return;
        }
        clear_timer(idle_timer);
        clear_timer(max_timer);
        pending_local.set(0);
        in_flight.set(pending);
        // One Higgs server call for the whole batch (`IncrementCounter` / amount).
        increment_action.dispatch(IncrementCounter { amount: pending });
    };

    let arm_idle = move || {
        clear_timer(idle_timer);
        if let Ok(handle) =
            set_timeout_with_handle(move || flush(), Duration::from_millis(IDLE_FLUSH_MS))
        {
            idle_timer.set_value(Some(handle));
        }
    };

    let arm_max = move || {
        clear_timer(max_timer);
        if let Ok(handle) =
            set_timeout_with_handle(move || flush(), Duration::from_millis(MAX_FLUSH_MS))
        {
            max_timer.set_value(Some(handle));
        }
    };

    on_cleanup(move || {
        clear_timer(idle_timer);
        clear_timer(max_timer);
    });

    // When an in-flight flush settles and clicks arrived during flight, arm timers.
    Effect::new(move |_| {
        let flying = in_flight.get();
        let prev = prev_in_flight.get_value();
        prev_in_flight.set_value(flying);
        if prev > 0 && flying == 0 && pending_local.get_untracked() > 0 {
            arm_idle();
            arm_max();
        }
    });

    let on_click = Callback::new(move |_| {
        let start_max = pending_local.get_untracked() == 0;
        pending_local.update(|n| *n += 1);
        if in_flight.get_untracked() > 0 {
            // Flush slot busy; Effect arms timers when in_flight clears.
            return;
        }
        arm_idle();
        if start_max {
            arm_max();
        }
    });

    view! {
        <div data-testid="increment-button">
            <Button button_type=ButtonType::Button on_click=on_click>
                "Click Me: " {move || display_global.get()}
            </Button>
        </div>
        {move || error_message
            .get().map_or_else(|| {
                let _: () = view! { <></> };
                ().into_any()
            }, |message| {
                view! {
                    <MessageBar intent=MessageBarIntent::Error>
                        "Increment failed: " {message}
                    </MessageBar>
                }
                .into_any()
            })}
    }
}
