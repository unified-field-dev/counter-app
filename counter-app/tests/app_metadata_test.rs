#![allow(missing_docs)]
use counter_app::AppMetadata;

#[test]
fn app_metadata_matches_uf_app_happy_path() {
    assert_eq!(AppMetadata::name(), "Counter");
    assert_eq!(AppMetadata::id(), "counter");
    assert_eq!(
        AppMetadata::description(),
        "A simple counter application demonstrating Valence ORM integration"
    );
    assert_eq!(AppMetadata::icon(), "📊");
    assert_eq!(AppMetadata::version(), "0.1.0");
}

#[test]
fn app_metadata_constants_match_accessors_happy_path() {
    assert_eq!(AppMetadata::NAME, "Counter");
    assert_eq!(AppMetadata::ID, "counter");
    assert_eq!(
        AppMetadata::DESCRIPTION,
        "A simple counter application demonstrating Valence ORM integration"
    );
    assert_eq!(AppMetadata::ICON, "📊");
    assert_eq!(AppMetadata::VERSION, "0.1.0");
}
