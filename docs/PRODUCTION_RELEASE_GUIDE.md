# Production Release Guide - v1.1.2

**IMPORTANT:** Follow these steps in order. All package managers must be updated together - we cannot have mismatched versions deployed.

## ✅ Pre-Release Checklist

Before creating the production release, ensure:

- [x] Beta release (v1.1.2-beta) created and tested
- [x] Homebrew tested and working ✅
- [ ] Scoop tested on Windows (waiting for friend)
- [ ] Ready to publish to all package managers simultaneously

---

## 📋 Step-by-Step Production Release

### Step 1: Create Production Release Tag

Once Scoop testing passes, create the production release:

```bash
cd ~/RustroverProjects/narsil-mcp

# Create production tag (NO -beta suffix)
git tag v1.1.2
git push --tags
```

**What happens:**
- GitHub Actions workflow triggers
- Runs tests, lint, security audit
- **Publishes to crates.io** (because version doesn't contain `-beta`, `-alpha`, or `-rc`)
- Builds all platform binaries
- Creates GitHub release with tarballs and SHA256 files

**Monitor:**
- Go to https://github.com/postrv/narsil-mcp/actions
- Verify all jobs pass (especially `publish` to crates.io)
- Check release assets are created with SHA256 files

---

### Step 2: Update Homebrew Formula

Once the GitHub release is created:

```bash
cd ~/RustroverProjects/homebrew-narsil

# Run update script
./update-formula.sh v1.1.2

# Review changes
git diff Formula/narsil-mcp.rb

# Commit and push
git add Formula/narsil-mcp.rb
git commit -m "chore: update to v1.1.2"
git push
```

**Verify:**
```bash
# Test locally (optional)
brew uninstall narsil-mcp
brew install narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

---

### Step 3: Update Scoop Manifest

```bash
cd ~/RustroverProjects/scoop-narsil

# Run update script
./update-manifest.sh v1.1.2

# Review changes
git diff bucket/narsil-mcp.json

# Commit and push
git add bucket/narsil-mcp.json
git commit -m "chore: update to v1.1.2"
git push
```

**Verify:**
```powershell
# On Windows (optional)
scoop uninstall narsil-mcp
scoop install narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

---

### Step 4: Update AUR Packages

#### 4A: Source Package (narsil-mcp)

```bash
cd ~/RustroverProjects/aur-narsil-mcp

# Run update script
./update-pkgbuild.sh v1.1.2

# Review changes
git diff PKGBUILD

# Test build locally (IMPORTANT!)
makepkg -si

# If build succeeds, publish to AUR
git add PKGBUILD .SRCINFO
git commit -m "Update to v1.1.2"
git push
```

#### 4B: Binary Package (narsil-mcp-bin)

```bash
cd ~/RustroverProjects/aur-narsil-mcp-bin

# Run update script
./update-pkgbuild.sh v1.1.2

# Review changes
git diff PKGBUILD

# Test build locally (IMPORTANT!)
makepkg -si

# If build succeeds, publish to AUR
git add PKGBUILD .SRCINFO
git commit -m "Update to v1.1.2"
git push
```

**Note:** Testing locally with `makepkg -si` is critical. It will:
- Download the tarball from GitHub
- Verify SHA256 checksum
- Build (source) or install (binary) the package
- Install it on your system

If this fails, the update scripts may need adjustment.

---

### Step 5: Publish npm Packages

**Prerequisites:**
- npm account with access to publish `@narsil-mcp/*` packages
- Logged in via `npm login`

```bash
cd ~/RustroverProjects/npm-narsil-mcp

# Check npm login status
npm whoami
# If not logged in, run: npm login

# Run publish script
./publish-npm.sh v1.1.2
```

**What the script does:**
1. Downloads all platform binaries from GitHub release
2. Extracts them to npm/darwin-x64/, npm/darwin-arm64/, etc.
3. Updates version in all package.json files
4. Verifies binaries are present
5. **Asks for confirmation** before publishing
6. Publishes platform-specific packages first:
   - `@narsil-mcp/darwin-x64@1.1.2`
   - `@narsil-mcp/darwin-arm64@1.1.2`
   - `@narsil-mcp/linux-x64@1.1.2`
   - `@narsil-mcp/win32-x64@1.1.2`
7. Waits 10 seconds for npm registry propagation
8. Publishes main package: `narsil-mcp@1.1.2`

**When prompted "Continue with publish? (y/N)":**
- Review the output carefully
- Ensure all binaries downloaded successfully
- Type `y` and press Enter to publish

**Verify:**
- Wait ~1 minute for npm registry to update
- Check: https://www.npmjs.com/package/narsil-mcp
- Test installation:
  ```bash
  npm install -g narsil-mcp
  narsil-mcp --version  # Should show 1.1.2
  ```

---

## 🔍 Post-Release Verification

After all package managers are updated, verify they all work:

### crates.io
```bash
cargo install narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

### Homebrew (macOS/Linux)
```bash
brew tap postrv/narsil
brew install narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

### Scoop (Windows)
```powershell
scoop bucket add narsil https://github.com/postrv/scoop-narsil
scoop install narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

### AUR (Arch Linux)
```bash
# Binary (faster)
yay -S narsil-mcp-bin
narsil-mcp --version  # Should show 1.1.2

# Source
yay -S narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

### npm (All Platforms)
```bash
npm install -g narsil-mcp
narsil-mcp --version  # Should show 1.1.2
```

---

## 📦 Package Manager URLs

After release, verify these URLs are live and correct:

- **crates.io**: https://crates.io/crates/narsil-mcp
- **npm**: https://www.npmjs.com/package/narsil-mcp
- **Homebrew**: `brew info narsil-mcp` (after tapping)
- **Scoop**: https://github.com/postrv/scoop-narsil
- **AUR (source)**: https://aur.archlinux.org/packages/narsil-mcp
- **AUR (binary)**: https://aur.archlinux.org/packages/narsil-mcp-bin

---

## 🐛 Troubleshooting

### Issue: GitHub Actions fails on publish step

**Symptoms:** `publish` job fails with "version already exists"

**Cause:** You created a tag without the `-beta` suffix after v1.1.1 was already published

**Solution:**
1. Delete the tag: `git tag -d v1.1.2 && git push --delete origin v1.1.2`
2. Bump version in `Cargo.toml` to `1.1.3` or `1.2.0`
3. Commit: `git add Cargo.toml && git commit -m "chore: bump version to 1.1.3"`
4. Tag: `git tag v1.1.3 && git push --tags`

---

### Issue: npm publish fails with "package already exists"

**Symptoms:** `npm publish` returns 403 or "cannot publish over existing version"

**Cause:** Version 1.1.2 was already published to npm

**Solution:**
1. Bump version in `Cargo.toml` and run the release flow again with a new version
2. Or, if this is a mistake: wait 24 hours and use `npm unpublish narsil-mcp@1.1.2 --force` (only works if published &lt;24h ago)

---

### Issue: AUR build fails

**Symptoms:** `makepkg` fails with "Failed to download source" or "Integrity check failed"

**Cause 1:** SHA256 checksum doesn't match
- Verify the checksum in PKGBUILD matches the one in GitHub releases
- Re-run `./update-pkgbuild.sh v1.1.2` to fetch the correct checksum

**Cause 2:** Tarball not available on GitHub
- Check https://github.com/postrv/narsil-mcp/releases/tag/v1.1.2
- Ensure the tarball exists and is publicly accessible

---

### Issue: Homebrew formula fails to install

**Symptoms:** `brew install narsil-mcp` fails with "SHA256 mismatch"

**Cause:** SHA256 in formula doesn't match the actual tarball

**Solution:**
1. Fetch the correct SHA256 from GitHub:
   ```bash
   curl -fsSL https://github.com/postrv/narsil-mcp/releases/download/v1.1.2/narsil-mcp-v1.1.2-macos-aarch64.tar.gz.sha256
   ```
2. Update the formula manually or re-run `./update-formula.sh v1.1.2`

---

## 📝 Release Announcement Template

After verifying all package managers work, create a GitHub release announcement:

```markdown
# narsil-mcp v1.1.2

Production release with multi-platform package manager distribution!

## 🎉 Installation

**Homebrew (macOS/Linux):**
\`\`\`bash
brew tap postrv/narsil
brew install narsil-mcp
\`\`\`

**Scoop (Windows):**
\`\`\`powershell
scoop bucket add narsil https://github.com/postrv/scoop-narsil
scoop install narsil-mcp
\`\`\`

**AUR (Arch Linux):**
\`\`\`bash
yay -S narsil-mcp-bin  # Fast binary install
# or
yay -S narsil-mcp      # Build from source
\`\`\`

**npm (All Platforms):**
\`\`\`bash
npm install -g narsil-mcp
\`\`\`

**Cargo (All Platforms):**
\`\`\`bash
cargo install narsil-mcp
\`\`\`

## 📦 Distribution

Now available on:
- ✅ [crates.io](https://crates.io/crates/narsil-mcp)
- ✅ [npm](https://www.npmjs.com/package/narsil-mcp)
- ✅ [Homebrew](https://github.com/postrv/homebrew-narsil)
- ✅ [Scoop](https://github.com/postrv/scoop-narsil)
- ✅ [AUR](https://aur.archlinux.org/packages/narsil-mcp)

## 🐛 Bug Fixes

- [List any bug fixes from beta testing]

## 📚 Documentation

See [INSTALL.md](https://github.com/postrv/narsil-mcp/blob/main/docs/INSTALL.md) for detailed installation instructions.

Full changelog: [CHANGELOG.md](https://github.com/postrv/narsil-mcp/blob/main/CHANGELOG.md)
\`\`\`

---

## ✅ Success Criteria

The release is successful when:

- [ ] All GitHub Actions jobs pass (including `publish` to crates.io)
- [ ] crates.io shows version 1.1.2
- [ ] npm shows version 1.1.2
- [ ] Homebrew installs version 1.1.2
- [ ] Scoop installs version 1.1.2
- [ ] AUR packages install version 1.1.2
- [ ] All `--version` commands return "narsil-mcp 1.1.2"
- [ ] Release announcement created on GitHub

---

## 🔄 Future Releases

For subsequent releases (e.g., v1.1.3, v1.2.0):

1. Update version in `Cargo.toml`
2. Commit: `git add Cargo.toml && git commit -m "chore: bump version to X.Y.Z"`
3. Tag: `git tag vX.Y.Z && git push --tags`
4. Wait for GitHub Actions to complete
5. Run update scripts for all package managers:
   - `cd ~/RustroverProjects/homebrew-narsil && ./update-formula.sh vX.Y.Z && git add . && git commit -m "chore: update to vX.Y.Z" && git push`
   - `cd ~/RustroverProjects/scoop-narsil && ./update-manifest.sh vX.Y.Z && git add . && git commit -m "chore: update to vX.Y.Z" && git push`
   - `cd ~/RustroverProjects/aur-narsil-mcp && ./update-pkgbuild.sh vX.Y.Z && makepkg -si && git add . && git commit -m "Update to vX.Y.Z" && git push`
   - `cd ~/RustroverProjects/aur-narsil-mcp-bin && ./update-pkgbuild.sh vX.Y.Z && makepkg -si && git add . && git commit -m "Update to vX.Y.Z" && git push`
   - `cd ~/RustroverProjects/npm-narsil-mcp && ./publish-npm.sh vX.Y.Z`
6. Verify all package managers
7. Create release announcement

**Estimated time per release:** 1-2 hours (after this initial setup)
