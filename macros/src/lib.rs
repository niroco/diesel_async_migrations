// Built-in Lints
// Clippy lints
#![allow(
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::type_complexity,
    clippy::needless_doctest_main
)]
#![warn(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::mut_mut,
    clippy::non_ascii_literal,
    clippy::similar_names,
    clippy::unicode_not_nfc,
    clippy::enum_glob_use,
    clippy::if_not_else,
    clippy::items_after_statements,
    clippy::used_underscore_binding,
    missing_debug_implementations,
    missing_copy_implementations
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]
extern crate proc_macro;

mod embed_migrations;
mod migrations;

use proc_macro::TokenStream;

/// This macro will read your migrations at compile time, and create a constant value containing
/// an embedded list of all your migrations as available at compile time.
/// This is useful if you would like to use Diesel's migration infrastructure, but want to ship a single executable
/// file (such as for embedded applications). It can also be used to apply migrations to an in
/// memory database (Diesel does this for its own test suite).
///
/// You can optionally pass the path to the migrations directory to this macro. When left
/// unspecified, Diesel will search for the migrations directory in the same way that
/// Diesel CLI does. If specified, the path should be relative to the directory where `Cargo.toml`
/// resides.
///
/// # Automatic rebuilds
///
/// Due to limitations in rusts proc-macro API there is currently no
/// way to signal that a specific proc macro should be rerun if some
/// external file changes/is added. This implies that `embed_migrations!`
/// cannot regenerate the list of embedded migrations if **only** the
/// migrations are changed. This limitation can be solved by adding a
/// custom `build.rs` file to your crate, such that the crate is rebuild
/// if the migration directory changes.
///
/// Add the following `build.rs` file to your project to fix the problem
///
/// ```
/// fn main() {
///    println!("cargo:rerun-if-changed=path/to/your/migration/dir/relative/to/your/Cargo.toml");
/// }
/// ```
#[proc_macro]
pub fn embed_migrations(input: TokenStream) -> TokenStream {
    embed_migrations::expand(input.to_string())
        .to_string()
        .parse()
        .expect("Failed create embedded migrations instance")
}

fn migrations_directories(
    path: &'_ std::path::Path,
) -> Result<impl Iterator<Item = Result<std::fs::DirEntry, std::io::Error>> + '_, std::io::Error> {
    Ok(path.read_dir()?.filter_map(|entry_res| {
        entry_res
            .and_then(|entry| {
                Ok(
                    if entry.metadata()?.is_file()
                        || entry.file_name().to_string_lossy().starts_with('.')
                    {
                        None
                    } else {
                        Some(entry)
                    },
                )
            })
            .transpose()
    }))
}

fn version_from_string(path: &str) -> Option<String> {
    path.split('_').next().map(|s| s.replace('-', ""))
}

fn search_for_migrations_directory(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let migration_path = path.join("migrations");
    if migration_path.is_dir() {
        Some(migration_path)
    } else {
        path.parent().and_then(search_for_migrations_directory)
    }
}
