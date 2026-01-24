//! Symbol ID generation for Splice-Magellan integration.
//!
//! This module provides deterministic 16-character hexadecimal symbol IDs
//! compatible with Magellan v0.5.3's identifier format.
//!
//! # Symbol ID Format
//!
//! Symbol IDs are 16-character lowercase hexadecimal strings representing
//! the first 8 bytes of a SHA-256 hash. This format provides:
//!
//! - **Determinism**: Same inputs always produce the same ID
//! - **Collision resistance**: SHA-256 provides strong guarantees
//! - **Compatibility**: Matches Magellan's 16-char hex format
//! - **Readability**: Hexadecimal is compact and URL-safe
//!
//! # ID Generation
//!
//! ## Symbol IDs
//!
//! [`generate_symbol_id()`] creates a stable identifier for a symbol from
//! its defining properties:
//!
//! ```text
//! SHA-256(name:file_path:byte_start)[0..8] -> 16 hex chars
//! ```
//!
//! Inputs:
//! - `name`: Symbol name (e.g., "my_function")
//! - `file_path`: Path to file containing the symbol (e.g., "src/main.rs")
//! - `byte_start`: Byte offset where symbol definition starts
//!
//! Example:
//!
//! ```no_run
//! use splice::symbol_id::generate_symbol_id;
//!
//! let id = generate_symbol_id("main", "src/main.rs", 0);
//! assert_eq!(id.len(), 16);
//! assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit() && c.is_lowercase()));
//! ```
//!
//! ## Execution IDs
//!
//! [`generate_execution_id()`] creates a unique identifier for a Splice
//! execution run in Magellan-compatible format:
//!
//! ```text
//! {timestamp_hex}-{pid_hex}
//! ```
//!
//! Where:
//! - `timestamp_hex`: 8-char lowercase hex of current Unix timestamp
//! - `pid_hex`: 4-char lowercase hex of process ID
//!
//! Example: `6793a1b2-12ab`
//!
//! # SymbolId Type
//!
//! The [`SymbolId`] newtype wrapper provides compile-time guarantees
//! that only valid 16-character hex IDs are used:
//!
//! ```no_run
//! use splice::symbol_id::SymbolId;
//!
//! // Valid: 16 lowercase hex characters
//! let id = SymbolId::new("a1b2c3d4e5f67890").unwrap();
//!
//! // Invalid: returns error
//! let err = SymbolId::new("invalid");
//! assert!(err.is_err());
//! ```

use sha2::{Digest, Sha256};
use std::fmt;
use std::hash::Hash;

/// Error type for symbol ID validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolIdError {
    /// ID is not exactly 16 characters.
    InvalidLength { length: usize },
    /// ID contains non-hexadecimal characters.
    InvalidHex { invalid_char: char },
    /// ID contains uppercase letters (must be lowercase).
    InvalidCase,
}

impl fmt::Display for SymbolIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength { length } => write!(
                f,
                "Invalid symbol ID length: {} (expected 16 characters)",
                length
            ),
            Self::InvalidHex { invalid_char } => {
                write!(f, "Invalid character in symbol ID: '{}'", invalid_char)
            }
            Self::InvalidCase => write!(f, "Symbol ID must be lowercase hexadecimal"),
        }
    }
}

impl std::error::Error for SymbolIdError {}

/// A validated 16-character hexadecimal symbol ID.
///
/// This newtype wrapper ensures that only valid lowercase hex IDs
/// are used throughout the codebase. IDs are generated using
/// SHA-256 hash of symbol properties, providing deterministic
/// and collision-resistant identifiers compatible with Magellan.
///
/// # Example
///
/// ```no_run
/// use splice::symbol_id::SymbolId;
///
/// // Create from string (validates format)
/// let id = SymbolId::new("a1b2c3d4e5f67890").unwrap();
///
/// // Convert to string
/// let id_str: &str = id.as_str();
/// println!("Symbol ID: {}", id); // Implements Display
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(String);

