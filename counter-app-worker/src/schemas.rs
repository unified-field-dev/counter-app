//! Force-link trait inventory only.
//!
//! Entity schemas (`counter`, `user_counter`) are registered by
//! `generated_models.rs` (build.rs / valence-codegen) with trait fields already
//! merged. Do **not** also `include!` the `valence_schema!` sources here, or
//! inventory submits a second [`valence::SchemaMetadataInit`] and panics under
//! Valence duplicate registration rules.

mod user_linked_counter_trait {
    include!("../schemas/user_linked_counter_valence_trait.rs");
    #[doc(hidden)]
    pub const __INVENTORY_LINK: () = ();
}

/// Keep trait `inventory` submissions linked and retain generated model modules
/// (which submit the complete entity [`valence::SchemaMetadataInit`] entries).
#[inline(never)]
pub fn ensure_inventory_linked() {
    let _ = (
        user_linked_counter_trait::__INVENTORY_LINK,
        std::any::type_name::<crate::generated::Counter>(),
        std::any::type_name::<crate::generated::UserCounter>(),
    );
}
