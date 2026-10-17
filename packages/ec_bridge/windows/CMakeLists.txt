# The Flutter tooling requires that developers have a version of Visual Studio
# installed that includes CMake 3.14 or later. You should not increase this
# version, as doing so will cause the plugin to fail to compile for some
# customers of the plugin.
cmake_minimum_required(VERSION 3.14)

# Project-level configuration.
set(PROJECT_NAME "ec_bridge")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
# Flutter reaches this plugin through a symlink (.plugin_symlinks), and Windows resolves
# ".." in paths textually, so "../../../crates/ec-ffi" would point into Flutter's
# ephemeral folder. Find the crate from cargokit's resolved real path instead, and give
# cargokit a path relative to this folder that resolves correctly when read as text.
get_filename_component(EC_FFI_DIR "${cargokit_cmake_root}/../../../crates/ec-ffi" ABSOLUTE)
file(RELATIVE_PATH EC_FFI_RELATIVE "${CMAKE_CURRENT_SOURCE_DIR}" "${EC_FFI_DIR}")
apply_cargokit(${PROJECT_NAME} "${EC_FFI_RELATIVE}" ec_ffi "")

# List of absolute paths to libraries that should be bundled with the plugin.
# This list could contain prebuilt libraries, or libraries created by an
# external build triggered from this build file.
set(ec_bridge_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  PARENT_SCOPE
)
