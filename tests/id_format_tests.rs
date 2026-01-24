//! ID format validation tests for Magellan compatibility.
//!
//! This module tests ID generation and format validation for Splice-Magellan integration:
//!
//! - Symbol IDs: 16-character lowercase hexadecimal strings
//! - Execution IDs: {timestamp_hex}-{pid_hex} format
//!
//! These tests ensure that IDs are compatible with Magellan v0.5.3 conventions.

use regex::Regex;
use splice::symbol_id::{generate_symbol_id, generate_execution_id, SymbolId};

///////////////////////////////////////////////////////////////////////////////
// Symbol ID Format Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_symbol_id_format() {
    let id = generate_symbol_id("test_function", "src/test.rs", 100);
    let id_str = id.as_str();

    // Exactly 16 characters
    assert_eq!(
        id_str.len(),
        16,
        "Symbol ID should be exactly 16 characters"
    );

    // All lowercase hex via regex ^[0-9a-f]{16}$
    let hex_regex = Regex::new(r"^[0-9a-f]{16}$").unwrap();
    assert!(
        hex_regex.is_match(id_str),
        "Symbol ID should match regex ^[0-9a-f]{{16}}$, got: {}",
        id_str
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
fn test_symbol_id_unique_different_inputs() {
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
fn test_symbol_id_components() {
    let base_id = generate_symbol_id("base", "file.rs", 0);

    // ID changes when name changes
    let name_id = generate_symbol_id("changed", "file.rs", 0);
    assert_ne!(base_id, name_id, "ID should change when name changes");

    // ID changes when file_path changes
    let path_id = generate_symbol_id("base", "other.rs", 0);
    assert_ne!(base_id, path_id, "ID should change when file_path changes");

    // ID changes when byte_start changes
    let offset_id = generate_symbol_id("base", "file.rs", 100);
    assert_ne!(base_id, offset_id, "ID should change when byte_start changes");
}

#[test]
fn test_symbol_id_unicode() {
    // Test with Unicode symbol names (verify UTF-8 handling)
    let unicode_names = vec![
        "café",
        "函数",
        "функция",
        "関数",
        "الأمر",
        "test_emoji_🦀",
    ];

    for name in unicode_names {
        let id = generate_symbol_id(name, "src/test.rs", 0);
        let id_str = id.as_str();

        // Should still produce valid 16-char hex
        assert_eq!(
            id_str.len(),
            16,
            "Unicode name '{}' should produce 16-char ID",
            name
        );

        let hex_regex = Regex::new(r"^[0-9a-f]{16}$").unwrap();
        assert!(
            hex_regex.is_match(id_str),
            "Unicode name '{}' should produce valid hex ID",
            name
        );
    }
}

#[test]
fn test_symbol_id_edge_cases() {
    // Empty string
    let id1 = generate_symbol_id("", "src/test.rs", 0);
    assert_eq!(id1.as_str().len(), 16, "Empty name should produce valid ID");

    // Very long name
    let long_name = "a".repeat(10000);
    let id2 = generate_symbol_id(&long_name, "src/test.rs", 0);
    assert_eq!(
        id2.as_str().len(),
        16,
        "Very long name should produce valid ID"
    );

    // Special characters
    let special_names = vec!["test!@#$%", "test\nnewline", "test\ttab", "path/with/slashes"];
    for name in special_names {
        let id = generate_symbol_id(name, "src/test.rs", 0);
        assert_eq!(
            id.as_str().len(),
            16,
            "Special chars in name should produce valid ID"
        );
    }

    // Empty path
    let id3 = generate_symbol_id("func", "", 0);
    assert_eq!(id3.as_str().len(), 16, "Empty path should produce valid ID");

    // Very long path
    let long_path = "/".repeat(1000);
    let id4 = generate_symbol_id("func", &long_path, 0);
    assert_eq!(
        id4.as_str().len(),
        16,
        "Very long path should produce valid ID"
    );

    // Large byte offset
    let id5 = generate_symbol_id("func", "src/test.rs", usize::MAX);
    assert_eq!(
        id5.as_str().len(),
        16,
        "Large byte offset should produce valid ID"
    );
}

///////////////////////////////////////////////////////////////////////////////
// Execution ID Format Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_execution_id_format() {
    let exec_id = generate_execution_id();

    // Verify format: {timestamp_hex}-{pid_hex} via regex ^[0-9a-f]{8}-[0-9a-f]{4}$
    let exec_regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}$").unwrap();
    assert!(
        exec_regex.is_match(&exec_id),
        "Execution ID should match regex ^[0-9a-f]{{8}}-[0-9a-f]{{4}}$, got: {}",
        exec_id
    );

    // Total length: 8 + 1 + 4 = 13
    assert_eq!(
        exec_id.len(),
        13,
        "Execution ID should be 13 characters (8-1-4)"
    );
}

