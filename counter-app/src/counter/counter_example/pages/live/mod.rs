//! Live counter page: Photon refetch, Orbital metrics, batched increments.
//!
//! [`CounterExamplePage`] is the default `/counter` leaf. It teaches the client
//! half of Photon-leptos `#[synced]`: `subscribe_counter_get` yields a
//! `ws_trigger` that keys a Leptos `Resource` calling `counter_get` or
//! `user_counter_get` depending on Orbital [`AuthSession`]. Local clicks batch
//! through `IncrementButton` before a single `ServerAction` flush.

mod batch;
mod counter_metrics;
mod delta_floaters;
mod increment_button;

use counter_metrics::CounterMetrics;
use delta_floaters::DeltaFloaters;
use increment_button::IncrementButton;

use super::session_display_label;
use crate::counter::counter_example::server::{
    counter_get, subscribe_counter_get, user_counter_get, CounterData, IncrementCounter,
};
use leptos::prelude::*;
use orbital::components::{Body1, ContentContainer, SkeletonItemSize, SpacingSize, Title3};
use orbital::primitives::{Box, Flex, FlexAlign, MessageBar, MessageBarIntent, SkeletonItem};
use orbital::services::permission_server_errors::report_server_fn_error;
use orbital::{use_auth_context, use_auth_state, AuthSession};

