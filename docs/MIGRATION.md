# Migration Guide

## Upgrading from v1.0.2 to v1.1.0

This guide helps you migrate to narsil-mcp v1.1.0, which introduces the tool selection and configuration system.

---

## TL;DR - Do I Need to Change Anything?

**No!** Existing usage continues to work without modification. All 69 tools are still available by default.

### Existing Usage (Still Works)

```bash
# v1.0.2 usage - still works in v1.1.0
narsil-mcp --repos ~/project --git --call-graph

# Still returns all 69 tools
# No changes required
```

### New Optional Features

```bash
# v1.1.0 - Apply Minimal preset for faster editors
narsil-mcp --repos ~/project --preset minimal

# v1.1.0 - Create user config
narsil-mcp config init

# v1.1.0 - List available tools
narsil-mcp tools list
```

---

## What's New in v1.1.0

### 1. Automatic Editor Detection

narsil-mcp now detects your editor from the MCP `initialize` request and applies an optimal preset:

| Editor | Preset | Tools | Tokens | Why |
|--------|--------|-------|--------|-----|
| **Zed** | Minimal | 26 | ~4,686 | Fast startup, minimal context |
| **Cursor** | Minimal | 26 | ~4,686 | Optimized for code completion |
| **VS Code** | Balanced | 51 | ~8,948 | Good feature balance |
| **IntelliJ** | Balanced | 51 | ~8,948 | Good feature balance |
| **Neovim** | Balanced | 51 | ~8,948 | Good feature balance |
| **Claude Desktop** | Full | 69 | ~12,001 | Maximum capabilities |
| **Unknown** | Full | 69 | ~12,001 | All features enabled |

**What this means:**
- If you use Zed, you'll automatically get a faster, lighter tool set (61% fewer tokens!)
- If you use VS Code, you'll get a balanced set with Git + security tools (25% fewer tokens)
- If you use Claude Desktop, you'll get all 69 tools (no change)

**How to disable:**
```bash
# Force Full preset for all editors
narsil-mcp --repos ~/project --preset full
```

---

### 2. Configuration Files (Optional)

You can now save your preferences in YAML configuration files:

#### User Config

Create `~/.config/narsil-mcp/config.yaml`:

```yaml
version: "1.0"
preset: "balanced"  # minimal, balanced, full, security-focused

tools:
  # Disable slow tools
  overrides:
    neural_search:
      enabled: false
      reason: "Too slow for interactive use"
    generate_sbom:
      enabled: false
      reason: "Only needed for security audits"

performance:
  max_tool_count: 50  # Limit total tools
```

#### Project Config

Create `.narsil.yaml` in your repo root:

```yaml
version: "1.0"
preset: "security-focused"  # Override user preset for this project

tools:
  categories:
    Security:
      enabled: true
    SupplyChain:
      enabled: true
```

**Priority (highest to lowest):**
1. CLI flags (`--preset minimal`)
2. Environment variables (`NARSIL_PRESET=minimal`)
3. Project config (`.narsil.yaml`)
4. User config (`~/.config/narsil-mcp/config.yaml`)
5. Default config (built-in)

---

### 3. New CLI Commands

```bash
# Generate default config
narsil-mcp config init

# View current effective config
narsil-mcp config show

# Validate a config file
narsil-mcp config validate ~/.config/narsil-mcp/config.yaml

# Apply a preset
narsil-mcp config preset minimal

# List available tools
narsil-mcp tools list

# List tools in a category
narsil-mcp tools list --category Search

# Search for tools
narsil-mcp tools search "git"

# Export current config
narsil-mcp config export > my-config.yaml
```

---

## Migration Scenarios

### Scenario 1: "I just want it to work like v1.0.2"

**No action required!** All 69 tools are enabled by default if you don't create any config files.

```bash
# This works exactly the same in v1.1.0
narsil-mcp --repos ~/project --git --call-graph
```

---

