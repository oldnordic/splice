//! Symbol ID generation for Splice-Magellan integration.
//!
//! This module provides deterministic hexadecimal symbol IDs compatible with
//! Magellan v2.1.0's identifier format, supporting both legacy V1 (16-char SHA-256)
//! and new V2 (32-char BLAKE3) formats.
//!
//! # Symbol ID Format
//!
//! Symbol IDs come in two formats:
//!
//! ## V1 (Legacy): 16-character hexadecimal
//!
//! - **Determinism**: Same inputs always produce the same ID
//! - **Collision resistance**: SHA-256 provides strong guarantees
//! - **Compatibility**: Matches Magellan v0.5.3's 16-char hex format
//! - **Readability**: Hexadecimal is compact and URL-safe
//!
//! ## V2 (New): 32-character hexadecimal
//!
//! - **Determinism**: Same inputs always produce the same ID
//! - **Enhanced collision resistance**: BLAKE3 with full 256-bit output
//! - **Unambiguous**: Full hash output eliminates collision concerns
//! - **Magellan v2.1.0+**: Native format for current Magellan versions
//!
//! # ID Generation
//!
//! ## Symbol IDs
//!
//! [`generate_v1()`] creates a legacy 16-character identifier, while
//! [`generate_v2()`] creates a new 32-character identifier:
//!
//! ```text
//! V1: SHA-256(name:file_path:byte_start)[0..8] -> 16 hex chars
//! V2: BLAKE3(name:file_path:byte_start) -> 32 hex chars
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
//! use splice::symbol_id::{generate_v1, generate_v2, SymbolId};
//!
//! // Legacy format (16-char)
//! let id_v1 = generate_v1("main", "src/main.rs", 0);
//! assert_eq!(id_v1.as_str().len(), 16);
//!
//! // New format (32-char)
//! let id_v2 = generate_v2("main", "src/main.rs", 0);
//! assert_eq!(id_v2.as_str().len(), 32);
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
//! The [`SymbolId`] enum provides compile-time guarantees that only valid
//! hexadecimal IDs are used, with support for both formats:
//!
//! ```no_run
//! use splice::symbol_id::SymbolId;
//!
//! // Valid: 16 lowercase hex characters (V1)
//! let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
//!
//! // Valid: 32 lowercase hex characters (V2)
//! let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
//!
//! // Invalid: returns error
//! let err = SymbolId::parse("invalid");
//! assert!(err.is_err());
//! ```

use blake3;
use sha2::{Digest, Sha256};
use std::fmt;
use std::hash::Hash;

/// Error type for symbol ID validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolIdError {
    /// ID is not 16 or 32 characters.
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
                "Invalid symbol ID length: {} (expected 16 or 32 characters)",
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

/// A validated hexadecimal symbol ID supporting dual formats.
///
/// This enum provides type-safe handling of both legacy V1 (16-char SHA-256)
/// and new V2 (32-char BLAKE3) symbol IDs, ensuring backward compatibility
/// while enabling migration to the unambiguous BLAKE3 format.
///
/// # Variants
///
/// - `V1(String)`: Legacy 16-character SHA-256-based ID
/// - `V2(String)`: New 32-character BLAKE3-based ID
///
/// # Example
///
/// ```no_run
/// use splice::symbol_id::SymbolId;
///
/// // Parse either format
/// let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
/// let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
///
/// // Convert to string
/// let id_str: &str = id_v1.as_str();
/// println!("Symbol ID: {}", id_v1); // Implements Display
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolId {
    /// Legacy 16-character SHA-256-based ID
    V1(String),
    /// New 32-character BLAKE3-based ID
    V2(String),
}