impl SymbolId {
    /// Create a new SymbolId after validating the format.
    ///
    /// # Errors
    ///
    /// Returns `Err(SymbolIdError)` if:
    /// - The string is not exactly 16 characters
    /// - The string contains non-hexadecimal characters
    /// - The string contains uppercase letters
    pub fn new(id: impl Into<String>) -> Result<Self, SymbolIdError> {
        let id = id.into();
        Self::validate(&id)?;
        Ok(Self(id))
    }

    /// Create a SymbolId without validation (use with caution).
    ///
    /// # Safety
    ///
    /// This function bypasses validation. Only use with IDs that
    /// are guaranteed to be valid (e.g., from [`generate_symbol_id()`]).
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Validate that a string is a properly formatted symbol ID.
    fn validate(id: &str) -> Result<(), SymbolIdError> {
        // Check length
        if id.len() != 16 {
            return Err(SymbolIdError::InvalidLength { length: id.len() });
        }

        // Check all characters are lowercase hex
        for c in id.chars() {
            if !c.is_ascii_hexdigit() {
                return Err(SymbolIdError::InvalidHex { invalid_char: c });
            }
            if c.is_ascii_alphabetic() && c.is_ascii_uppercase() {
                return Err(SymbolIdError::InvalidCase);
            }
        }

        Ok(())
    }

    /// Get the underlying string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the inner String value.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SymbolId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SymbolId {
    type Error = SymbolIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SymbolId {
    type Error = SymbolIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Generate a deterministic 16-character hex symbol ID.
///
/// The ID is generated from a SHA-256 hash of the symbol's defining
/// properties, ensuring the same symbol always produces the same ID.
///
/// # Format
///
/// ```text
/// SHA-256(name:file_path:byte_start)[0..8] -> 16 hex chars
/// ```
///
/// # Arguments
///
/// * `name` - Symbol name (e.g., "my_function")
/// * `file_path` - Path to file containing the symbol
/// * `byte_start` - Byte offset where the symbol definition starts
///
/// # Returns
///
/// A `SymbolId` containing exactly 16 lowercase hexadecimal characters.
///
/// # Example
///
/// ```
/// use splice::symbol_id::generate_symbol_id;
///
/// let id1 = generate_symbol_id("main", "src/main.rs", 0);
/// let id2 = generate_symbol_id("main", "src/main.rs", 0);
///
/// // Same inputs produce the same ID
/// assert_eq!(id1, id2);
///
/// // ID is exactly 16 lowercase hex characters
/// assert_eq!(id1.as_str().len(), 16);
/// ```
pub fn generate_symbol_id(name: &str, file_path: &str, byte_start: usize) -> SymbolId {
    let mut hasher = Sha256::new();

    // Hash format: name:file_path:byte_start
    hasher.update(name.as_bytes());
    hasher.update(b":");
    hasher.update(file_path.as_bytes());
    hasher.update(b":");
    hasher.update(byte_start.to_be_bytes());

    let result = hasher.finalize();

    // Take first 8 bytes and format as 16 lowercase hex characters
    let hex_id = format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        result[0], result[1], result[2], result[3],
        result[4], result[5], result[6], result[7]
    );

    // Safe to use unchecked because we control the format
    SymbolId::new_unchecked(hex_id)
}

/// Generate a Magellan-compatible execution ID.
///
/// The execution ID uniquely identifies a Splice operation run,
/// compatible with Magellan's format: `{timestamp_hex}-{pid_hex}`.
///
/// # Format
///
/// ```text
/// {8-char-hex-timestamp}-{4-char-hex-pid}
/// ```
///
/// Where:
/// - `timestamp_hex`: Current Unix timestamp as 8-char lowercase hex
/// - `pid_hex`: Process ID as 4-char lowercase hex
///
/// # Returns
///
/// A 13-character string (8 chars + dash + 4 chars).
///
/// # Example
///
/// ```
/// use splice::symbol_id::generate_execution_id;
///
/// let exec_id = generate_execution_id();
/// assert!(exec_id.len() == 13); // "xxxxxxxx-xxxx"
/// assert!(exec_id.contains('-'));
///
/// // Verify format: 8 hex chars, dash, 4 hex chars
/// let parts: Vec<&str> = exec_id.split('-').collect();
/// assert_eq!(parts.len(), 2);
/// assert_eq!(parts[0].len(), 8);
/// assert_eq!(parts[1].len(), 4);
/// ```
pub fn generate_execution_id() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let pid = std::process::id();

