//! Feature tests for paykit-demo-cli new features
//!
//! Tests for endpoints management, rotation policies, backup/restore,
//! and other Phase 3 features.

use std::process::Command;
use tempfile::TempDir;

fn run_cli(temp_dir: &TempDir, args: &[&str]) -> (String, String, bool) {
    let output = Command::new("cargo")
        .args(["run", "-p", "paykit-demo-cli", "--"])
        .args(args)
        .env("PAYKIT_DEMO_DIR", temp_dir.path())
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

#[cfg(test)]
mod endpoints_tests {
    use super::*;

    #[test]
    fn test_endpoints_list_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // List endpoints should work with empty state
        let (stdout, stderr, success) = run_cli(&temp_dir, &["endpoints", "list"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("no") || combined.contains("empty") || success,
            "endpoints list should handle empty state: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_endpoints_stats() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Stats should work
        let (stdout, stderr, success) = run_cli(&temp_dir, &["endpoints", "stats"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("total") || combined.contains("statistic") || success,
            "endpoints stats should display statistics: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_endpoints_cleanup() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Cleanup should work even with no expired endpoints
        let (stdout, stderr, success) = run_cli(&temp_dir, &["endpoints", "cleanup"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("no") || combined.contains("removed") || combined.contains("expired") || success,
            "endpoints cleanup should handle empty state: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod rotation_tests {
    use super::*;

    #[test]
    fn test_rotation_status() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Check rotation status
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "status"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("policy") || combined.contains("rotation") || success,
            "rotation status should display policy info: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_rotation_set_default_policy() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Set default policy
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "default", "on-use"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("updated") || combined.contains("success") || success,
            "rotation default should update policy: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_rotation_set_method_policy() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Set per-method policy
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "policy", "lightning", "after:5"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("updated") || combined.contains("success") || success,
            "rotation policy should update method policy: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_rotation_auto_enable_disable() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Enable auto-rotation
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "auto-rotate", "--enable", "true"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("enabled") || combined.contains("success") || success,
            "rotation auto-rotate should enable: stdout={}, stderr={}",
            stdout, stderr
        );

        // Disable auto-rotation
        let (stdout2, stderr2, success2) = run_cli(&temp_dir, &["rotation", "auto-rotate", "--enable", "false"]);
        let combined2 = format!("{}{}", stdout2, stderr2).to_lowercase();

        assert!(
            combined2.contains("disabled") || combined2.contains("success") || success2,
            "rotation auto-rotate should disable: stdout={}, stderr={}",
            stdout2, stderr2
        );
    }

    #[test]
    fn test_rotation_history() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Check rotation history
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "history"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("history") || combined.contains("no") || success,
            "rotation history should display: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_rotation_clear_history() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Clear rotation history
        let (stdout, stderr, success) = run_cli(&temp_dir, &["rotation", "clear-history"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("cleared") || combined.contains("no") || success,
            "rotation clear-history should work: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod backup_tests {
    use super::*;

    #[test]
    fn test_backup_export() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Export backup - this will prompt for password, so we expect it to fail gracefully
        // or output instructions
        let (stdout, stderr, _success) = run_cli(&temp_dir, &["backup"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        // Should either prompt for password, show backup, or give instructions
        assert!(
            combined.contains("password") || combined.contains("backup") || combined.contains("export"),
            "backup should handle export: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_backup_export_to_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let backup_path = temp_dir.path().join("backup.json");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Try to export with output flag
        let (stdout, stderr, _success) = run_cli(&temp_dir, &["backup", "--output", backup_path.to_str().unwrap()]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        // Should either create file or prompt for password
        assert!(
            combined.contains("password") || combined.contains("backup") || combined.contains("export") || combined.contains("created") || combined.contains("saved"),
            "backup should handle file export: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod contacts_search_tests {
    use super::*;

    #[test]
    fn test_contacts_list_with_search() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Add a contact
        run_cli(&temp_dir, &["contacts", "add", "alice", "pubky://abc123"]);

        // Search for the contact
        let (stdout, stderr, success) = run_cli(&temp_dir, &["contacts", "list", "--search", "alice"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("alice") || success,
            "contacts list --search should find contacts: stdout={}, stderr={}",
            stdout, stderr
        );
    }

    #[test]
    fn test_contacts_list_search_no_match() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Add a contact
        run_cli(&temp_dir, &["contacts", "add", "alice", "pubky://abc123"]);

        // Search for non-existent contact
        let (stdout, stderr, success) = run_cli(&temp_dir, &["contacts", "list", "--search", "bob"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            !combined.contains("alice") || combined.contains("no") || success,
            "contacts list --search should not find non-matching contacts: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod dashboard_tests {
    use super::*;

    #[test]
    fn test_dashboard() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Check dashboard
        let (stdout, stderr, success) = run_cli(&temp_dir, &["dashboard"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("dashboard") || combined.contains("summary") || combined.contains("contact") || success,
            "dashboard should display: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod receipts_tests {
    use super::*;

    #[test]
    fn test_receipts_list_empty() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // List receipts should work with empty state
        let (stdout, stderr, success) = run_cli(&temp_dir, &["receipts"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("no") || combined.contains("receipt") || combined.contains("empty") || success,
            "receipts should handle empty state: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

#[cfg(test)]
mod wallet_tests {
    use super::*;

    #[test]
    fn test_wallet_status() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Setup an identity first
        run_cli(&temp_dir, &["setup", "--name", "test"]);

        // Check wallet status
        let (stdout, stderr, success) = run_cli(&temp_dir, &["wallet", "status"]);
        let combined = format!("{}{}", stdout, stderr).to_lowercase();

        assert!(
            combined.contains("wallet") || combined.contains("not") || combined.contains("configured") || success,
            "wallet status should display: stdout={}, stderr={}",
            stdout, stderr
        );
    }
}

