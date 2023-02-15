use std::{
    collections::HashMap,
    sync::atomic::{AtomicU8, Ordering},
};

use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
pub use macros::embed_migrations;
use scoped_futures::ScopedFutureExt;
use tracing::info;

diesel::table! {
    __diesel_schema_migrations (version) {
        version -> VarChar,
        run_on -> Timestamp,
    }
}

type Result<T> = std::result::Result<T, diesel::result::Error>;

pub const CREATE_MIGRATIONS_TABLE: &str = include_str!("setup_migration_table.sql");

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedMigration {
    pub up: &'static str,
    pub down: Option<&'static str>,
    pub name: &'static str,
}

impl EmbeddedMigration {
    pub fn version(&self) -> String {
        self.name
            .split('_')
            .next()
            .map(|s| s.replace('-', ""))
            .expect("invalid migration name")
    }
}

#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct EmbeddedMigrations {
    pub migrations: &'static [EmbeddedMigration],
    pub setup_attempted: AtomicU8,
}

impl EmbeddedMigrations {
    async fn setup_db<C>(&self, conn: &mut C) -> Result<()>
    where
        C: AsyncConnection<Backend = diesel::pg::Pg>,
    {
        if self.setup_attempted.fetch_add(1, Ordering::SeqCst) != 0 {
            return Ok(());
        }

        conn.batch_execute(CREATE_MIGRATIONS_TABLE).await?;

        Ok(())
    }

    pub async fn run_pending_migs<C>(&self, conn: &mut C) -> Result<()>
    where
        C: AsyncConnection<Backend = diesel::pg::Pg> + 'static + Send,
    {
        self.setup_db(conn).await?;

        let pending_migs = self.pending_migrations(conn).await?;

        if pending_migs.is_empty() {
            info!("no pending migrations");
        } else {
            info!("applying {} pending migrations", pending_migs.len());
        }

        for mig in pending_migs {
            info!("applying migration {}", mig.name);
            run_migration(conn, &mig).await?;
        }

        Ok(())
    }

    pub async fn pending_migrations<C>(&self, conn: &mut C) -> Result<Vec<EmbeddedMigration>>
    where
        C: AsyncConnection<Backend = diesel::pg::Pg>,
    {
        self.setup_db(conn).await?;
        let applied_versions = get_applied_migrations(conn).await?;

        let mut migrations = self
            .migrations
            .iter()
            .map(|mig| (mig.version(), *mig))
            .collect::<HashMap<_, _>>();

        for applied_version in applied_versions {
            migrations.remove(&applied_version.version);
        }

        let mut migrations = migrations.into_values().collect::<Vec<_>>();

        migrations.sort_unstable_by_key(|mig| mig.version());

        Ok(migrations)
    }
}

#[derive(Queryable)]
struct Version {
    version: String,
}

async fn run_migration<'a, C>(conn: &mut C, migration: &'a EmbeddedMigration) -> Result<Version>
where
    C: AsyncConnection<Backend = diesel::pg::Pg> + 'static + Send,
{
    let qry = migration.up.to_string();
    let version = migration.version();
    let res = conn
        .transaction::<_, diesel::result::Error, _>(|conn| {
            async move {
                conn.batch_execute(&qry).await?;

                let version = diesel::insert_into(__diesel_schema_migrations::table)
                    .values(__diesel_schema_migrations::version.eq(version))
                    .returning(__diesel_schema_migrations::version)
                    .get_result::<String>(conn)
                    .await?;

                Ok(Version { version })
            }
            .scope_boxed()
        })
        .await?;

    Ok(res)
}

async fn get_applied_migrations<C>(conn: &mut C) -> Result<Vec<Version>>
where
    C: AsyncConnection<Backend = diesel::pg::Pg>,
{
    let res = __diesel_schema_migrations::table
        .select(__diesel_schema_migrations::version)
        .order(__diesel_schema_migrations::version.desc())
        .get_results::<String>(conn)
        .await?
        .into_iter()
        .map(|version| Version { version })
        .collect::<Vec<_>>();

    Ok(res)
}
