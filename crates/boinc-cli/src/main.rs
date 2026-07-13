//! `boinc` CLI. Subcommands (`convert`, `list-conversions`, `integrate`)
//! arrive in Phase 2 of `plan.md`.

fn main() {
    println!("boinc {}", boinc_core::version());
}
