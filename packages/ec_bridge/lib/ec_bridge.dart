/// Dart bindings for the Rust core of Ecosystem Connector.
///
/// Call `await initRustCore()` once at startup, then `EcCore.start(...)`.
/// The files under `lib/src/rust` are generated; edit `crates/ec-ffi/src/api` instead.
library;

import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';

import 'src/rust/frb_generated.dart';

export 'src/rust/api/core.dart';
export 'src/rust/frb_generated.dart' show RustLib;

/// Loads the Rust library and sets up the bindings.
///
/// The library's file name differs per platform. On Android and Windows cargokit ships
/// it as `ec_ffi` (the crate's name), which the bindings find on their own. On macOS it
/// is linked into this plugin's framework, `ec_bridge.framework`.
Future<void> initRustCore() async {
  if (Platform.isMacOS) {
    await RustLib.init(
      externalLibrary: ExternalLibrary.open('ec_bridge.framework/ec_bridge'),
    );
  } else {
    await RustLib.init();
  }
}