### Scenario 2: "I use Zed and it's slow to start"

**Automatic:** Zed now gets the Minimal preset automatically (26 tools, 61% fewer tokens)

**Manual override if needed:**
```bash
# Force Full preset
narsil-mcp --repos ~/project --preset full
```

**Or create user config:**
```yaml
# ~/.config/narsil-mcp/config.yaml
version: "1.0"
preset: "full"  # Override Zed's minimal preset
```

---

### Scenario 3: "I want to disable slow tools"

**Create user config:**

```bash
# Interactive wizard
narsil-mcp config init
# Select "Custom" preset
# Choose which tools to disable
```

**Or manually create:**
```yaml
# ~/.config/narsil-mcp/config.yaml
version: "1.0"

tools:
  overrides:
    # Disable neural embeddings (slow, requires API key)
    neural_search:
      enabled: false
    find_semantic_clones:
      enabled: false

    # Disable supply chain tools (slow for large projects)
    generate_sbom:
      enabled: false
    check_dependencies:
      enabled: false
    check_licenses:
      enabled: false
```

---

### Scenario 4: "I have a security-focused project"

**Create project config:**

```yaml
# .narsil.yaml in your repo root
version: "1.0"
preset: "security-focused"

tools:
  categories:
    Security:
      enabled: true
      description: "OWASP, CWE, secrets scanning"
    SupplyChain:
      enabled: true
      description: "SBOM, dependencies, licenses"
```

**Now all team members get security tools automatically!**

---

### Scenario 5: "I want different presets for different repos"

**Use project configs:**

```bash
# In ~/work/critical-app/.narsil.yaml
preset: "security-focused"

# In ~/personal/blog/.narsil.yaml
preset: "minimal"

# In ~/open-source/library/.narsil.yaml
preset: "balanced"
```

Each repo gets its own preset automatically when you start narsil-mcp in that directory.

---

## Breaking Changes

### None! ✅

v1.1.0 is 100% backwards compatible with v1.0.2:
- All CLI flags work the same
- All 69 tools still available by default
- No configuration files required
- Existing integrations continue working

---

## Opt-In Features Only

### What Changes Automatically

1. **Editor Detection**: Editors get optimized presets
   - Zed → Minimal (26 tools)
   - VS Code → Balanced (51 tools)
   - Claude Desktop → Full (69 tools)

**How to disable:**
```bash
export NARSIL_PRESET=full  # Force all editors to use Full preset
```

### What Requires Opt-In

1. **Config files**: Only used if you create them
2. **Tool filtering**: Only applied if you set preset or overrides
3. **Environment variables**: Only used if you set them

---

## Troubleshooting

### "I'm not getting all the tools I expect"

**Check which preset is active:**
```bash
narsil-mcp config show
# Look for "preset" field
```

**Force Full preset:**
```bash
narsil-mcp --repos ~/project --preset full
```

**Or via environment:**
```bash
export NARSIL_PRESET=full
narsil-mcp --repos ~/project
```

---

### "Config file isn't working"

**Validate syntax:**
```bash
narsil-mcp config validate ~/.config/narsil-mcp/config.yaml
```

**Check priority order:**
```bash
narsil-mcp config show --verbose
# Shows which config sources were loaded
```

**Common issues:**
- Config file not in the right location (`~/.config/narsil-mcp/config.yaml`)
- Invalid YAML syntax (use `config validate`)
- CLI flags override config file (expected behavior)

---

### "Tools are disabled even though I enabled --git"

**Check feature flags vs preset:**

Presets can disable tools even if feature flags are enabled:

```yaml
# This disables Git tools despite --git flag
preset: "minimal"  # Minimal doesn't include Git tools
```

**Solution:**
```bash
# Force Full preset
narsil-mcp --repos ~/project --git --preset full

# Or use Balanced preset (includes Git)
narsil-mcp --repos ~/project --git --preset balanced
```

---