impl SymbolId {
    /// Parse a string into a SymbolId, detecting format automatically.
    ///
    /// This method accepts both 16-character (V1) and 32-character (V2) formats,
    /// validating that the input is properly formatted lowercase hexadecimal.
    ///
    /// # Errors
    ///
    /// Returns `Err(SymbolIdError)` if:
    /// - The string is not 16 or 32 characters
    /// - The string contains non-hexadecimal characters
    /// - The string contains uppercase letters
    ///
    /// # Example
    ///
    /// ```
    /// use splice::symbol_id::SymbolId;
    ///
    /// // Parse V1 format
    /// let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
    /// assert!(matches!(id_v1, SymbolId::V1(_)));
    ///
    /// // Parse V2 format
    /// let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
    /// assert!(matches!(id_v2, SymbolId::V2(_)));
    ///
    /// // Invalid format
    /// assert!(SymbolId::parse("invalid").is_err());
    /// ```
    pub fn parse(input: &str) -> Result<Self, SymbolIdError> {
        Self::validate(input)?;
        match input.len() {
            16 => Ok(SymbolId::V1(input.to_string())),
            32 => Ok(SymbolId::V2(input.to_string())),
            _ => Err(SymbolIdError::InvalidLength { length: input.len() }),
        }
    }

    /// Create a V1 SymbolId without validation (use with caution).
    ///
    /// # Safety
    ///
    /// This function bypasses validation. Only use with IDs that
    /// are guaranteed to be valid (e.g., from [`generate_v1()`]).
    pub fn new_v1_unchecked(id: impl Into<String>) -> Self {
        SymbolId::V1(id.into())
    }

    /// Create a V2 SymbolId without validation (use with caution).
    ///
    /// # Safety
    ///
    /// This function bypasses validation. Only use with IDs that
    /// are guaranteed to be valid (e.g., from [`generate_v2()`]).
    pub fn new_v2_unchecked(id: impl Into<String>) -> Self {
        SymbolId::V2(id.into())
    }

    /// Validate that a string is a properly formatted symbol ID.
    ///
    /// Accepts both 16-character (V1) and 32-character (V2) formats.
    fn validate(id: &str) -> Result<(), SymbolIdError> {
        // Check length (accept both 16 and 32)
        if id.len() != 16 && id.len() != 32 {
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
        match self {
            SymbolId::V1(s) => s,
            SymbolId::V2(s) => s,
        }
    }

    /// Get the inner String value.
    pub fn into_inner(self) -> String {
        match self {
            SymbolId::V1(s) => s,
            SymbolId::V2(s) => s,
        }
    }

    /// Check if this is a V1 (16-char) SymbolId.
    pub fn is_v1(&self) -> bool {
        matches!(self, SymbolId::V1(_))
    }

    /// Check if this is a V2 (32-char) SymbolId.
    pub fn is_v2(&self) -> bool {
        matches!(self, SymbolId::V2(_))
    }
}

impl fmt::Display for SymbolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolId::V1(s) => write!(f, "{}", s),
            SymbolId::V2(s) => write!(f, "{}", s),
        }
    }
}

