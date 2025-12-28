/// Configuration system for narsil-mcp
///
/// This module provides a flexible configuration system that supports:
/// - YAML configuration files
/// - Multi-source config loading (default, user, project)
/// - Configuration validation
/// - Environment variable overrides
///
/// Configuration priority (highest to lowest):
/// 1. CLI flags (handled in main.rs)
/// 2. Environment variables (NARSIL_*)
/// 3. Project config (.narsil.yaml in repo root)
/// 4. User config (~/.config/narsil-mcp/config.yaml)
/// 5. Default config (built-in)
pub mod cli;
pub mod editor;
pub mod filter;
pub mod loader;
pub mod preset;
pub mod schema;
pub mod validation;

// Re-export main types used by other modules and tests
pub use cli::{handle_config_command, handle_tools_command, ConfigCommand, ToolsCommand};
pub use filter::{ClientInfo, ToolFilter};
pub use loader::ConfigLoader;
pub use validation::validate_config;

// Re-export schema types for public API. These are used by integration tests
// and potentially external users to construct configurations programmatically.
// The binary doesn't use them directly (only via ToolConfig serialization),
// but they're essential parts of the library's public API.
//
// Suppress unused warnings for nested types that are part of ToolConfig's structure.
// The binary deserializes these from YAML but never constructs them directly.
#[allow(unused_imports)]
pub use schema::{CategoryConfig, PerformanceConfig, ToolConfig, ToolOverride, ToolsConfig};

// Note: Preset is an internal implementation detail of the filter module and
// is not re-exported. External code should use preset strings in YAML configs.
