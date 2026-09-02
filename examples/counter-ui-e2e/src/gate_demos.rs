//! Harness auth: tower-sessions e2e keys → AuthContext (no lepton Backend).

use leptos::prelude::*;
use uf_product::models::auth::{AnonymousUser, AuthSession, AuthenticatedUser};
use uf_product::{provide_auth_context, provide_auth_dialog_controller};

#[cfg(feature = "ssr")]
const E2E_AUTH_KEY: &str = "e2e_auth_kind";

/// Higgs / Valence session user ids (`table:id`).
#[cfg(feature = "ssr")]
pub const E2E_OWNER_USER: &str = "user:owner";
#[cfg(feature = "ssr")]
pub const E2E_UNVERIFIED_USER: &str = "user:unverified";
#[cfg(feature = "ssr")]
pub const E2E_MEMBER_USER: &str = "user:alice";

/// Session kinds for Playwright seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum E2eAuthKind {
    /// Signed out.
    Anonymous,
    /// Verified owner with CounterAdmin grant.
    Owner,
    /// Verified member without CounterAdmin (permission sad path).
    Member,
    /// Authenticated but email unverified.
    Unverified,
}

impl E2eAuthKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::Owner => "owner",
            Self::Member => "member",
            Self::Unverified => "unverified",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "owner" | "admin" | "authenticated_verified" => Self::Owner,
            "member" | "alice" => Self::Member,
            "unverified" | "authenticated_unverified" => Self::Unverified,
            _ => Self::Anonymous,
        }
    }

    #[cfg(feature = "ssr")]
    pub const fn session_user_id(self) -> Option<&'static str> {
        match self {
            Self::Anonymous => None,
            Self::Owner => Some(E2E_OWNER_USER),
            Self::Member => Some(E2E_MEMBER_USER),
            Self::Unverified => Some(E2E_UNVERIFIED_USER),
        }
    }

    pub fn to_session(self) -> AuthSession {
        match self {
            Self::Anonymous => AuthSession::Anonymous(AnonymousUser { reason: None }),
            Self::Owner => AuthSession::Authenticated(AuthenticatedUser {
                user_id: "owner".into(),
                email: Some("owner@example.com".into()),
                display_name: Some("Owner".into()),
                avatar_url: None,
                roles: vec!["user".into()],
                email_verified: true,
            }),
            Self::Member => AuthSession::Authenticated(AuthenticatedUser {
                user_id: "alice".into(),
                email: Some("alice@example.com".into()),
                display_name: Some("Alice".into()),
                avatar_url: None,
                roles: vec!["user".into()],
                email_verified: true,
            }),
            Self::Unverified => AuthSession::Authenticated(AuthenticatedUser {
                user_id: "unverified".into(),
                email: Some("unverified@example.com".into()),
                display_name: Some("Unverified".into()),
                avatar_url: None,
                roles: vec!["user".into()],
                email_verified: false,
            }),
        }
    }
}

#[server(E2eGetAuthKind)]
pub async fn e2e_get_auth_kind() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use leptos_axum::extract;
        use tower_sessions::Session;

        let session: Session = extract().await?;
        let kind = session
            .get::<String>(E2E_AUTH_KEY)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .unwrap_or_else(|| E2eAuthKind::Anonymous.as_str().to_string());
        return Ok(kind);
    }
    #[cfg(not(feature = "ssr"))]
    {
        Ok(E2eAuthKind::Anonymous.as_str().to_string())
    }
}

/// Resolve e2e session → AuthContext under Suspense.
#[component]
pub fn E2eAuthProvider(children: ChildrenFn) -> impl IntoView {
    let auth_kind = Resource::new(
        || (),
        |_| async move {
            e2e_get_auth_kind()
                .await
                .unwrap_or_else(|_| E2eAuthKind::Anonymous.as_str().to_string())
        },
    );

    view! {
        <Suspense fallback=|| {
            view! { <div data-testid="e2e-auth-loading" style="display:none" /> }
        }>
            {move || {
                let kind = auth_kind
                    .get()
                    .map(|raw| E2eAuthKind::parse(&raw))
                    .unwrap_or(E2eAuthKind::Anonymous);
                let _auth = provide_auth_context(kind.to_session());
                let _auth_dialog = provide_auth_dialog_controller();
                view! {
                    <div
                        data-testid="e2e-auth-bootstrap"
                        data-auth=kind.as_str()
                        style="display:none"
                    />
                    {children()}
                }
            }}
        </Suspense>
    }
}

#[cfg(feature = "ssr")]
pub async fn write_e2e_auth_kind(
    session: &tower_sessions::Session,
    kind: E2eAuthKind,
) -> anyhow::Result<()> {
    session
        .insert(E2E_AUTH_KEY, kind.as_str().to_string())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Mirror e2e tower-session into higgs_identity::SessionSnapshot.
#[cfg(feature = "ssr")]
pub async fn inject_e2e_session_snapshot(
    session: tower_sessions::Session,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use higgs_identity::SessionSnapshot;

    let kind = session
        .get::<String>(E2E_AUTH_KEY)
        .await
        .ok()
        .flatten()
        .map(|raw| E2eAuthKind::parse(&raw))
        .unwrap_or(E2eAuthKind::Anonymous);
    if let Some(user_id) = kind.session_user_id() {
        req.extensions_mut()
            .insert(SessionSnapshot::new(user_id, b"e2e-auth-hash"));
    }
    next.run(req).await
}
