//! Success and error `MessageBar`s for admin set operations.

use leptos::prelude::*;
use orbital::primitives::{MessageBar, MessageBarIntent};

/// Success and error message bars for admin set operations.
///
/// Pass the same `RwSignal`s the parent updates from `ServerAction<SetCounter>`.
/// Renders an Orbital success bar when `admin_success` holds the new value, and
/// an error bar when `admin_error` holds a server message.
#[component]
pub fn CounterStatusMessages(
    admin_success: RwSignal<Option<usize>>,
    admin_error: RwSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <div data-testid="counter-admin-status-messages">
        {move || admin_success.get().map_or_else(|| ().into_any(), |value| view! {
            <MessageBar intent=MessageBarIntent::Success>
                "Counter updated to " {value}
            </MessageBar>
        }.into_any())}
        {move || admin_error.get().map_or_else(|| ().into_any(), |message| view! {
            <MessageBar intent=MessageBarIntent::Error>
                "Update failed: " {message}
            </MessageBar>
        }.into_any())}
        </div>
    }
}
