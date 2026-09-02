//! Static synthetic bot definitions for the leaderboard demo.
//!
//! Consumed by Chronon seed / bump / reset scripts (`ensure_bot_users`,
//! `bot_score_bumper`, `daily_highscores_reset`). Emails are the stable key for
//! "is this a bot?" checks via `is_bot_email` and `bot_reset_score`.

/// Tier classification for bot users.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotTier {
    /// Top-tier bots actively compete on the leaderboard and are bumped by the
    /// `bot_score_bumper` script when real users overtake them.
    Top,
    /// Bottom-tier bots sit at the base of the leaderboard as background
    /// filler. They are never bumped.
    Bottom,
}

/// Definition of a single bot user.
#[derive(Debug, Clone)]
pub struct BotDef {
    /// Email used as the bot's login identifier.
    pub email: &'static str,
    /// Human-readable display name shown on the leaderboard.
    pub display_name: &'static str,
    /// Counter value assigned on each daily reset.
    pub reset_score: i64,
    /// Whether this bot competes (Top) or stays as filler (Bottom).
    pub tier: BotTier,
}

/// The full 40-bot roster.
///
/// **Bottom 30** sit at scores 1--30 and are never bumped.
/// **Top 10** sit at scores 35--80 (step 5) and are actively maintained
/// above real users by the `bot_score_bumper` script.
///
/// With page size 10, 40 bots + real users guarantees 4+ pages,
/// providing a compelling infinite-scroll pagination demo.
pub const BOT_ROSTER: &[BotDef] = &[
    // ── Bottom 30 (filler, never bumped) ────────────────────────────
    BotDef {
        email: "bottom30@example.com",
        display_name: "Rookie 30",
        reset_score: 1,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom29@example.com",
        display_name: "Rookie 29",
        reset_score: 2,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom28@example.com",
        display_name: "Rookie 28",
        reset_score: 3,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom27@example.com",
        display_name: "Rookie 27",
        reset_score: 4,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom26@example.com",
        display_name: "Rookie 26",
        reset_score: 5,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom25@example.com",
        display_name: "Rookie 25",
        reset_score: 6,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom24@example.com",
        display_name: "Rookie 24",
        reset_score: 7,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom23@example.com",
        display_name: "Rookie 23",
        reset_score: 8,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom22@example.com",
        display_name: "Rookie 22",
        reset_score: 9,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom21@example.com",
        display_name: "Rookie 21",
        reset_score: 10,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom20@example.com",
        display_name: "Rookie 20",
        reset_score: 11,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom19@example.com",
        display_name: "Rookie 19",
        reset_score: 12,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom18@example.com",
        display_name: "Rookie 18",
        reset_score: 13,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom17@example.com",
        display_name: "Rookie 17",
        reset_score: 14,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom16@example.com",
        display_name: "Rookie 16",
        reset_score: 15,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom15@example.com",
        display_name: "Rookie 15",
        reset_score: 16,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom14@example.com",
        display_name: "Rookie 14",
        reset_score: 17,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom13@example.com",
        display_name: "Rookie 13",
        reset_score: 18,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom12@example.com",
        display_name: "Rookie 12",
        reset_score: 19,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom11@example.com",
        display_name: "Rookie 11",
        reset_score: 20,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom10@example.com",
        display_name: "Rookie 10",
        reset_score: 21,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom9@example.com",
        display_name: "Rookie 9",
        reset_score: 22,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom8@example.com",
        display_name: "Rookie 8",
        reset_score: 23,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom7@example.com",
        display_name: "Rookie 7",
        reset_score: 24,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom6@example.com",
        display_name: "Rookie 6",
        reset_score: 25,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom5@example.com",
        display_name: "Rookie 5",
        reset_score: 26,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom4@example.com",
        display_name: "Rookie 4",
        reset_score: 27,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom3@example.com",
        display_name: "Rookie 3",
        reset_score: 28,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom2@example.com",
        display_name: "Rookie 2",
        reset_score: 29,
        tier: BotTier::Bottom,
    },
    BotDef {
        email: "bottom1@example.com",
        display_name: "Rookie 1",
        reset_score: 30,
        tier: BotTier::Bottom,
    },
    // ── Top 10 (actively compete) ───────────────────────────────────
    BotDef {
        email: "top10@example.com",
        display_name: "Contender 10",
        reset_score: 35,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top9@example.com",
        display_name: "Contender 9",
        reset_score: 40,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top8@example.com",
        display_name: "Contender 8",
        reset_score: 45,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top7@example.com",
        display_name: "Contender 7",
        reset_score: 50,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top6@example.com",
        display_name: "Contender 6",
        reset_score: 55,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top5@example.com",
        display_name: "Contender 5",
        reset_score: 60,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top4@example.com",
        display_name: "Contender 4",
        reset_score: 65,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top3@example.com",
        display_name: "Contender 3",
        reset_score: 70,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top2@example.com",
        display_name: "Contender 2",
        reset_score: 75,
        tier: BotTier::Top,
    },
    BotDef {
        email: "top1@example.com",
        display_name: "Contender 1",
        reset_score: 80,
        tier: BotTier::Top,
    },
];

/// Convenience: return only the top-tier bots, ordered by `reset_score`
/// ascending (lowest rank first).
pub fn top_tier_bots() -> impl Iterator<Item = &'static BotDef> {
    BOT_ROSTER.iter().filter(|b| b.tier == BotTier::Top)
}

/// Check whether an email belongs to any bot in the roster.
pub fn is_bot_email(email: &str) -> bool {
    BOT_ROSTER.iter().any(|b| b.email == email)
}

/// Look up a bot's reset score by email. Returns `None` for non-bot emails.
pub fn bot_reset_score(email: &str) -> Option<i64> {
    BOT_ROSTER
        .iter()
        .find(|b| b.email == email)
        .map(|b| b.reset_score)
}
