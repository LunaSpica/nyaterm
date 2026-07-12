//! Root GPUI shell boundary.
//!
//! `AppShell` is the public root-view name used by the application crate. It is
//! currently a compatibility alias while state is moved out of the migrated
//! `NyaTermApp` implementation in smaller slices.

pub type AppShell = crate::ui::NyaTermApp;