#[test]
fn test_execution_id_timestamp_valid() {
    let exec_id = generate_execution_id();
    let parts: Vec<&str> = exec_id.split('-').collect();

    assert_eq!(parts.len(), 2, "Execution ID should have 2 parts separated by -");

    let timestamp_hex = parts[0];

    // Parse timestamp from hex
    let timestamp = u32::from_str_radix(timestamp_hex, 16)
        .expect("Timestamp should be valid hex");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    // Timestamp should be within reasonable range (last 60 seconds, or within 60 seconds in future due to clock skew)
    let time_diff = if now > timestamp { now - timestamp } else { timestamp - now };
    assert!(
        time_diff < 60,
        "Timestamp should be within 60 seconds of current time. Got: {}, now: {}, diff: {}",
        timestamp, now, time_diff
    );
}

#[test]
fn test_execution_id_pid_matches() {
    let exec_id = generate_execution_id();
    let parts: Vec<&str> = exec_id.split('-').collect();

    let pid_hex = parts[1];
    let pid_from_id = u16::from_str_radix(pid_hex, 16)
        .expect("PID should be valid hex");

    let actual_pid = std::process::id() as u16;

    assert_eq!(
        pid_from_id, actual_pid,
        "PID from execution ID should match current process ID"
    );
}

#[test]
fn test_execution_id_uniqueness() {
    // Generate IDs and verify they all have valid format
    let mut ids = Vec::new();

    for _ in 0..5 {
        let id = generate_execution_id();
        ids.push(id);
    }

    // All should have valid format
    let exec_regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}$").unwrap();
    for id in &ids {
        assert!(exec_regex.is_match(id), "ID should have valid format: {}", id);
    }

    // All IDs should have the same PID
    let pids: std::collections::HashSet<_> = ids
        .iter()
        .map(|id| id.split('-').nth(1).unwrap())
        .collect();
    assert_eq!(pids.len(), 1, "All IDs should have the same PID");

    // All IDs should have the same timestamp (generated within same second)
    let timestamps: std::collections::HashSet<_> = ids
        .iter()
        .map(|id| id.split('-').next().unwrap())
        .collect();
    assert_eq!(
        timestamps.len(),
        1,
        "All IDs generated in quick succession should have same timestamp"
    );
}

#[test]
fn test_execution_id_lowercase() {
    let exec_id = generate_execution_id();

    // Verify all hex characters are lowercase
    for c in exec_id.chars() {
        if c.is_ascii_alphabetic() {
            assert!(
                c.is_ascii_lowercase(),
                "Execution ID should contain only lowercase letters, found: {}",
                c
            );
        }
    }
}

#[test]
fn test_symbol_id_type_validation() {
    // Valid ID creation
    let valid_id = SymbolId::new("a1b2c3d4e5f67890");
    assert!(valid_id.is_ok(), "Valid 16-char hex ID should be accepted");

    // Test as_str() access
    let id = valid_id.unwrap();
    assert_eq!(id.as_str(), "a1b2c3d4e5f67890");

    // Invalid: too short
    let err = SymbolId::new("abc123").unwrap_err();
    assert!(matches!(err, splice::symbol_id::SymbolIdError::InvalidLength { length: 6 }));

    // Invalid: too long
    let err = SymbolId::new("a1b2c3d4e5f678901234").unwrap_err();
    assert!(matches!(err, splice::symbol_id::SymbolIdError::InvalidLength { .. }));

    // Invalid: non-hex
    let err = SymbolId::new("abcdefghijklmnop").unwrap_err();
    assert!(matches!(err, splice::symbol_id::SymbolIdError::InvalidHex { .. }));

    // Invalid: uppercase
    let err = SymbolId::new("A1B2C3D4E5F67890").unwrap_err();
    assert!(matches!(err, splice::symbol_id::SymbolIdError::InvalidCase));
}

#[test]
fn test_symbol_id_from_generated() {
    // Generate an ID and verify it's a valid SymbolId
    let generated = generate_symbol_id("test", "file.rs", 0);
    let generated_str = generated.as_str();

    // The generated ID should be valid
    let validated = SymbolId::new(generated_str);
    assert!(validated.is_ok(), "Generated ID should be a valid SymbolId");

    let validated_id = validated.unwrap();
    assert_eq!(validated_id.as_str(), generated_str);
}
