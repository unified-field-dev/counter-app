//! Photon user extractor for WS auth on counter topics.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use higgs_identity::SessionSnapshot;
use photon_axum::PhotonUserExtractor;

/// Reads [`SessionSnapshot`] inserted by [`crate::gate_demos::inject_e2e_session_snapshot`].
#[derive(Clone, Debug, Default)]
pub struct E2ePhotonAuth {
    user_key: Option<String>,
}

impl PhotonUserExtractor for E2ePhotonAuth {
    fn user_key(&self) -> Option<String> {
        self.user_key.clone()
    }
}

impl<S> FromRequestParts<S> for E2ePhotonAuth
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user_key = parts
            .extensions
            .get::<SessionSnapshot>()
            .map(|snap| snap.user_id.clone());
        Ok(Self { user_key })
    }
}
