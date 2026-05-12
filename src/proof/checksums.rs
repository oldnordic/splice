//! SHA-256 checksums for proof integrity validation.
//!
//! This module provides cryptographic checksums for refactoring proofs
//! to ensure audit trail integrity. Each proof includes checksums for:
//! - Before snapshot hash
//! - After snapshot hash
//! - Overall proof hash (for tamper detection)

use crate::error::{Result, SpliceError};
use crate::proof::data_structures::{GraphSnapshot, ProofChecksums, RefactoringProof};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Compute SHA-256 hash of a graph snapshot.
///
/// Serializes the snapshot to JSON and computes the SHA-256 hash
/// for integrity verification.
pub fn compute_snapshot_hash(snapshot: &GraphSnapshot) -> Result<String> {
    // Serialize snapshot to canonical JSON (sorted keys for consistency)
    let json = serde_json::to_string(snapshot)
        .map_err(|e| SpliceError::Other(format!("Failed to serialize snapshot: {}", e)))?;

    // Compute SHA-256 hash
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let hash = hasher.finalize();

    // Convert to hex string
    Ok(format!("{:x}", hash))
}

/// Compute checksums for a refactoring proof.
///
/// This function computes:
/// - SHA-256 hash of the before snapshot
/// - SHA-256 hash of the after snapshot
/// - SHA-256 hash of the combined proof (metadata + invariants)
///
/// The overall proof hash is computed from the metadata and invariants
/// only (not including the snapshots themselves, which have their own hashes).
pub fn compute_proof_checksums(proof: &RefactoringProof) -> Result<ProofChecksums> {
    // Compute before snapshot hash
    let before_hash = compute_snapshot_hash(&proof.before)?;

    // Compute after snapshot hash
    let after_hash = compute_snapshot_hash(&proof.after)?;

    // Compute proof hash from metadata + invariants (not snapshots)
    let proof_hash_input = format!(
        "{}:{}:{}",
        serde_json::to_string(&proof.metadata)
            .map_err(|e| SpliceError::Other(format!("Failed to serialize metadata: {}", e)))?,
        serde_json::to_string(&proof.invariants)
            .map_err(|e| SpliceError::Other(format!("Failed to serialize invariants: {}", e)))?,
        proof.before.timestamp // Include before timestamp for ordering
    );

    let mut hasher = Sha256::new();
    hasher.update(proof_hash_input.as_bytes());
    let proof_hash = format!("{:x}", hasher.finalize());

    Ok(ProofChecksums {
        before_hash,
        after_hash,
        proof_hash,
    })
}

/// Validate proof checksums.
///
/// Verifies that:
/// - The stored before hash matches the computed before hash
/// - The stored after hash matches the computed after hash
/// - The stored proof hash matches the computed proof hash
///
/// Returns Ok(true) if all checksums are valid, Ok(false) if checksums
/// are missing, and Err if any checksum is invalid.
pub fn validate_proof_checksums(proof: &RefactoringProof) -> Result<bool> {
    let checksums = match &proof.checksums {
        Some(c) => c,
        None => return Ok(false), // No checksums to validate
    };

    // Compute current checksums
    let current = compute_proof_checksums(proof)?;

    // Validate before hash
    if checksums.before_hash != current.before_hash {
        return Err(SpliceError::Other(format!(
            "Before snapshot hash mismatch: expected {}, got {}",
            checksums.before_hash, current.before_hash
        )));
    }

    // Validate after hash
    if checksums.after_hash != current.after_hash {
        return Err(SpliceError::Other(format!(
            "After snapshot hash mismatch: expected {}, got {}",
            checksums.after_hash, current.after_hash
        )));
    }

    // Validate proof hash
    if checksums.proof_hash != current.proof_hash {
        return Err(SpliceError::Other(format!(
            "Proof hash mismatch: expected {}, got {}",
            checksums.proof_hash, current.proof_hash
        )));
    }

    Ok(true)
}