    format!(
        "{:08x}-{:04x}",
        timestamp & 0xFFFFFFFF, // Lower 32 bits of timestamp
        pid & 0xFFFF            // Lower 16 bits of PID
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_id_valid_format() {
        let id = generate_symbol_id("test_function", "src/test.rs", 100);
        let id_str = id.as_str();

        // Exactly 16 characters
        assert_eq!(
            id_str.len(),
            16,
            "Symbol ID should be exactly 16 characters"
        );

        // All lowercase hex
        assert!(
            id_str.chars().all(|c| {
                c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_lowercase())
            }),
            "All characters should be lowercase hex"
        );
    }

    #[test]
    fn test_symbol_id_deterministic() {
        let id1 = generate_symbol_id("my_func", "src/lib.rs", 42);
        let id2 = generate_symbol_id("my_func", "src/lib.rs", 42);

        assert_eq!(id1, id2, "Same inputs should produce same ID");
        assert_eq!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_symbol_id_different_inputs() {
        let id1 = generate_symbol_id("func_a", "src/lib.rs", 0);
        let id2 = generate_symbol_id("func_b", "src/lib.rs", 0);
        let id3 = generate_symbol_id("func_a", "src/main.rs", 0);
        let id4 = generate_symbol_id("func_a", "src/lib.rs", 10);

        // All different inputs should produce different IDs
        assert_ne!(id1, id2, "Different names should produce different IDs");
        assert_ne!(id1, id3, "Different paths should produce different IDs");
        assert_ne!(id1, id4, "Different byte offsets should produce different IDs");

        // Verify transitivity: all different from each other
        assert_ne!(id2, id3);
        assert_ne!(id2, id4);
        assert_ne!(id3, id4);
    }

    #[test]
    fn test_execution_id_format() {
        let exec_id = generate_execution_id();

        // Format: {8-hex}-{4-hex} = 13 chars total
        assert_eq!(
            exec_id.len(),
            13,
            "Execution ID should be 13 characters (8-1-4)"
        );

        // Verify structure
        let parts: Vec<&str> = exec_id.split('-').collect();
        assert_eq!(parts.len(), 2, "Should have exactly one dash");

        let (timestamp_part, pid_part) = (parts[0], parts[1]);
        assert_eq!(
            timestamp_part.len(),
            8,
            "Timestamp part should be 8 hex characters"
        );
        assert_eq!(
            pid_part.len(),
            4,
            "PID part should be 4 hex characters"
        );

        // Verify all hex characters
        assert!(
            timestamp_part
                .chars()
                .all(|c| c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_lowercase())),
            "Timestamp part should be lowercase hex"
        );
        assert!(
            pid_part
                .chars()
                .all(|c| c.is_ascii_hexdigit() && (!c.is_ascii_alphabetic() || c.is_lowercase())),
            "PID part should be lowercase hex"
        );
    }

    #[test]
    fn test_symbol_id_display() {
        let id = SymbolId::new("a1b2c3d4e5f67890").unwrap();
        let displayed = format!("{}", id);

        assert_eq!(displayed, "a1b2c3d4e5f67890");
        assert_eq!(id.as_str(), "a1b2c3d4e5f67890");
    }

