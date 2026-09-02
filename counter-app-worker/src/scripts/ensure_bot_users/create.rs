//! Create a single bot user (identity + counter) when the email is not yet present.
//!
//! Walks Lepton identity models under Valence (`User`, `Account`, email, membership,
//! profile), then upserts [`crate::generated::UserCounter`]. Called only from
//! [`super::ensure_bot_users_seed`] after the email uniqueness check.

use anyhow::Result;
use chrono::{DateTime, Utc};
use lepton_identity::generated::{Account, User};
use valence::{Model, RecordId, Valence};

use super::super::bot_roster::BotDef;

pub(super) async fn create_bot_user(valence: &Valence, bot: &BotDef) -> Result<()> {
    let now = Utc::now();
    let (user_created, user_thing) = create_user(valence, bot, now).await?;
    let (account_created, account_thing) =
        create_account(valence, bot, user_thing.clone(), now).await?;
    create_and_link_email(
        valence,
        bot,
        &user_created,
        &account_created,
        account_thing.clone(),
        now,
    )
    .await?;
    create_membership_and_profile(valence, bot, user_thing.clone(), account_thing, now).await?;
    upsert_bot_counter(valence, bot, user_thing).await
}

async fn create_user(
    valence: &Valence,
    bot: &BotDef,
    now: DateTime<Utc>,
) -> Result<(User, RecordId)> {
    use lepton_identity::auth::hash_password;
    use lepton_identity::generated::{UserStatus, UserUserType};

    let bot_secret = uuid::Uuid::new_v4().to_string();
    let password_hash = hash_password(&bot_secret)
        .map_err(|e| anyhow::anyhow!("Failed to hash bot password for {}: {e}", bot.email))?;

    let user = User::new(
        Some(UserUserType::Service),
        Some(password_hash),
        Some(UserStatus::Active),
        None,
        None,
        None,
        None,
        None,
        now,
        now,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create user for {}: {e}", bot.email))?;

    let user_created = User::create(user, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create user for {}: {e}", bot.email))?;
    let user_thing = user_created
        .id()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Created bot user {} missing ID", bot.email))?;
    Ok((user_created, user_thing))
}

async fn create_account(
    valence: &Valence,
    bot: &BotDef,
    user_thing: RecordId,
    now: DateTime<Utc>,
) -> Result<(Account, RecordId)> {
    use lepton_identity::generated::{AccountPlan, AccountStatus};

    let account = Account::new(
        bot.display_name.to_string(),
        user_thing,
        Some(AccountPlan::Free),
        Some(AccountStatus::Active),
        None,
        None,
        now,
        now,
    )
    .map_err(|e| anyhow::anyhow!("Failed to create account for {}: {e}", bot.email))?;

    let account_created = Account::create(account, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create account for {}: {e}", bot.email))?;
    let account_thing = account_created
        .id()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Created bot account {} missing ID", bot.email))?;
    Ok((account_created, account_thing))
}

async fn create_and_link_email(
    valence: &Valence,
    bot: &BotDef,
    user_created: &User,
    account_created: &Account,
    account_thing: RecordId,
    now: DateTime<Utc>,
) -> Result<()> {
    use lepton_identity::generated::AccountEmail;

    let email_row = AccountEmail::new(account_thing, bot.email.to_string(), Some(now), now, now)
        .map_err(|e| anyhow::anyhow!("Failed to build AccountEmail for {}: {e}", bot.email))?;
    let email_created = AccountEmail::create(email_row, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create AccountEmail for {}: {e}", bot.email))?;
    let email_thing = email_created
        .id()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Bot email {} missing id after create", bot.email))?;

    account_created
        .get_mutable(valence)
        .set_primary_email(email_thing.clone())
        .map_err(|e| anyhow::anyhow!("Failed to set account primary email for {}: {e}", bot.email))?
        .set_updated_at(now)
        .map_err(|e| anyhow::anyhow!("Failed to update account for {}: {e}", bot.email))?
        .commit()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to persist account primary email for {}: {e}",
                bot.email
            )
        })?;

    user_created
        .get_mutable(valence)
        .set_primary_email(email_thing)
        .map_err(|e| anyhow::anyhow!("Failed to set user primary email for {}: {e}", bot.email))?
        .set_updated_at(now)
        .map_err(|e| anyhow::anyhow!("Failed to update user for {}: {e}", bot.email))?
        .commit()
        .await
        .map_err(|e| {
            anyhow::anyhow!(
                "Failed to persist user primary email for {}: {e}",
                bot.email
            )
        })?;

    Ok(())
}

async fn create_membership_and_profile(
    valence: &Valence,
    bot: &BotDef,
    user_thing: RecordId,
    account_thing: RecordId,
    now: DateTime<Utc>,
) -> Result<()> {
    use lepton_identity::generated::{AccountMembership, AccountMembershipRole, UserProfile};

    let membership = AccountMembership::new(
        account_thing,
        user_thing.clone(),
        AccountMembershipRole::Owner,
        now,
        now,
    )
    .map_err(|e| anyhow::anyhow!("Failed to build membership for {}: {e}", bot.email))?;
    AccountMembership::create(membership, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create membership for {}: {e}", bot.email))?;

    let profile = UserProfile::new(
        user_thing,
        bot.display_name.to_string(),
        bot.display_name.to_string(),
        now,
        now,
        None,
    )
    .map_err(|e| anyhow::anyhow!("Failed to build UserProfile for {}: {e}", bot.email))?;

    UserProfile::create(profile, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create UserProfile for {}: {e}", bot.email))?;

    Ok(())
}

async fn upsert_bot_counter(valence: &Valence, bot: &BotDef, user_thing: RecordId) -> Result<()> {
    use crate::generated::UserCounter;

    let user_id_clean = valence::extract_id_from_record(&user_thing)
        .map_err(|e| anyhow::anyhow!("Bot user {} invalid ID after create: {e}", bot.email))?;

    let counter = UserCounter::new(user_thing, bot.reset_score)
        .map_err(|e| anyhow::anyhow!("Failed to create UserCounter for {}: {e}", bot.email))?;
    UserCounter::upsert(&user_id_clean, counter, valence)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to upsert UserCounter for {}: {e}", bot.email))?;

    Ok(())
}
