//! Orbital metrics labels for personal and global counter values.
//!
//! Keeps `data-testid="global-counter"` on the global line so
//! [`super::delta_floaters::DeltaFloaters`] can anchor spawn origin.

use leptos::prelude::*;
use orbital::components::{Body1, SpacingSize};
use orbital::primitives::Flex;

/// Orbital metrics block for personal and/or global counter values.
///
/// Render under the live page stage. When `is_authenticated` is true, shows
/// "Your count" and "Global count" (the latter keeps `data-testid="global-counter"`
/// as the anchor for [`super::delta_floaters::DeltaFloaters`]). Anonymous
/// sessions show only the global line.
///
/// Bind `user_count` / `global_count` to the page's display signals (confirmed +
/// pending + in-flight) so optimistic clicks match what the button label shows.
#[component]
pub fn CounterMetrics(
    is_authenticated: Memo<bool>,
    user_count: Signal<usize>,
    global_count: Signal<usize>,
) -> impl IntoView {
    view! {
        <Show
            when=move || is_authenticated.get()
            fallback=move || {
                view! {
                    <div data-testid="global-counter">
                        <Body1>"Global count: " {move || global_count.get()}</Body1>
                    </div>
                }
                .into_any()
            }
        >
            <div data-testid="user-counter">
                <Flex vertical=true gap=SpacingSize::Size80.flex_gap()>
                    <div data-testid="user-count">
                        <Body1>"Your count: " {move || user_count.get()}</Body1>
                    </div>
                    <div data-testid="global-counter">
                        <Body1>"Global count: " {move || global_count.get()}</Body1>
                    </div>
                </Flex>
            </div>
        </Show>
    }
}
