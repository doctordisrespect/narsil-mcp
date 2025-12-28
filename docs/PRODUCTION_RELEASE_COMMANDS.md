# Production Release - Quick Command Reference

## When Scoop Testing Passes: Run These Commands

### 1. Create Production Release
```bash
cd ~/RustroverProjects/narsil-mcp
git tag v1.1.2
git push --tags
# Wait for GitHub Actions to complete: https://github.com/postrv/narsil-mcp/actions
```

---

### 2. Update Homebrew
```bash
cd ~/RustroverProjects/homebrew-narsil
./update-formula.sh v1.1.2
git diff Formula/narsil-mcp.rb  # Review
git add Formula/narsil-mcp.rb
git commit -m "chore: update to v1.1.2"
git push
```

---

### 3. Update Scoop
```bash
cd ~/RustroverProjects/scoop-narsil
./update-manifest.sh v1.1.2
git diff bucket/narsil-mcp.json  # Review
git add bucket/narsil-mcp.json
git commit -m "chore: update to v1.1.2"
git push
```

---

### 4. Update AUR (Source Package)
```bash
cd ~/RustroverProjects/aur-narsil-mcp
./update-pkgbuild.sh v1.1.2
git diff PKGBUILD  # Review
makepkg -si  # Test build locally - CRITICAL!
# If successful:
git add PKGBUILD .SRCINFO
git commit -m "Update to v1.1.2"
git push
```

---

### 5. Update AUR (Binary Package)
```bash
cd ~/RustroverProjects/aur-narsil-mcp-bin
./update-pkgbuild.sh v1.1.2
git diff PKGBUILD  # Review
makepkg -si  # Test build locally - CRITICAL!
# If successful:
git add PKGBUILD .SRCINFO
git commit -m "Update to v1.1.2"
git push
```

---

### 6. Publish npm
```bash
cd ~/RustroverProjects/npm-narsil-mcp

# Ensure you're logged in
npm whoami  # If not logged in: npm login

# Run publish script
./publish-npm.sh v1.1.2
# When prompted "Continue with publish? (y/N)", type: y
```

---

## Verification Commands

After all packages are published:

```bash
# crates.io
cargo install narsil-mcp
narsil-mcp --version  # Should show: narsil-mcp 1.1.2

# Homebrew
brew install narsil-mcp
narsil-mcp --version  # Should show: narsil-mcp 1.1.2

# npm
npm install -g narsil-mcp
narsil-mcp --version  # Should show: narsil-mcp 1.1.2
```

On Windows:
```powershell
# Scoop
scoop install narsil-mcp
narsil-mcp --version  # Should show: narsil-mcp 1.1.2
```

On Arch Linux:
```bash
# AUR
yay -S narsil-mcp-bin
narsil-mcp --version  # Should show: narsil-mcp 1.1.2
```

---

## Package Registry URLs

Verify these after release:

- crates.io: https://crates.io/crates/narsil-mcp
- npm: https://www.npmjs.com/package/narsil-mcp
- Homebrew: https://github.com/postrv/homebrew-narsil
- Scoop: https://github.com/postrv/scoop-narsil
- AUR (source): https://aur.archlinux.org/packages/narsil-mcp
- AUR (binary): https://aur.archlinux.org/packages/narsil-mcp-bin

---

## Timeline Estimate

- Step 1 (GitHub Actions): 15-20 minutes
- Steps 2-6 (Update all package managers): 30-45 minutes
- Verification: 15-30 minutes
- **Total: 1-2 hours**

---

## Key Notes

1. **Wait for GitHub Actions** - Don't proceed to package manager updates until the GitHub release is complete with all assets
2. **Test AUR locally** - `makepkg -si` is critical to catch any issues before publishing
3. **npm requires login** - Run `npm login` before the publish script
4. **All or nothing** - All package managers must work, or we roll back
5. **SHA256 verification** - The update scripts fetch SHA256s from GitHub, so the release must be complete first

---

See [PRODUCTION_RELEASE_GUIDE.md](./PRODUCTION_RELEASE_GUIDE.md) for detailed instructions and troubleshooting.