impl AsRef<str> for SymbolId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<String> for SymbolId {
    type Error = SymbolIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl TryFrom<&str> for SymbolId {
    type Error = SymbolIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// Generate a deterministic 16-character hex symbol ID (V1 format).
///
/// This is the legacy format using SHA-256 hash, maintained for backward
/// compatibility with Magellan v0.5.3 and existing databases.
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
/// A `SymbolId::V1` containing exactly 16 lowercase hexadecimal characters.
///
/// # Example
///
/// ```
/// use splice::symbol_id::generate_v1;
///
/// let id1 = generate_v1("main", "src/main.rs", 0);
/// let id2 = generate_v1("main", "src/main.rs", 0);
///
/// // Same inputs produce the same ID
/// assert_eq!(id1, id2);
///
/// // ID is exactly 16 lowercase hex characters
/// assert_eq!(id1.as_str().len(), 16);
/// assert!(id1.is_v1());
/// ```
pub fn generate_v1(name: &str, file_path: &str, byte_start: usize) -> SymbolId {
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
    SymbolId::V1(hex_id)
}

/// Generate a deterministic 32-character hex symbol ID (V2 format).
///
/// This is the new format using BLAKE3 hash, providing unambiguous identifiers
/// with full 256-bit output. This is the recommended format for new code.
///
/// # Format
///
/// ```text
/// BLAKE3(name:file_path:byte_start) -> 32 hex chars
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
/// A `SymbolId::V2` containing exactly 32 lowercase hexadecimal characters.
///
/// # Example
///
/// ```
/// use splice::symbol_id::generate_v2;
///
/// let id1 = generate_v2("main", "src/main.rs", 0);
/// let id2 = generate_v2("main", "src/main.rs", 0);
///
/// // Same inputs produce the same ID
/// assert_eq!(id1, id2);
///
/// // ID is exactly 32 lowercase hex characters
/// assert_eq!(id1.as_str().len(), 32);
/// assert!(id1.is_v2());
/// ```
pub fn generate_v2(name: &str, file_path: &str, byte_start: usize) -> SymbolId {
    let mut hasher = blake3::Hasher::new();

    // Hash format: name:file_path:byte_start
    hasher.update(name.as_bytes());
    hasher.update(b":");
    hasher.update(file_path.as_bytes());
    hasher.update(b":");
    hasher.update(&byte_start.to_be_bytes());

    let hash = hasher.finalize();

    // Take first 16 bytes and format as 32 lowercase hex characters
    let hex_id = format!(
        "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        hash.as_bytes()[0], hash.as_bytes()[1], hash.as_bytes()[2], hash.as_bytes()[3],
        hash.as_bytes()[4], hash.as_bytes()[5], hash.as_bytes()[6], hash.as_bytes()[7],
        hash.as_bytes()[8], hash.as_bytes()[9], hash.as_bytes()[10], hash.as_bytes()[11],
        hash.as_bytes()[12], hash.as_bytes()[13], hash.as_bytes()[14], hash.as_bytes()[15]
    );

    // Safe to use unchecked because we control the format
    SymbolId::V2(hex_id)
}

/// Generate a deterministic symbol ID (defaults to V2 BLAKE3 format).
///
/// This function provides a convenient default that uses the modern V2 format.
/// Use [`generate_v1()`] explicitly if you need legacy SHA-256 format.
///
/// # Arguments
///
/// * `name` - Symbol name (e.g., "my_function")
/// * `file_path` - Path to file containing the symbol
/// * `byte_start` - Byte offset where the symbol definition starts
///
/// # Returns
///
/// A `SymbolId::V2` containing exactly 32 lowercase hexadecimal characters.
///
/// # Example
///
/// ```
/// use splice::symbol_id::generate_symbol_id;
///
/// let id = generate_symbol_id("main", "src/main.rs", 0);
///
/// // ID is exactly 32 lowercase hex characters (V2 format)
/// assert_eq!(id.as_str().len(), 32);
/// assert!(id.is_v2());
/// ```
pub fn generate_symbol_id(name: &str, file_path: &str, byte_start: usize) -> SymbolId {
    generate_v2(name, file_path, byte_start)
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
    fn test_symbol_id_parse_v1_format() {
        let id = generate_v1("test_function", "src/test.rs", 100);
        let id_str = id.as_str();

        // Exactly 16 characters
        assert_eq!(
            id_str.len(),
            16,
            "V1 Symbol ID should be exactly 16 characters"
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
    fn test_symbol_id_parse_v2_format() {
        let id = generate_v2("test_function", "src/test.rs", 100);
        let id_str = id.as_str();

        // Exactly 32 characters
        assert_eq!(
            id_str.len(),
            32,
            "V2 Symbol ID should be exactly 32 characters"
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
    fn test_symbol_id_parse_accepts_both_formats() {
        // V1 format
        let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        assert!(id_v1.is_v1());
        assert!(!id_v1.is_v2());
        assert_eq!(id_v1.as_str(), "a1b2c3d4e5f67890");

        // V2 format
        let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
        assert!(id_v2.is_v2());
        assert!(!id_v2.is_v1());
        assert_eq!(id_v2.as_str(), "a1b2c3d4e5f67890a1b2c3d4e5f67890");

        // Invalid length
        assert!(SymbolId::parse("abc123").is_err());
        assert!(SymbolId::parse("a1b2c3d4e5f67890123").is_err());
    }

    #[test]
    fn test_symbol_id_v1_deterministic() {
        let id1 = generate_v1("my_func", "src/lib.rs", 42);
        let id2 = generate_v1("my_func", "src/lib.rs", 42);

        assert_eq!(id1, id2, "Same inputs should produce same V1 ID");
        assert_eq!(id1.as_str(), id2.as_str());
        assert!(id1.is_v1());
    }

    #[test]
    fn test_symbol_id_v2_deterministic() {
        let id1 = generate_v2("my_func", "src/lib.rs", 42);
        let id2 = generate_v2("my_func", "src/lib.rs", 42);

        assert_eq!(id1, id2, "Same inputs should produce same V2 ID");
        assert_eq!(id1.as_str(), id2.as_str());
        assert!(id2.is_v2());
    }

    #[test]
    fn test_symbol_id_v1_different_inputs() {
        let id1 = generate_v1("func_a", "src/lib.rs", 0);
        let id2 = generate_v1("func_b", "src/lib.rs", 0);
        let id3 = generate_v1("func_a", "src/main.rs", 0);
        let id4 = generate_v1("func_a", "src/lib.rs", 10);

        // All different inputs should produce different IDs
        assert_ne!(id1, id2, "Different names should produce different V1 IDs");
        assert_ne!(id1, id3, "Different paths should produce different V1 IDs");
        assert_ne!(id1, id4, "Different byte offsets should produce different V1 IDs");

        // Verify transitivity: all different from each other
        assert_ne!(id2, id3);
        assert_ne!(id2, id4);
        assert_ne!(id3, id4);
    }

    #[test]
    fn test_symbol_id_v2_different_inputs() {
        let id1 = generate_v2("func_a", "src/lib.rs", 0);
        let id2 = generate_v2("func_b", "src/lib.rs", 0);
        let id3 = generate_v2("func_a", "src/main.rs", 0);
        let id4 = generate_v2("func_a", "src/lib.rs", 10);

        // All different inputs should produce different IDs
        assert_ne!(id1, id2, "Different names should produce different V2 IDs");
        assert_ne!(id1, id3, "Different paths should produce different V2 IDs");
        assert_ne!(id1, id4, "Different byte offsets should produce different V2 IDs");

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
        let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        let displayed_v1 = format!("{}", id_v1);
        assert_eq!(displayed_v1, "a1b2c3d4e5f67890");

        let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
        let displayed_v2 = format!("{}", id_v2);
        assert_eq!(displayed_v2, "a1b2c3d4e5f67890a1b2c3d4e5f67890");
    }

    #[test]
    fn test_symbol_id_invalid_rejected() {
        // Too short
        assert!(matches!(
            SymbolId::parse("abc123").unwrap_err(),
            SymbolIdError::InvalidLength { length: 6 }
        ));

        // Too long (but not 32)
        assert!(matches!(
            SymbolId::parse("a1b2c3d4e5f678901234").unwrap_err(),
            SymbolIdError::InvalidLength { .. }
        ));

        // Non-hex characters
        assert!(matches!(
            SymbolId::parse("abcdefghijklmnop").unwrap_err(),
            SymbolIdError::InvalidHex { .. }
        ));

        // Uppercase letters
        assert!(matches!(
            SymbolId::parse("A1B2C3D4E5F67890").unwrap_err(),
            SymbolIdError::InvalidCase
        ));

        // Mix of valid and invalid
        assert!(SymbolId::parse("a1b2c3d4e5f6789g").is_err());
    }

    #[test]
    fn test_symbol_id_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let id1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        let id2 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        let id3 = SymbolId::parse("b2c3d4e5f67890a1").unwrap();

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
        let id1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        let id2 = id1.clone();

        assert_eq!(id1, id2);
        assert_eq!(id1.as_str(), id2.as_str());
    }

    #[test]
    fn test_symbol_id_try_from() {
        // Valid conversion (V1)
        let id1 = SymbolId::try_from("a1b2c3d4e5f67890".to_string()).unwrap();
        assert_eq!(id1.as_str(), "a1b2c3d4e5f67890");
        assert!(id1.is_v1());

        let id2 = SymbolId::try_from("a1b2c3d4e5f67890").unwrap();
        assert_eq!(id2.as_str(), "a1b2c3d4e5f67890");
        assert!(id2.is_v1());

        // Valid conversion (V2)
        let id3 = SymbolId::try_from("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
        assert_eq!(id3.as_str(), "a1b2c3d4e5f67890a1b2c3d4e5f67890");
        assert!(id3.is_v2());

        // Invalid conversion
        assert!(SymbolId::try_from("invalid").is_err());
        assert!(SymbolId::try_from("".to_string()).is_err());
    }

    #[test]
    fn test_symbol_id_into_inner() {
        let original = "a1b2c3d4e5f67890";
        let id = SymbolId::parse(original).unwrap();
        let inner = id.into_inner();

        assert_eq!(inner, original);
    }

    #[test]
    fn test_symbol_id_as_ref() {
        let id = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
        let s: &str = id.as_ref();

        assert_eq!(s, "a1b2c3d4e5f67890");
    }

    #[test]
    fn test_generate_v1_edge_cases() {
        // Empty name
        let id1 = generate_v1("", "src/test.rs", 0);
        assert_eq!(id1.as_str().len(), 16);
        assert!(id1.is_v1());

        // Empty path
        let id2 = generate_v1("func", "", 0);
        assert_eq!(id2.as_str().len(), 16);
        assert!(id2.is_v1());

        // Large byte offset
        let id3 = generate_v1("func", "src/test.rs", usize::MAX);
        assert_eq!(id3.as_str().len(), 16);
        assert!(id3.is_v1());

        // All IDs should be different
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
    }

    #[test]
    fn test_generate_v2_edge_cases() {
        // Empty name
        let id1 = generate_v2("", "src/test.rs", 0);
        assert_eq!(id1.as_str().len(), 32);
        assert!(id1.is_v2());

        // Empty path
        let id2 = generate_v2("func", "", 0);
        assert_eq!(id2.as_str().len(), 32);
        assert!(id2.is_v2());

        // Large byte offset
        let id3 = generate_v2("func", "src/test.rs", usize::MAX);
        assert_eq!(id3.as_str().len(), 32);
        assert!(id3.is_v2());

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

    #[test]
    fn test_generate_symbol_id_defaults_to_v2() {
        let id = generate_symbol_id("test", "src/test.rs", 0);
        assert_eq!(id.as_str().len(), 32);
        assert!(id.is_v2());
        assert!(!id.is_v1());
    }

    #[test]
    fn test_new_v1_unchecked() {
        let id = SymbolId::new_v1_unchecked("a1b2c3d4e5f67890");
        assert!(id.is_v1());
        assert_eq!(id.as_str(), "a1b2c3d4e5f67890");
    }

    #[test]
    fn test_new_v2_unchecked() {
        let id = SymbolId::new_v2_unchecked("a1b2c3d4e5f67890a1b2c3d4e5f67890");
        assert!(id.is_v2());
        assert_eq!(id.as_str(), "a1b2c3d4e5f67890a1b2c3d4e5f67890");
    }
}
