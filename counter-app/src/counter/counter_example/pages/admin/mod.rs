//! Admin page: set the global counter to an explicit value.
//!
//! The lazy route wraps this view in [`uf_product::routes::RequireAuthenticated`]
//! with verified email and `CounterAdmin`. `SetCounter` / `counter_set` enforces
//! the same permission on the server (SECURITY.md).

mod counter_set_form;
mod counter_status_messages;

use counter_set_form::CounterSetForm;
use counter_status_messages::CounterStatusMessages;

use crate::counter::counter_example::server::{counter_get, SetCounter};
use leptos::prelude::*;
use orbital::components::{Body1, ContentContainer, SkeletonItemSize, SpacingSize, Title3};
use orbital::primitives::{Flex, MessageBar, MessageBarIntent, SkeletonItem};
use orbital::{use_auth_state, AuthSession};

/// Admin page: set the global counter to an explicit value.
///
/// Loads the current value with `counter_get` when the Orbital session is
/// authenticated, then binds `CounterSetForm` to a `ServerAction<SetCounter>`.
/// Success and failure feed `CounterStatusMessages`. Anonymous sessions in
/// this component path get a local "Not authenticated" error; the outer
/// `RequireAuthenticated` gate usually prevents that render.
#[component]
pub fn CounterAdminPage() -> impl IntoView {
    let auth_state = use_auth_state();

    let counter_res = Resource::new(
        move || auth_state.get(),
        |session| async move {
            match session {
                AuthSession::Authenticated(_) => counter_get().await,
                AuthSession::Anonymous(_) => {
                    Err(ServerFnError::ServerError("Not authenticated".into()))
                }
            }
        },
    );

    let set_action = ServerAction::<SetCounter>::new();

    let current_value = RwSignal::new(None::<usize>);
    let admin_error = RwSignal::new(None::<String>);
    let admin_success = RwSignal::new(None::<usize>);

    let _sync_counter = Effect::new(move |_| {
        if let Some(Ok(resp)) = counter_res.get() {
            current_value.set(Some(resp.value));
        }
    });

    let _sync_set = Effect::new(move |_| {
        if let Some(result) = set_action.value().get() {
            match result {
                Ok(resp) => {
                    admin_error.set(None);
                    admin_success.set(Some(resp.value));
                    current_value.set(Some(resp.value));
                }
                Err(err) => {
                    admin_success.set(None);
                    admin_error.set(Some(err.to_string()));
                }
            }
        }
    });

    view! {
        <ContentContainer max_width="480px" data_testid="counter-admin-container">
            <Flex vertical=true gap=SpacingSize::Size160.flex_gap()>
                <span data-testid="counter-admin-logged-in-marker" style="display:none" aria-hidden="true" />
                <Suspense fallback=move || view! {
                    <SkeletonItem size=Signal::from(SkeletonItemSize::S32) width="200px".to_string() />
                }>
                    {move || match counter_res.get() {
                        Some(Err(err)) => view! {
                            <MessageBar intent=MessageBarIntent::Error>
                                "Failed to load counter: " {err.to_string()}
                            </MessageBar>
                        }.into_any(),
                        _ => view! {
                            <Title3>"Counter Administration"</Title3>
                            <Body1>
                                "Current value: "
                                {move || current_value.get().map_or_else(|| "\u{2014}".to_string(), |value| value.to_string())}
                            </Body1>
                            <CounterSetForm set_action=set_action />
                            <CounterStatusMessages
                                admin_success=admin_success
                                admin_error=admin_error
                            />
                        }.into_any(),
                    }}
                </Suspense>
            </Flex>
        </ContentContainer>
    }
}
