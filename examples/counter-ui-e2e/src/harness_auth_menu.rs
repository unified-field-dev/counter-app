//! Demo auth menu for RequireAuthenticated gate dialogs.

use leptos::prelude::*;
use uf_product::primitives::{Body1, Button, ButtonAppearance, Title3};
use uf_product::{
    provide_auth_dialog_controller, use_auth_dialog_controller, AuthDialogController,
    AuthDialogIntent,
};

/// App-bar stub plus harness dialog driven by AuthDialogController.
#[component]
pub fn HarnessAuthMenu() -> impl IntoView {
    let controller = use_auth_dialog_controller().unwrap_or_else(provide_auth_dialog_controller);
    view! {
        <span data-testid="demo-auth-menu">"Demo user"</span>
        <HarnessAuthDialog controller />
    }
}

#[component]
fn HarnessAuthDialog(controller: AuthDialogController) -> impl IntoView {
    let open = controller.open();
    let intent = controller.intent();
    let close = Callback::new(move |_| {
        controller.close();
    });

    view! {
        {move || {
            if !open.get() {
                return ().into_any();
            }
            view! {
                <div data-testid="auth-dialog-root" role="dialog" aria-modal="true">
                    <Title3>
                        {move || match intent.get() {
                            AuthDialogIntent::Signin => "Sign in required",
                            AuthDialogIntent::Signup => "Harness sign up",
                            AuthDialogIntent::Logout => "Harness log out",
                        }}
                    </Title3>
                    <Body1>
                        "Counter e2e harness. Seed auth via /api/test/seed-data."
                    </Body1>
                    <div data-testid="harness-auth-dialog-empty" />
                    <Button appearance=ButtonAppearance::Subtle on_click=close>
                        "Close"
                    </Button>
                </div>
            }
            .into_any()
        }}
    }
}
