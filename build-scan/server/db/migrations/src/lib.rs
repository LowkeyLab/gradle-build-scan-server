use std::borrow::Cow;

use sqlx::migrate::{Migration, MigrationType, Migrator};

/// Bazel-compatible migrator: `sqlx::migrate!()` relies on CARGO_MANIFEST_DIR
/// which doesn't resolve correctly in Bazel's sandbox. We construct the Migrator
/// manually using `include_str!` (which Bazel handles via `compile_data`).
pub static MIGRATOR: Migrator = Migrator {
    migrations: Cow::Borrowed(&[
        Migration {
            version: 1,
            description: Cow::Borrowed("initial"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/001_initial.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 2,
            description: Cow::Borrowed("composite pagination index"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/002_composite_pagination_index.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 3,
            description: Cow::Borrowed("tests table"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/003_tests_table.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 4,
            description: Cow::Borrowed("task caching reason"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/004_task_caching_reason.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 5,
            description: Cow::Borrowed("task cache detail"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/005_task_cache_detail.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 6,
            description: Cow::Borrowed("task cache operations"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!("sql/006_task_cache_operations.sql")),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 7,
            description: Cow::Borrowed("add duration_ms to cache operations"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!(
                "sql/007_add_duration_ms_to_cache_operations.sql"
            )),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
        Migration {
            version: 8,
            description: Cow::Borrowed("add test duration and failure details"),
            migration_type: MigrationType::Simple,
            sql: Cow::Borrowed(include_str!(
                "sql/008_test_duration_and_failure.sql"
            )),
            checksum: Cow::Borrowed(&[]),
            no_tx: false,
        },
    ]),
    ignore_missing: false,
    locking: true,
    no_tx: false,
};
