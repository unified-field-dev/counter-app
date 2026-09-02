// Valence DSL: global singleton counter (`Counter` / table `counter`).
//
// Authored with `valence_schema!`. Key teaching bits: `database` points at
// [`crate::embedded_surreal::DEFAULT_STORAGE`]; policies allow `PUBLIC_READ` on
// update so anonymous demos can increment; delete stays `SYSTEM_ONLY`. The
// service layer always uses record id `"singleton"`.

use valence::prelude::*;
use valence::privacy_policies::common::{PUBLIC_READ, AUTHENTICATED, SYSTEM_ONLY};

valence_schema! {
    Counter {
        table: "counter",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        version: "0.1.0",
        description: "Simple counter for demonstration (PUBLIC_READ update is intentional: demo increments use user or system actors)",
        
        privacy: {
            gdpr_compliant: false,
        },
        
        policies: {
            read: {
                always_allow: [],
                allow: [PUBLIC_READ],
                block: [],
                always_block: [],
            },
            create: {
                always_allow: [],
                allow: [AUTHENTICATED],
                block: [],
                always_block: [],
            },
            update: {
                always_allow: [],
                allow: [PUBLIC_READ],
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
            value: {
                r#type: FieldType::Integer,
                required: true,
                default: 0,
                validations: [Validator::NonNegative],
                policies: {
                    read: { allow: [PUBLIC_READ] },
                },
            }
        ]
    }
}
