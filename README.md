# !!! UNOFFICIAL


# Diesel Async Migrations


Handles Postgres migrations via async diesel


## Example usage

```rust

pub const MIGRATIONS: diesel_async_migrations::EmbeddedMigrations = diesel_async_migrations::embed_migrations!();


async fn run_migrations(url: impl AsRef<str>) -> anyhow::Result<()> {
    let mut conn = AsyncConnection::establish(url.as_ref()).await?;
    MIGRATIONS.run_pending_migrations(&mut conn).await?;
    Ok(())
}


```


## Build.rs

In order for Cargo to correctly pick up changes to migration directory. Add a build.rs:

```rust

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}


```
