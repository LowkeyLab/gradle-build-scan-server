use std::borrow::Cow;

use sqlx::migrate::{Migration, MigrationType, Migrator};

/// Bazel-compatible migrator: `sqlx::migrate!()` relies on CARGO_MANIFEST_DIR
/// which doesn't resolve correctly in Bazel's sandbox. We construct the Migrator
/// manually using `include_str!` (which Bazel handles via `compile_data`).
pub static MIGRATOR: Migrator = Migrator {
    migrations: Cow::Borrowed(&[Migration {
        version: 1,
        description: Cow::Borrowed("initial"),
        migration_type: MigrationType::Simple,
        sql: Cow::Borrowed(include_str!("sql/001_initial.sql")),
        checksum: Cow::Borrowed(&[]),
        no_tx: false,
    }]),
    ignore_missing: false,
    locking: true,
    no_tx: false,
};
