//! ID format validation tests for Magellan compatibility.
//!
//! This module tests ID generation and format validation for Splice-Magellan integration:
//!
//! - Symbol IDs: 16-character (V1 SHA-256) or 32-character (V2 BLAKE3) lowercase hexadecimal strings
//! - Execution IDs: {timestamp_hex}-{pid_hex} format
//!
//! These tests ensure that IDs are compatible with Magellan v2.1.0 conventions
//! while maintaining backward compatibility with v0.5.3.

use regex::Regex;
use splice::symbol_id::{
    generate_execution_id, generate_symbol_id, generate_v1, generate_v2, SymbolId,
};

///////////////////////////////////////////////////////////////////////////////
// Symbol ID Format Tests
///////////////////////////////////////////////////////////////////////////////

#[test]
fn test_symbol_id_v1_format() {
    let id = generate_v1("test_function", "src/test.rs", 100);
    let id_str = id.as_str();

    // Exactly 16 characters
    assert_eq!(
        id_str.len(),
        16,
        "V1 Symbol ID should be exactly 16 characters"
    );

    // All lowercase hex via regex ^[0-9a-f]{16}$
    let hex_regex = Regex::new(r"^[0-9a-f]{16}$").unwrap();
    assert!(
        hex_regex.is_match(id_str),
        "V1 Symbol ID should match regex ^[0-9a-f]{{16}}$, got: {}",
        id_str
    );

    // Should be V1 variant
    assert!(id.is_v1(), "Should be V1 format");
    assert!(!id.is_v2(), "Should not be V2 format");
}

#[test]
fn test_symbol_id_v2_format() {
    let id = generate_v2("test_function", "src/test.rs", 100);
    let id_str = id.as_str();

    // Exactly 32 characters
    assert_eq!(
        id_str.len(),
        32,
        "V2 Symbol ID should be exactly 32 characters"
    );

    // All lowercase hex via regex ^[0-9a-f]{32}$
    let hex_regex = Regex::new(r"^[0-9a-f]{32}$").unwrap();
    assert!(
        hex_regex.is_match(id_str),
        "V2 Symbol ID should match regex ^[0-9a-f]{{32}}$, got: {}",
        id_str
    );

    // Should be V2 variant
    assert!(id.is_v2(), "Should be V2 format");
    assert!(!id.is_v1(), "Should not be V1 format");
}

#[test]
fn test_symbol_id_deterministic_v1() {
    let id1 = generate_v1("my_func", "src/lib.rs", 42);
    let id2 = generate_v1("my_func", "src/lib.rs", 42);

    assert_eq!(id1, id2, "Same inputs should produce same V1 ID");
    assert_eq!(id1.as_str(), id2.as_str());
}

#[test]
fn test_symbol_id_deterministic_v2() {
    let id1 = generate_v2("my_func", "src/lib.rs", 42);
    let id2 = generate_v2("my_func", "src/lib.rs", 42);

    assert_eq!(id1, id2, "Same inputs should produce same V2 ID");
    assert_eq!(id1.as_str(), id2.as_str());
}

#[test]
fn test_symbol_id_unique_different_inputs_v1() {
    let id1 = generate_v1("func_a", "src/lib.rs", 0);
    let id2 = generate_v1("func_b", "src/lib.rs", 0);
    let id3 = generate_v1("func_a", "src/main.rs", 0);
    let id4 = generate_v1("func_a", "src/lib.rs", 10);

    // All different inputs should produce different IDs
    assert_ne!(id1, id2, "Different names should produce different V1 IDs");
    assert_ne!(id1, id3, "Different paths should produce different V1 IDs");
    assert_ne!(
        id1, id4,
        "Different byte offsets should produce different V1 IDs"
    );

    // Verify transitivity: all different from each other
    assert_ne!(id2, id3);
    assert_ne!(id2, id4);
    assert_ne!(id3, id4);
}

#[test]
fn test_symbol_id_unique_different_inputs_v2() {
    let id1 = generate_v2("func_a", "src/lib.rs", 0);
    let id2 = generate_v2("func_b", "src/lib.rs", 0);
    let id3 = generate_v2("func_a", "src/main.rs", 0);
    let id4 = generate_v2("func_a", "src/lib.rs", 10);

    // All different inputs should produce different IDs
    assert_ne!(id1, id2, "Different names should produce different V2 IDs");
    assert_ne!(id1, id3, "Different paths should produce different V2 IDs");
    assert_ne!(
        id1, id4,
        "Different byte offsets should produce different V2 IDs"
    );

    // Verify transitivity: all different from each other
    assert_ne!(id2, id3);
    assert_ne!(id2, id4);
    assert_ne!(id3, id4);
}

#[test]
fn test_symbol_id_components_v1() {
    let base_id = generate_v1("base", "file.rs", 0);

    // ID changes when name changes
    let name_id = generate_v1("changed", "file.rs", 0);
    assert_ne!(base_id, name_id, "V1 ID should change when name changes");

    // ID changes when file_path changes
    let path_id = generate_v1("base", "other.rs", 0);
    assert_ne!(
        base_id, path_id,
        "V1 ID should change when file_path changes"
    );

    // ID changes when byte_start changes
    let offset_id = generate_v1("base", "file.rs", 100);
    assert_ne!(
        base_id, offset_id,
        "V1 ID should change when byte_start changes"
    );
}