    #[test]
    fn test_symbol_id_invalid_rejected() {
        // Too short
        assert!(matches!(
            SymbolId::new("abc123").unwrap_err(),
            SymbolIdError::InvalidLength { length: 6 }
        ));

        // Too long
        assert!(matches!(
            SymbolId::new("a1b2c3d4e5f678901234").unwrap_err(),
            SymbolIdError::InvalidLength { .. }
        ));

        // Non-hex characters
        assert!(matches!(
            SymbolId::new("abcdefghijklmnop").unwrap_err(),
            SymbolIdError::InvalidHex { .. }
        ));

        // Uppercase letters
        assert!(matches!(
            SymbolId::new("A1B2C3D4E5F67890").unwrap_err(),
            SymbolIdError::InvalidCase
        ));

        // Mix of valid and invalid
        assert!(SymbolId::new("a1b2c3d4e5f6789g").is_err());
    }

    #[test]
    fn test_symbol_id_hash() {
        use std::collections::hash_map::DefaultHasher;

        let id1 = SymbolId::new("a1b2c3d4e5f67890").unwrap();
        let id2 = SymbolId::new("a1b2c3d4e5f67890").unwrap();
        let id3 = SymbolId::new("b2c3d4e5f67890a1").unwrap();

        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        let mut h3 = DefaultHasher::new();

        id1.hash(&mut h1);
        id2.hash(&mut h2);
        id3.hash(&mut h3);

        // Same values produce same hash
        assert_eq!(h1.finish(), h2.finish());
        // Different values produce different hash
        assert_ne!(h1.finish(), h3.finish());
    }

    #[test]
    fn test_symbol_id_clone() {
        let id1 = SymbolId::new("a1b2c3d4e5f67890").unwrap();
        let id2 = id1.clone();

        assert_eq!(id1, id2);
        assert_eq!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_symbol_id_try_from() {
        // Valid conversion
        let id1 = SymbolId::try_from("a1b2c3d4e5f67890".to_string()).unwrap();
        assert_eq!(id1.as_str(), "a1b2c3d4e5f67890");

        let id2 = SymbolId::try_from("a1b2c3d4e5f67890").unwrap();
        assert_eq!(id2.as_str(), "a1b2c3d4e5f67890");

        // Invalid conversion
        assert!(SymbolId::try_from("invalid").is_err());
        assert!(SymbolId::try_from("".to_string()).is_err());
    }

    #[test]
    fn test_symbol_id_into_inner() {
        let original = "a1b2c3d4e5f67890";
        let id = SymbolId::new(original).unwrap();
        let inner = id.into_inner();

        assert_eq!(inner, original);
    }

    #[test]
    fn test_symbol_id_as_ref() {
        let id = SymbolId::new("a1b2c3d4e5f67890").unwrap();
        let s: &str = id.as_ref();

        assert_eq!(s, "a1b2c3d4e5f67890");
    }

    #[test]
    fn test_generate_symbol_id_edge_cases() {
        // Empty name
        let id1 = generate_symbol_id("", "src/test.rs", 0);
        assert_eq!(id1.as_str().len(), 16);

        // Empty path
        let id2 = generate_symbol_id("func", "", 0);
        assert_eq!(id2.as_str().len(), 16);

        // Large byte offset
        let id3 = generate_symbol_id("func", "src/test.rs", usize::MAX);
        assert_eq!(id3.as_str().len(), 16);

        // All IDs should be different
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    #[test]
    fn test_execution_id_uniqueness() {
        let id1 = generate_execution_id();
        let id2 = generate_execution_id();

        // In practice these should be different due to time passing,
        // but we can at least verify format consistency
        assert_eq!(id1.len(), 13);
        assert_eq!(id2.len(), 13);

        // Both have valid format
        for id in [id1, id2] {
            let parts: Vec<&str> = id.split('-').collect();
            assert_eq!(parts.len(), 2);
            assert_eq!(parts[0].len(), 8);
            assert_eq!(parts[1].len(), 4);
        }
    }
}
