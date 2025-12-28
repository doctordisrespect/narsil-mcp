/// Editor detection and preset mapping
///
/// Maps MCP client names to appropriate tool presets based on editor capabilities
/// and performance characteristics.
use super::preset::Preset;

/// Map an editor name to a preset
///
/// # Arguments
/// * `editor_name` - The name from MCP clientInfo (case-insensitive)
///
/// # Returns
/// * `Some(Preset)` - If the editor is recognized
/// * `None` - If the editor is unknown (caller should use Full preset)
///
/// # Supported Editors
/// - **VS Code / Code**: Balanced preset (40-50 tools)
/// - **Zed**: Minimal preset (20-30 tools, optimized for speed)
/// - **Claude Desktop / Claude**: Full preset (all tools)
/// - **Unknown**: Full preset by default (conservative choice)
pub fn get_editor_preset(editor_name: &str) -> Option<Preset> {
    let normalized = editor_name.trim().to_lowercase();

    match normalized.as_str() {
        // VS Code and variants
        "vscode" | "code" | "visual studio code" => Some(Preset::Balanced),

        // Zed editor - optimized for speed
        "zed" => Some(Preset::Minimal),

        // Claude Desktop - full capabilities
        "claude-desktop" | "claude" | "claude.ai" => Some(Preset::Full),

        // JetBrains IDEs - balanced
        "intellij" | "idea" | "pycharm" | "webstorm" | "rustrover" | "clion" | "goland"
        | "phpstorm" | "rider" => Some(Preset::Balanced),

        // Vim/Neovim - minimal for speed
        "vim" | "nvim" | "neovim" => Some(Preset::Minimal),

        // Emacs - balanced
        "emacs" => Some(Preset::Balanced),

        // Sublime Text - balanced
        "sublime" | "sublime text" | "subl" => Some(Preset::Balanced),

        // Cursor - VS Code fork
        "cursor" => Some(Preset::Balanced),

        // Unknown editor - return None to signal "use full preset"
        _ => None,
    }
}

/// Get the preset for an editor, with fallback to Full
///
/// This is a convenience wrapper around `get_editor_preset` that always
/// returns a preset (never None).
pub fn get_editor_preset_or_full(editor_name: &str) -> Preset {
    get_editor_preset(editor_name).unwrap_or(Preset::Full)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vscode_detection() {
        assert_eq!(
            get_editor_preset("vscode"),
            Some(Preset::Balanced),
            "vscode should map to Balanced"
        );
        assert_eq!(
            get_editor_preset("code"),
            Some(Preset::Balanced),
            "code should map to Balanced"
        );
        assert_eq!(
            get_editor_preset("visual studio code"),
            Some(Preset::Balanced)
        );
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(get_editor_preset("VSCode"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("VSCODE"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("VsCode"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("Zed"), Some(Preset::Minimal));
        assert_eq!(get_editor_preset("ZED"), Some(Preset::Minimal));
    }

    #[test]
    fn test_zed_detection() {
        assert_eq!(
            get_editor_preset("zed"),
            Some(Preset::Minimal),
            "zed should map to Minimal"
        );
    }

    #[test]
    fn test_claude_detection() {
        assert_eq!(get_editor_preset("claude-desktop"), Some(Preset::Full));
        assert_eq!(get_editor_preset("claude"), Some(Preset::Full));
        assert_eq!(get_editor_preset("claude.ai"), Some(Preset::Full));
    }

    #[test]
    fn test_jetbrains_detection() {
        assert_eq!(get_editor_preset("intellij"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("idea"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("pycharm"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("webstorm"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("rustrover"), Some(Preset::Balanced));
    }

    #[test]
    fn test_vim_detection() {
        assert_eq!(get_editor_preset("vim"), Some(Preset::Minimal));
        assert_eq!(get_editor_preset("nvim"), Some(Preset::Minimal));
        assert_eq!(get_editor_preset("neovim"), Some(Preset::Minimal));
    }

    #[test]
    fn test_emacs_detection() {
        assert_eq!(get_editor_preset("emacs"), Some(Preset::Balanced));
    }

    #[test]
    fn test_sublime_detection() {
        assert_eq!(get_editor_preset("sublime"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("sublime text"), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("subl"), Some(Preset::Balanced));
    }

    #[test]
    fn test_cursor_detection() {
        assert_eq!(get_editor_preset("cursor"), Some(Preset::Balanced));
    }

    #[test]
    fn test_unknown_editor() {
        assert_eq!(get_editor_preset("unknown-editor"), None);
        assert_eq!(get_editor_preset("some-new-ide"), None);
        assert_eq!(get_editor_preset(""), None);
    }

    #[test]
    fn test_fallback_to_full() {
        assert_eq!(
            get_editor_preset_or_full("unknown"),
            Preset::Full,
            "Unknown editors should default to Full preset"
        );
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(get_editor_preset(" vscode "), Some(Preset::Balanced));
        assert_eq!(get_editor_preset("  zed  "), Some(Preset::Minimal));
    }
}