## Environment Variables

### New in v1.1.0

```bash
# Override config path
export NARSIL_CONFIG_PATH=/path/to/config.yaml

# Apply preset
export NARSIL_PRESET=minimal

# Enable specific categories
export NARSIL_ENABLED_CATEGORIES=Repository,Symbols,Search

# Disable specific tools
export NARSIL_DISABLED_TOOLS=neural_search,generate_sbom
```

### Existing (Still Work)

```bash
# GitHub token for remote repos
export GITHUB_TOKEN=ghp_xxxx

# Embedding API keys
export VOYAGE_API_KEY=pa_xxxx
export OPENAI_API_KEY=sk_xxxx
export EMBEDDING_API_KEY=xxxx

# Logging
export RUST_LOG=debug
```

---

## Performance Impact

### Token Usage Reduction

| Scenario | Before (v1.0.2) | After (v1.1.0) | Reduction |
|----------|-----------------|----------------|-----------|
| **Zed editor** | 69 tools<br>~12,001 tokens | 26 tools<br>~4,686 tokens | **61% fewer tokens** |
| **VS Code** | 69 tools<br>~12,001 tokens | 51 tools<br>~8,948 tokens | **25% fewer tokens** |
| **Claude Desktop** | 69 tools<br>~12,001 tokens | 69 tools<br>~12,001 tokens | No change |

See [PERFORMANCE.md](./PERFORMANCE.md) for detailed benchmarks.

---

## Rollback Plan

### If you encounter issues:

1. **Force Full preset:**
   ```bash
   export NARSIL_PRESET=full
   ```

2. **Remove config files:**
   ```bash
   rm ~/.config/narsil-mcp/config.yaml
   rm .narsil.yaml
   ```

3. **Downgrade to v1.0.2:**
   ```bash
   cargo install narsil-mcp@1.0.2
   ```

---

## FAQ

### Q: Will my existing MCP client integration break?

**A:** No. All MCP protocol interactions remain the same. The only change is which tools are advertised in the `tools/list` response.

---

### Q: Can I still use all 69 tools?

**A:** Yes! Use `--preset full` or don't create any config files.

---

### Q: What if my editor isn't recognized?

**A:** Unknown editors get the Full preset (all 69 tools) by default.

---

### Q: Can I create custom presets?

**A:** Yes, via config files. Define your own tool whitelist:

```yaml
version: "1.0"
tools:
  # Enable only specific categories
  categories:
    Repository:
      enabled: true
    Symbols:
      enabled: true
    Git:
      enabled: true

  # Or disable specific tools
  overrides:
    neural_search:
      enabled: false
```

---

### Q: How do I know which preset is active?

**A:**
```bash
narsil-mcp config show | grep preset
```

Or check the MCP server logs:
```
INFO narsil_mcp::mcp: MCP client detected: vscode
INFO narsil_mcp::config::filter: Applying preset: Balanced
```

---

### Q: Can different team members use different presets?

**A:** Yes! User configs are per-machine. Project configs affect everyone, but can be overridden by user configs or CLI flags.

---

## Support

### Get Help

1. **Check documentation:**
   - [README.md](../README.md) - Installation and usage
   - [PERFORMANCE.md](./PERFORMANCE.md) - Benchmarks and analysis
   - [Configuration Guide](./CONFIGURATION.md) - Detailed config docs (coming soon)

2. **Debug logging:**
   ```bash
   RUST_LOG=debug narsil-mcp --repos ~/project
   ```

3. **Report issues:**
   - GitHub: https://github.com/postrv/narsil-mcp/issues
   - Include output of: `narsil-mcp config show`

---

## Feedback

We'd love to hear about your migration experience!

- **Reddit:** Share your experience at r/ClaudeAI
- **GitHub:** Open an issue or discussion
- **Discord:** Join the community (link in README)

---

*Last Updated: 2025-12-28*
*narsil-mcp v1.1.0*
