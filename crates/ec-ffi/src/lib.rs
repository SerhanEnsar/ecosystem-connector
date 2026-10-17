//! Flutter bindings for the Ecosystem Connector runtime.
//!
//! Everything in [`api`] is exposed to Dart by flutter_rust_bridge; the Dart side lives
//! in `packages/ec_bridge`. Regenerate the bindings after changing the API:
//! `cd packages/ec_bridge && flutter_rust_bridge_codegen generate`.

pub mod api;

#[allow(
    unsafe_code,
    missing_docs,
    unexpected_cfgs,
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used
)]
mod frb_generated;