/// Live counter page for authenticated and anonymous sessions.
///
/// Call when rendering the `/counter` leaf. Flow:
/// 1. `use_auth_state` / `use_auth_context` for session and reload token.
/// 2. `subscribe_counter_get` (Photon-leptos client helper from `#[synced]` on
///    `counter_get`) for a `ws_trigger` that bumps on `CounterUpdated`.
/// 3. `Resource` keyed on `(reload_token, auth, ws_trigger)` calling
///    `user_counter_get` or `counter_get`.
/// 4. Confirmed vs pending vs in-flight signals so the UI feels instant while
///    `IncrementButton` batches clicks into one `IncrementCounter` action.
///
/// Sticky `show_main` keeps `DeltaFloaters` mounted across WS-driven refetches
/// so remote deltas still animate. Permission denials go through Orbital
/// `report_server_fn_error`; other failures surface in a `MessageBar`.
#[component]
pub fn CounterExamplePage() -> impl IntoView {
    // Orbital auth: reactive session + reload token (login/logout invalidates).
    let auth_state = use_auth_state();
    let user_label = Memo::new(move |_| auth_state.with(session_display_label));
    let is_authenticated =
        Memo::new(move |_| auth_state.with(orbital::AuthSession::is_authenticated));

    let auth_ctx = use_auth_context();
    let reload_token = auth_ctx.reload_token();

    // Photon-leptos client helper from `#[synced]` on `counter_get`.
    // Bumps when `CounterUpdated` publishes; SSR stays at 0 (single fetch).
    let ws_trigger = subscribe_counter_get(|| {});

    // Key includes ws_trigger so a remote publish refetches without a full page reload.
    let counter_data = Resource::new(
        move || (reload_token.get(), auth_state.get(), ws_trigger.get()),
        |(_, session, _)| async move {
            match session {
                AuthSession::Authenticated(_) => user_counter_get().await.map(CounterData::User),
                AuthSession::Anonymous(_) => counter_get().await.map(CounterData::Global),
            }
        },
    );

    let confirmed_global = RwSignal::new(0usize);
    let confirmed_user = RwSignal::new(0usize);
    // Optimistic UI: pending clicks + in-flight flush while ServerAction runs.
    let pending_local = RwSignal::new(0usize);
    let in_flight = RwSignal::new(0usize);
    let increment_error = RwSignal::new(None::<String>);
    let increment_action = ServerAction::<IncrementCounter>::new();

    let display_global =
        Signal::derive(move || confirmed_global.get() + pending_local.get() + in_flight.get());
    let display_user =
        Signal::derive(move || confirmed_user.get() + pending_local.get() + in_flight.get());

    let _sync_resource = Effect::new(move |_| {
        if let Some(Ok(data)) = counter_data.get() {
            match data {
                CounterData::Global(resp) => {
                    confirmed_global.set(resp.value);
                    confirmed_user.set(0);
                }
                CounterData::User(resp) => {
                    confirmed_user.set(resp.user_count);
                    confirmed_global.set(resp.global_count);
                }
            }
        }
    });

    let _sync_increment_action = Effect::new(move |_| {
        if let Some(result) = increment_action.value().get() {
            match result {
                Ok(CounterData::Global(resp)) => {
                    increment_error.set(None);
                    confirmed_global.set(resp.value);
                    confirmed_user.set(0);
                    in_flight.set(0);
                }
                Ok(CounterData::User(resp)) => {
                    increment_error.set(None);
                    confirmed_user.set(resp.user_count);
                    confirmed_global.set(resp.global_count);
                    in_flight.set(0);
                }
                Err(err) => {
                    let n = in_flight.get_untracked();
                    if n > 0 {
                        pending_local.update(|p| *p += n);
                        in_flight.set(0);
                    }
                    if report_server_fn_error(&err) {
                        // Orbital: permission / auth denials get a shared toast path.
                        increment_error.set(None);
                    } else {
                        increment_error.set(Some(err.to_string()));
                    }
                }
            }
        }
    });

    let heading_text = Signal::derive(move || {
        if is_authenticated.get() {
            format!("Welcome back, {}", user_label.get())
        } else {
            "Welcome to Unified Field!".to_string()
        }
    });

    let error_message = Signal::derive(move || increment_error.get());
    let confirmed_global_signal = Signal::derive(move || confirmed_global.get());

    // Sticky UI gates: Transition handles SSR/hydrate Resource pending state.
    // These signals only flip on first Ok / Err so the ready tree (and DeltaFloaters)
    // is not remounted on every WS-driven Resource refetch.
    let show_main = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let _gate_resource = Effect::new(move |_| match counter_data.get() {
        Some(Ok(_)) => {
            show_main.set(true);
            load_error.set(None);
        }
        Some(Err(err)) => {
            load_error.set(Some(err.to_string()));
        }
        None => {}
    });

    view! {
        <ContentContainer max_width="640px" data_testid="counter-container">
            <Flex vertical=true align=FlexAlign::Center gap=SpacingSize::Size160.flex_gap()>
                <Title3>{move || heading_text.get()}</Title3>
                <Transition fallback=move || view! {
                    <SkeletonItem size=Signal::from(SkeletonItemSize::S32) width="240px".to_string() />
                }>
                    {move || {
                        let main = || {
                            view! {
                                <Flex vertical=true align=FlexAlign::Center gap=SpacingSize::Size120.flex_gap()>
                                    // G7: `position: relative` anchors delta floaters; Orbital Box has no position prop.
                                    <Box style="position: relative; width: 100%;">
                                        <Flex vertical=true align=FlexAlign::Center full_width=true>
                                            <DeltaFloaters confirmed_global=confirmed_global_signal />
                                            <CounterMetrics
                                                is_authenticated=is_authenticated
                                                user_count=display_user
                                                global_count=display_global
                                            />
                                        </Flex>
                                    </Box>
                                    <IncrementButton
                                        increment_action=increment_action
                                        display_global=display_global
                                        pending_local=pending_local
                                        in_flight=in_flight
                                        error_message=error_message
                                    />
                                </Flex>
                            }
                            .into_any()
                        };

                        // After first success, avoid reading `counter_data` so WS refetches
                        // do not remount DeltaFloaters (which would skip remote deltas).
                        if show_main.get() {
                            if let Some(message) = load_error.get() {
                                return view! {
                                    <MessageBar intent=MessageBarIntent::Error>
                                        "Failed to load counter: " {message}
                                    </MessageBar>
                                }
                                .into_any();
                            }
                            return main();
                        }

                        // Track the Resource here so Transition can suspend for SSR/hydrate.
                        match counter_data.get() {
                            None => ().into_any(),
                            Some(Err(err)) => view! {
                                <MessageBar intent=MessageBarIntent::Error>
                                    "Failed to load counter: " {err.to_string()}
                                </MessageBar>
                            }
                            .into_any(),
                            Some(Ok(_)) => main(),
                        }
                    }}
                </Transition>
            </Flex>
        </ContentContainer>
    }
}
