// Valence trait schema: shared `value` + `user` connection for linked counters.
//
// Declared with `valence_trait_schema!` and listed under `traits: [UserLinkedCounter]`
// on the UserCounter model. Trait-detail E2E exercises connections and "Used By"
// against this definition.

use valence::prelude::*;

// Trait exercised by Valence trait-detail E2E (connections + Used By on `user_counter`).
valence_trait_schema! {
    UserLinkedCounter {
        fields: [
            value: {
                r#type: FieldType::Integer,
                required: true,
            },
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
    }
}
