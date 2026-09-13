//! Resolve login email from a User's `primary_email` FK (`AccountEmail`).
//!
//! Leaderboard scripts need the address string to classify bots vs humans via
//! [`super::bot_roster::is_bot_email`]. This helper is the Valence walk:
//! `user.primary_email()` → bare id → [`AccountEmail::get`] → `address()`.

use lepton_identity::generated::{AccountEmail, User};
use valence::{extract_id_from_record, Model, Valence};

/// Load the address on `user.primary_email`, if present.
///
/// Returns `None` when the FK is missing, the id cannot be parsed, or the
/// `AccountEmail` row is absent — callers typically skip that leaderboard entry.
pub async fn primary_email_address(user: &User, valence: &Valence) -> Option<String> {
    let email_id = user.primary_email()?;
    let bare = extract_id_from_record(email_id).ok()?;
    AccountEmail::get_used(&bare, valence, valence::use_!(r#"In **Counter app**, we **load Account Email** so the application can decide what to do next in this workflow. The result is used by **Counter app** logic—not necessarily displayed on a page unless that feature’s UI shows it."#))
        .await
        .ok()
        .flatten()
        .map(|row| row.address().clone())
}
