//! Security configuration and output sanitization for narsil-mcp
//!
//! This module provides security hardening features:
//! - Maximum file size limits to prevent DoS
//! - Secret redaction from tool outputs
//! - Configurable security policies

use regex::Regex;
use std::sync::LazyLock;

/// Security configuration options
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Maximum file size to parse (in bytes). Default: 10MB
    pub max_file_size: usize,

    /// Whether to redact secrets from outputs
    pub redact_secrets: bool,

    /// Read-only mode - disables any write operations
    pub read_only: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            max_file_size: 10 * 1024 * 1024, // 10MB
            redact_secrets: true,
            read_only: true, // Default to read-only for safety
        }
    }
}

/// Patterns for detecting secrets that should be redacted
static SECRET_PATTERNS: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        // API Keys and Tokens
        (
            Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
        (
            Regex::new(r#"(?i)(secret[_-]?key|secretkey)\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
        (
            Regex::new(r#"(?i)(access[_-]?token|accesstoken)\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
        (
            Regex::new(r#"(?i)(auth[_-]?token|authtoken)\s*[:=]\s*["']?([a-zA-Z0-9_-]{20,})["']?"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
        // AWS Keys
        (
            Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            "[AWS_KEY_REDACTED]",
        ),
        (
            Regex::new(r#"(?i)aws[_-]?secret[_-]?access[_-]?key\s*[:=]\s*["']?([a-zA-Z0-9/+=]{40})["']?"#)
                .unwrap(),
            "AWS_SECRET_ACCESS_KEY=[REDACTED]",
        ),
        // GitHub Tokens
        (
            Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),
            "[GITHUB_TOKEN_REDACTED]",
        ),
        (
            Regex::new(r"gho_[a-zA-Z0-9]{36}").unwrap(),
            "[GITHUB_OAUTH_REDACTED]",
        ),
        (
            Regex::new(r"ghu_[a-zA-Z0-9]{36}").unwrap(),
            "[GITHUB_USER_TOKEN_REDACTED]",
        ),
        (
            Regex::new(r"ghs_[a-zA-Z0-9]{36}").unwrap(),
            "[GITHUB_SERVER_TOKEN_REDACTED]",
        ),
        (
            Regex::new(r"github_pat_[a-zA-Z0-9_]{22,}").unwrap(),
            "[GITHUB_PAT_REDACTED]",
        ),
        // Private Keys
        (
            Regex::new(r"-----BEGIN (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----")
                .unwrap(),
            "[PRIVATE_KEY_REDACTED]",
        ),
        // Passwords in connection strings
        (
            Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*["']?([^"'\s]{8,})["']?"#).unwrap(),
            "$1=[REDACTED]",
        ),
        // Bearer tokens
        (
            Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_=-]+\.[a-zA-Z0-9_=-]+\.?[a-zA-Z0-9_=-]*")
                .unwrap(),
            "Bearer [JWT_REDACTED]",
        ),
        // Database connection strings
        (
            Regex::new(r"(?i)(mongodb|postgres|mysql|redis)://[^@]+@").unwrap(),
            "$1://[CREDENTIALS_REDACTED]@",
        ),
        // Slack tokens
        (
            Regex::new(r"xox[baprs]-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*").unwrap(),
            "[SLACK_TOKEN_REDACTED]",
        ),
        // Stripe keys
        (
            Regex::new(r"sk_live_[a-zA-Z0-9]{24,}").unwrap(),
            "[STRIPE_KEY_REDACTED]",
        ),
        (
            Regex::new(r"rk_live_[a-zA-Z0-9]{24,}").unwrap(),
            "[STRIPE_RESTRICTED_KEY_REDACTED]",
        ),
        // SendGrid API keys
        (
            Regex::new(r"SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}").unwrap(),
            "[SENDGRID_KEY_REDACTED]",
        ),
        // Twilio keys
        (
            Regex::new(r"SK[a-f0-9]{32}").unwrap(),
            "[TWILIO_KEY_REDACTED]",
        ),
        // Generic long hex strings that look like secrets
        (
            Regex::new(r#"(?i)(secret|token|key|credential|auth).*["']([a-f0-9]{32,64})["']"#)
                .unwrap(),
            "$1=[REDACTED]",
        ),
    ]
});

/// Redact secrets from a string
///
/// Scans the input for patterns that look like secrets (API keys, tokens,
/// passwords, private keys, etc.) and replaces them with redaction markers.
pub fn redact_secrets(input: &str) -> String {
    let mut result = input.to_string();

    for (pattern, replacement) in SECRET_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }

    result
}

/// Check if a file should be skipped due to size limits
pub fn should_skip_file(size: usize, config: &SecurityConfig) -> bool {
    size > config.max_file_size
}

/// Check if a file path looks like it contains sensitive data
pub fn is_sensitive_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    let sensitive_patterns = [
        ".env",
        ".pem",
        ".key",
        ".p12",
        ".pfx",
        "credentials",
        "secrets",
        ".htpasswd",
        "id_rsa",
        "id_dsa",
        "id_ecdsa",
        "id_ed25519",
        ".npmrc",
        ".pypirc",
        ".netrc",
        "aws_access",
        "gcloud",
        "keystore",
    ];

    sensitive_patterns
        .iter()
        .any(|p| path_lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_api_keys() {
        let input = r#"api_key = "sk_test_abc123def456ghi789jkl012mno345""#;
        let result = redact_secrets(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("abc123"));
    }

    #[test]
    fn test_redact_aws_key() {
        let input = "aws_access_key_id = AKIAIOSFODNN7EXAMPLE";
        let result = redact_secrets(input);
        assert!(result.contains("[AWS_KEY_REDACTED]"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn test_redact_github_token() {
        let input = "token: ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
        let result = redact_secrets(input);
        assert!(result.contains("[GITHUB_TOKEN_REDACTED]"));
    }

    #[test]
    fn test_redact_private_key() {
        let input = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA0Z3VS5JJcds3
-----END RSA PRIVATE KEY-----"#;
        let result = redact_secrets(input);
        assert!(result.contains("[PRIVATE_KEY_REDACTED]"));
    }

    #[test]
    fn test_redact_password() {
        let input = r#"password = "super_secret_password_123""#;
        let result = redact_secrets(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("super_secret"));
    }

    #[test]
    fn test_redact_connection_string() {
        let input = "mongodb://user:password123@localhost:27017/db";
        let result = redact_secrets(input);
        assert!(result.contains("[CREDENTIALS_REDACTED]"));
        assert!(!result.contains("password123"));
    }

    #[test]
    fn test_no_false_positives() {
        // Normal code shouldn't be redacted
        let input = "let api = createApi(); let key = 'short';";
        let result = redact_secrets(input);
        assert_eq!(input, result);
    }

    #[test]
    fn test_is_sensitive_file() {
        assert!(is_sensitive_file(".env"));
        assert!(is_sensitive_file("/home/user/.env.local"));
        assert!(is_sensitive_file("config/credentials.json"));
        assert!(is_sensitive_file("/home/user/.ssh/id_rsa"));
        assert!(!is_sensitive_file("src/main.rs"));
        assert!(!is_sensitive_file("package.json"));
    }

    #[test]
    fn test_should_skip_file() {
        let config = SecurityConfig::default();
        assert!(!should_skip_file(1000, &config)); // 1KB - OK
        assert!(!should_skip_file(1_000_000, &config)); // 1MB - OK
        assert!(should_skip_file(20_000_000, &config)); // 20MB - Too large
    }

    #[test]
    fn test_default_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(config.redact_secrets);
        assert!(config.read_only);
    }
}