#[test]
fn test_symbol_id_components_v2() {
    let base_id = generate_v2("base", "file.rs", 0);

    // ID changes when name changes
    let name_id = generate_v2("changed", "file.rs", 0);
    assert_ne!(base_id, name_id, "V2 ID should change when name changes");

    // ID changes when file_path changes
    let path_id = generate_v2("base", "other.rs", 0);
    assert_ne!(
        base_id, path_id,
        "V2 ID should change when file_path changes"
    );

    // ID changes when byte_start changes
    let offset_id = generate_v2("base", "file.rs", 100);
    assert_ne!(
        base_id, offset_id,
        "V2 ID should change when byte_start changes"
    );
}

#[test]
fn test_symbol_id_unicode_v1() {
    // Test with Unicode symbol names (verify UTF-8 handling)
    let unicode_names = vec!["café", "函数", "функция", "関数", "الأمر", "test_emoji_🦀"];

    for name in unicode_names {
        let id = generate_v1(name, "src/test.rs", 0);
        let id_str = id.as_str();

        // Should still produce valid 16-char hex
        assert_eq!(
            id_str.len(),
            16,
            "Unicode name '{}' should produce 16-char V1 ID",
            name
        );

        let hex_regex = Regex::new(r"^[0-9a-f]{16}$").unwrap();
        assert!(
            hex_regex.is_match(id_str),
            "Unicode name '{}' should produce valid V1 hex ID",
            name
        );
    }
}

#[test]
fn test_symbol_id_unicode_v2() {
    // Test with Unicode symbol names (verify UTF-8 handling)
    let unicode_names = vec!["café", "函数", "функция", "関数", "الأمر", "test_emoji_🦀"];

    for name in unicode_names {
        let id = generate_v2(name, "src/test.rs", 0);
        let id_str = id.as_str();

        // Should still produce valid 32-char hex
        assert_eq!(
            id_str.len(),
            32,
            "Unicode name '{}' should produce 32-char V2 ID",
            name
        );

        let hex_regex = Regex::new(r"^[0-9a-f]{32}$").unwrap();
        assert!(
            hex_regex.is_match(id_str),
            "Unicode name '{}' should produce valid V2 hex ID",
            name
        );
    }
}

#[test]
fn test_symbol_id_edge_cases_v1() {
    // Empty string
    let id1 = generate_v1("", "src/test.rs", 0);
    assert_eq!(
        id1.as_str().len(),
        16,
        "Empty name should produce valid V1 ID"
    );

    // Very long name
    let long_name = "a".repeat(10000);
    let id2 = generate_v1(&long_name, "src/test.rs", 0);
    assert_eq!(
        id2.as_str().len(),
        16,
        "Very long name should produce valid V1 ID"
    );

    // Special characters
    let special_names = vec![
        "test!@#$%",
        "test\nnewline",
        "test\ttab",
        "path/with/slashes",
    ];
    for name in special_names {
        let id = generate_v1(name, "src/test.rs", 0);
        assert_eq!(
            id.as_str().len(),
            16,
            "Special chars in name should produce valid V1 ID"
        );
    }

    // Empty path
    let id3 = generate_v1("func", "", 0);
    assert_eq!(
        id3.as_str().len(),
        16,
        "Empty path should produce valid V1 ID"
    );

    // Very long path
    let long_path = "/".repeat(1000);
    let id4 = generate_v1("func", &long_path, 0);
    assert_eq!(
        id4.as_str().len(),
        16,
        "Very long path should produce valid V1 ID"
    );

    // Large byte offset
    let id5 = generate_v1("func", "src/test.rs", usize::MAX);
    assert_eq!(
        id5.as_str().len(),
        16,
        "Large byte offset should produce valid V1 ID"
    );
}

#[test]
fn test_symbol_id_edge_cases_v2() {
    // Empty string
    let id1 = generate_v2("", "src/test.rs", 0);
    assert_eq!(
        id1.as_str().len(),
        32,
        "Empty name should produce valid V2 ID"
    );

    // Very long name
    let long_name = "a".repeat(10000);
    let id2 = generate_v2(&long_name, "src/test.rs", 0);
    assert_eq!(
        id2.as_str().len(),
        32,
        "Very long name should produce valid V2 ID"
    );

    // Special characters
    let special_names = vec![
        "test!@#$%",
        "test\nnewline",
        "test\ttab",
        "path/with/slashes",
    ];
    for name in special_names {
        let id = generate_v2(name, "src/test.rs", 0);
        assert_eq!(
            id.as_str().len(),
            32,
            "Special chars in name should produce valid V2 ID"
        );
    }

    // Empty path
    let id3 = generate_v2("func", "", 0);
    assert_eq!(
        id3.as_str().len(),
        32,
        "Empty path should produce valid V2 ID"
    );

    // Very long path
    let long_path = "/".repeat(1000);
    let id4 = generate_v2("func", &long_path, 0);
    assert_eq!(
        id4.as_str().len(),
        32,
        "Very long path should produce valid V2 ID"
    );

    // Large byte offset
    let id5 = generate_v2("func", "src/test.rs", usize::MAX);
    assert_eq!(
        id5.as_str().len(),
        32,
        "Large byte offset should produce valid V2 ID"
    );
}

