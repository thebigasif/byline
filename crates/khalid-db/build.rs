//! Rebuilds the crate when a migration is added, so `sqlx::migrate!` never
//! embeds an old list of migrations.

fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
