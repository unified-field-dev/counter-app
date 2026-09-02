//! High scores page: Orbital chrome around the paginated leaderboard table.
//!
//! [`HighScoresPage`] is the `/counter/high-scores` leaf. It reads Orbital
//! auth for the heading and anonymous warning, then mounts
//! [`crate::counter::components::HighScoresTable`], which pages Valence
//! `UserCounter` rows through a Higgs server function.

use super::session_display_label;
use crate::counter::components::HighScoresTable;
use leptos::prelude::*;
use orbital::components::{ContentContainer, SpacingSize, Title3};
use orbital::primitives::{Flex, MessageBar, MessageBarIntent};
use orbital::{use_auth_context, use_auth_state};

/// High scores page: paginated leaderboard of user counters.
///
/// Mount via [`crate::HighScoresRoute`]. Waits for Orbital `session_loaded`
/// before showing the anonymous warning bar. Authenticated users get a hidden
/// E2E marker; everyone sees the leaderboard title and [`HighScoresTable`].
#[component]
pub fn HighScoresPage() -> impl IntoView {
    let auth_state = use_auth_state();
    let auth = use_auth_context();
    let session_loaded = auth.session_loaded();
    let user_label = Memo::new(move |_| auth_state.with(session_display_label));
    let is_authenticated =
        Memo::new(move |_| auth_state.with(orbital::AuthSession::is_authenticated));

    view! {
        <ContentContainer max_width="720px" data_testid="high-scores-page">
            <Flex vertical=true fill=true full_width=true gap=SpacingSize::Size240.flex_gap()>
                {move || {
                    if !session_loaded.get() {
                        return ().into_any();
                    }
                    if is_authenticated.get() {
                        view! {
                            <span data-testid="high-scores-logged-in-marker" style="display:none" aria-hidden="true" />
                        }.into_any()
                    } else {
                        view! {
                            <div data-testid="high-scores-auth-warning">
                                <MessageBar intent=MessageBarIntent::Warning>
                                    "Sign in to track your score and join the leaderboard."
                                </MessageBar>
                            </div>
                        }.into_any()
                    }
                }}
                <Title3 test_id="high-scores-heading">
                    {move || format!("Leaderboard for {}", user_label.get())}
                </Title3>
                <HighScoresTable />
            </Flex>
        </ContentContainer>
    }
}
