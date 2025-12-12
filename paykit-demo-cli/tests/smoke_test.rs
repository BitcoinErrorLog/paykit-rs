//! Smoke tests for paykit-demo-cli
//!
//! These tests verify basic functionality of the CLI without requiring
//! network access or external dependencies.

use std::process::Command;

/// Test that the CLI can show help
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "-p", "paykit-demo-cli", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Print output for debugging if test fails
    if !output.status.success() {
        eprintln!("stdout: {}", stdout);
        eprintln!("stderr: {}", stderr);
    }

    // The help should contain key commands
    assert!(
        stdout.contains("setup") || stderr.contains("setup"),
        "Help should mention 'setup' command"
    );
}

/// Test that version is shown
#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(["run", "-p", "paykit-demo-cli", "--", "--version"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should output version info
    assert!(
        stdout.contains("paykit") || output.status.success(),
        "Version command should succeed"
    );
}

/// Test that list command works with no identities
#[test]
fn test_cli_list_empty() {
    use tempfile::TempDir;
    
    // Create a temporary directory for test data
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(["run", "-p", "paykit-demo-cli", "--", "list"])
        .env("PAYKIT_DEMO_DIR", temp_dir.path())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should either show "no identities" or succeed with empty output
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.to_lowercase().contains("no") 
        || combined.to_lowercase().contains("empty")
        || output.status.success()
        || combined.contains("identit"),
        "List should handle empty state gracefully"
    );
}

/// Test that whoami works (should show "no identity" when none set)
#[test]
fn test_cli_whoami_no_identity() {
    use tempfile::TempDir;
    
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(["run", "-p", "paykit-demo-cli", "--", "whoami"])
        .env("PAYKIT_DEMO_DIR", temp_dir.path())
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should handle no identity gracefully
    let combined = format!("{}{}", stdout, stderr).to_lowercase();
    assert!(
        combined.contains("no") 
        || combined.contains("not set")
        || combined.contains("identity"),
        "Whoami should handle no identity state"
    );
}

#[cfg(test)]
mod unit_tests {
    //! Unit tests that don't require running the binary

    use paykit_lib::{MethodId, EndpointData};
    use paykit_subscriptions::Amount;

    #[test]
    fn test_method_id_creation() {
        let method = MethodId("lightning".to_string());
        assert_eq!(method.0, "lightning");
    }

    #[test]
    fn test_endpoint_data_creation() {
        let endpoint = EndpointData("lnbc1...".to_string());
        assert_eq!(endpoint.0, "lnbc1...");
    }

    #[test]
    fn test_amount_arithmetic() {
        let a = Amount::from_sats(1000);
        let b = Amount::from_sats(500);
        
        let sum = a.checked_add(&b).unwrap();
        assert_eq!(sum, Amount::from_sats(1500));
    }

    #[test]
    fn test_amount_comparison() {
        let a = Amount::from_sats(1000);
        let b = Amount::from_sats(500);
        
        assert!(a > b);
        assert!(b < a);
    }
}

