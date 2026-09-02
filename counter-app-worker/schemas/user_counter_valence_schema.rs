// Valence DSL: per-user score (`UserCounter` / table `user_counter`).
//
// Authored with `valence_schema!`. Implements trait [`UserLinkedCounter`] (see
// sibling trait file). Create/update require `OWNER_BY_USER_FIELD` or
// `SYSTEM_ONLY` (blocks create-path IDOR); read is `PUBLIC_READ` for the
// anonymous high-scores page. `side_effects: [LeaderboardNotifier]` wires the
// Valence → Boson bridge after score changes.

use valence::prelude::*;
use valence::privacy_policies::common::{PUBLIC_READ, SYSTEM_ONLY};
use valence::privacy_policies::owner::OWNER_BY_USER_FIELD;

valence_schema! {
    UserCounter {
        table: "user_counter",
        version: "0.1.0",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "Per-user counter tracking for high scores and user-specific counts",

        traits: [UserLinkedCounter],
        
        privacy: {
            gdpr_compliant: false,
        },
        
        policies: {
            read: {
                always_allow: [],
                // Public leaderboard: anonymous high-scores page must list rows.
                // Writes stay owner/system; display names still use viewer User/Profile policies.
                allow: [PUBLIC_READ],
                block: [],
                always_block: [],
            },
            create: {
                always_allow: [],
                // Owner (or System) only — prevents authenticated create-path IDOR
                // where actor A upserts a UserCounter for user B.
                allow: [OWNER_BY_USER_FIELD, SYSTEM_ONLY],
                block: [],
                always_block: [],
            },
            update: {
                always_allow: [],
                allow: [OWNER_BY_USER_FIELD, SYSTEM_ONLY],
                block: [],
                always_block: [],
            },
            delete: {
                always_allow: [],
                allow: [SYSTEM_ONLY],
                block: [],
                always_block: [],
            },
        },
        
        fields: [
            id: {
                r#type: FieldType::String,
                primary_key: true,
                required: true,
            },
            user: {
                r#type: FieldType::Record("user"),
                required: true,
            },
            value: {
                r#type: FieldType::Integer,
                required: true,
                default: 0,
                validations: [Validator::NonNegative],
                policies: {
                    read: { allow: [PUBLIC_READ] },
                },
            }
        ],

        connections: [
            user: {
                table: "user",
                cardinality: HasOne,
                required: true,
                on_delete: Cascade,
                model: "lepton_identity::generated::User",
            },
        ],

        side_effects: [LeaderboardNotifier]
    }
}