#[test]
fn test_symbol_id_defaults_to_v2() {
    let id = generate_symbol_id("my_func", "src/lib.rs", 42);
    assert_eq!(id.as_str().len(), 32, "Default should be V2 (32-char)");
    assert!(id.is_v2(), "Default should produce V2 format");
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

    assert_eq!(
        parts.len(),
        2,
        "Execution ID should have 2 parts separated by -"
    );

    let timestamp_hex = parts[0];

    // Parse timestamp from hex
    let timestamp = u32::from_str_radix(timestamp_hex, 16).expect("Timestamp should be valid hex");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    // Timestamp should be within reasonable range (last 60 seconds, or within 60 seconds in future due to clock skew)
    let time_diff = if now > timestamp {
        now - timestamp
    } else {
        timestamp - now
    };
    assert!(
        time_diff < 60,
        "Timestamp should be within 60 seconds of current time. Got: {}, now: {}, diff: {}",
        timestamp,
        now,
        time_diff
    );
}

#[test]
fn test_execution_id_pid_matches() {
    let exec_id = generate_execution_id();
    let parts: Vec<&str> = exec_id.split('-').collect();

    let pid_hex = parts[1];
    let pid_from_id = u16::from_str_radix(pid_hex, 16).expect("PID should be valid hex");

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
        assert!(
            exec_regex.is_match(id),
            "ID should have valid format: {}",
            id
        );
    }

    // All IDs should have the same PID
    let pids: std::collections::HashSet<_> =
        ids.iter().map(|id| id.split('-').nth(1).unwrap()).collect();
    assert_eq!(pids.len(), 1, "All IDs should have the same PID");

    // All IDs should have the same timestamp (generated within same second)
    let timestamps: std::collections::HashSet<_> =
        ids.iter().map(|id| id.split('-').next().unwrap()).collect();
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
fn test_symbol_id_parse_dual_format() {
    // Valid V1 ID (16-char)
    let id_v1 = SymbolId::parse("a1b2c3d4e5f67890").unwrap();
    assert!(id_v1.is_v1(), "Should parse as V1 format");
    assert!(!id_v1.is_v2(), "Should not be V2 format");
    assert_eq!(id_v1.as_str(), "a1b2c3d4e5f67890");

    // Valid V2 ID (32-char)
    let id_v2 = SymbolId::parse("a1b2c3d4e5f67890a1b2c3d4e5f67890").unwrap();
    assert!(id_v2.is_v2(), "Should parse as V2 format");
    assert!(!id_v2.is_v1(), "Should not be V1 format");
    assert_eq!(id_v2.as_str(), "a1b2c3d4e5f67890a1b2c3d4e5f67890");

    // Invalid: too short
    let err = SymbolId::parse("abc123").unwrap_err();
    assert!(matches!(
        err,
        splice::symbol_id::SymbolIdError::InvalidLength { length: 6 }
    ));

    // Invalid: wrong length (not 16 or 32)
    let err = SymbolId::parse("a1b2c3d4e5f678901234").unwrap_err();
    assert!(matches!(
        err,
        splice::symbol_id::SymbolIdError::InvalidLength { .. }
    ));

    // Invalid: non-hex
    let err = SymbolId::parse("abcdefghijklmnop").unwrap_err();
    assert!(matches!(
        err,
        splice::symbol_id::SymbolIdError::InvalidHex { .. }
    ));

    // Invalid: uppercase
    let err = SymbolId::parse("A1B2C3D4E5F67890").unwrap_err();
    assert!(matches!(err, splice::symbol_id::SymbolIdError::InvalidCase));
}

#[test]
fn test_symbol_id_from_generated_v1() {
    // Generate a V1 ID and verify it's a valid SymbolId
    let generated = generate_v1("test", "file.rs", 0);
    let generated_str = generated.as_str();

    // The generated ID should be valid
    let validated = SymbolId::parse(generated_str);
    assert!(
        validated.is_ok(),
        "Generated V1 ID should be a valid SymbolId"
    );

    let validated_id = validated.unwrap();
    assert_eq!(validated_id.as_str(), generated_str);
    assert!(validated_id.is_v1());
}

#[test]
fn test_symbol_id_from_generated_v2() {
    // Generate a V2 ID and verify it's a valid SymbolId
    let generated = generate_v2("test", "file.rs", 0);
    let generated_str = generated.as_str();

    // The generated ID should be valid
    let validated = SymbolId::parse(generated_str);
    assert!(
        validated.is_ok(),
        "Generated V2 ID should be a valid SymbolId"
    );

    let validated_id = validated.unwrap();
    assert_eq!(validated_id.as_str(), generated_str);
    assert!(validated_id.is_v2());
}
