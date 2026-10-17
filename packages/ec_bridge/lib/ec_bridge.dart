/// Dart bindings for the Rust core of Ecosystem Connector.
///
/// Call `await RustLib.init()` once at startup, then `EcCore.start(...)`.
/// The files under `lib/src/rust` are generated; edit `crates/ec-ffi/src/api` instead.
library;

export 'src/rust/api/core.dart';
export 'src/rust/frb_generated.dart' show RustLib;
