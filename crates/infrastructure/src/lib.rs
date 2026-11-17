//! Infrastructure layer: the adapters behind the application's ports.
//!
//! - [`UreqHttpClient`] sends requests (ureq, the system's certificates).
//! - [`JsonCollectionRepository`] and [`JsonCollectionArchive`] keep the
//!   collection files; they read ours, sendingUnicorns 0.2's and Postman's.
//! - [`JsonSettingsRepository`] keeps `settings.json`.

mod files;
mod formats;
mod http;

pub use files::{
    JsonCollectionArchive, JsonCollectionRepository, JsonGlobalsRepository, JsonHistoryRepository,
    JsonSettingsRepository, data_dir, legacy_collections_dir, migrate_legacy,
};
pub use http::UreqHttpClient;