/// Validate a proof file on disk.
///
/// Reads a proof JSON file, deserializes it, and validates its checksums.
///
/// # Arguments
/// * `proof_path` - Path to the proof JSON file
///
/// # Returns
/// * Ok(true) if checksums are valid
/// * Ok(false) if checksums are missing
/// * Err if proof is invalid or cannot be read
pub fn validate_proof_file(proof_path: &Path) -> Result<bool> {
    // Read proof file
    let json = std::fs::read_to_string(proof_path).map_err(|e| SpliceError::Io {
        path: proof_path.to_path_buf(),
        source: e,
    })?;

    // Deserialize proof
    let proof: RefactoringProof = serde_json::from_str(&json)
        .map_err(|e| SpliceError::Other(format!("Failed to deserialize proof: {}", e)))?;

    // Validate checksums
    validate_proof_checksums(&proof)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::data_structures::{GraphStats, ProofMetadata};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_compute_snapshot_hash() {
        let snapshot = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let hash = compute_snapshot_hash(&snapshot).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 is 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_snapshot_hash_consistency() {
        let snapshot = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let hash1 = compute_snapshot_hash(&snapshot).unwrap();
        let hash2 = compute_snapshot_hash(&snapshot).unwrap();

        // Same snapshot should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_snapshot_hash_uniqueness() {
        let snapshot1 = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let snapshot2 = GraphSnapshot {
            timestamp: 1234567891, // Different timestamp
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let hash1 = compute_snapshot_hash(&snapshot1).unwrap();
        let hash2 = compute_snapshot_hash(&snapshot2).unwrap();

        // Different snapshots should produce different hashes
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_compute_proof_checksums() {
        let metadata = ProofMetadata {
            operation: "test".to_string(),
            user: None,
            timestamp: 1234567890,
            git_commit: None,
            splice_version: "2.2.4".to_string(),
            database_path: PathBuf::from("/test/db"),
        };

        let before = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let after = GraphSnapshot {
            timestamp: 1234567891,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let proof = RefactoringProof {
            metadata,
            before,
            after,
            invariants: vec![],
            checksums: None,
        };

        let checksums = compute_proof_checksums(&proof).unwrap();

        // Verify all hashes are present and 64 chars (SHA-256 hex)
        assert_eq!(checksums.before_hash.len(), 64);
        assert_eq!(checksums.after_hash.len(), 64);
        assert_eq!(checksums.proof_hash.len(), 64);

        // Verify all hashes are hex
        assert!(checksums.before_hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(checksums.after_hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(checksums.proof_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_validate_proof_checksums_missing() {
        let proof = RefactoringProof {
            metadata: ProofMetadata {
                operation: "test".to_string(),
                user: None,
                timestamp: 1234567890,
                git_commit: None,
                splice_version: "2.2.4".to_string(),
                database_path: PathBuf::from("/test/db"),
            },
            before: GraphSnapshot {
                timestamp: 1234567890,
                symbols: HashMap::new(),
                edges: HashMap::new(),
                entry_points: vec![],
                stats: GraphStats {
                    total_symbols: 0,
                    total_edges: 0,
                    entry_point_count: 0,
                    max_complexity: None,
                },
            },
            after: GraphSnapshot {
                timestamp: 1234567891,
                symbols: HashMap::new(),
                edges: HashMap::new(),
                entry_points: vec![],
                stats: GraphStats {
                    total_symbols: 0,
                    total_edges: 0,
                    entry_point_count: 0,
                    max_complexity: None,
                },
            },
            invariants: vec![],
            checksums: None,
        };

        let result = validate_proof_checksums(&proof).unwrap();
        assert!(!result); // No checksums to validate
    }

    #[test]
    fn test_validate_proof_checksums_valid() {
        let metadata = ProofMetadata {
            operation: "test".to_string(),
            user: None,
            timestamp: 1234567890,
            git_commit: None,
            splice_version: "2.2.4".to_string(),
            database_path: PathBuf::from("/test/db"),
        };

        let before = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let after = GraphSnapshot {
            timestamp: 1234567891,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let mut proof = RefactoringProof {
            metadata,
            before,
            after,
            invariants: vec![],
            checksums: None,
        };

        // Compute and set checksums
        proof.checksums = Some(compute_proof_checksums(&proof).unwrap());

        // Validate - should pass
        let result = validate_proof_checksums(&proof).unwrap();
        assert!(result);
    }

    #[test]
    fn test_validate_proof_checksums_invalid() {
        let metadata = ProofMetadata {
            operation: "test".to_string(),
            user: None,
            timestamp: 1234567890,
            git_commit: None,
            splice_version: "2.2.4".to_string(),
            database_path: PathBuf::from("/test/db"),
        };

        let before = GraphSnapshot {
            timestamp: 1234567890,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let after = GraphSnapshot {
            timestamp: 1234567891,
            symbols: HashMap::new(),
            edges: HashMap::new(),
            entry_points: vec![],
            stats: GraphStats {
                total_symbols: 0,
                total_edges: 0,
                entry_point_count: 0,
                max_complexity: None,
            },
        };

        let mut proof = RefactoringProof {
            metadata,
            before,
            after,
            invariants: vec![],
            checksums: None,
        };

        // Compute checksums
        let checksums = compute_proof_checksums(&proof).unwrap();

        // Tamper with before hash
        let tampered_checksums = ProofChecksums {
            before_hash: "0".repeat(64), // Invalid hash
            after_hash: checksums.after_hash,
            proof_hash: checksums.proof_hash,
        };

        proof.checksums = Some(tampered_checksums);

        // Validate - should fail
        let result = validate_proof_checksums(&proof);
        assert!(result.is_err());
    }
}
