// Platform detection and feature flags
//
// Windows support is opt-in and explicit. This avoids silent behavior changes,
// bug reports from unsupported paths, and keeps trust with existing users.

#[cfg(feature = "windows")]
pub const IS_WINDOWS: bool = true;

#[cfg(not(feature = "windows"))]
pub const IS_WINDOWS: bool = false;

#[cfg(feature = "unix")]
pub const IS_UNIX: bool = true;

#[cfg(not(feature = "unix"))]
pub const IS_UNIX: bool = false;

/// Warn users about Windows limitations on first run
pub fn check_platform_support() {
    if IS_WINDOWS {
        eprintln!("=== Windows Support Notice ===");
        eprintln!("splice on Windows is fully supported for refactoring.");
        eprintln!();
        eprintln!("splice applies patches and edits code directly.");
        eprintln!("All features work identically across platforms.");
        eprintln!("==================================");
    }
}
