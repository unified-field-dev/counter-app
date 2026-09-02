//! Reusable Orbital warning banner for unauthenticated call-outs.
//!
//! Prefer this when several pages need the same `MessageBar` + `Warning` intent
//! shell. Pass the message string from the page; keep product copy in the
//! caller so the component stays presentation-only.

use leptos::prelude::*;
use orbital::primitives::{MessageBar, MessageBarIntent};

/// Warning banner shown to unauthenticated users.
///
/// Displays an Orbital `MessageBar` with `Warning` intent. The caller provides
/// the message text. Optional `test_id` is accepted for call-site clarity; the
/// DOM uses a fixed `data-testid` so E2E tooling can key off a literal hook.
#[component]
// Component props are taken by value per the `#[component]` macro contract.
#[allow(clippy::needless_pass_by_value)]
pub fn AuthWarningBanner(
    #[prop(into)] message: String,
    /// Kept for call-site clarity; static `data-testid` is used so E2E coverage tooling sees a literal hook.
    #[prop(optional, into)]
    test_id: Option<String>,
) -> impl IntoView {
    let _ = test_id;
    view! {
        <div data-testid="counter-auth-warning-banner">
            <MessageBar intent=MessageBarIntent::Warning>
                {message}
            </MessageBar>
        </div>
    }
}
