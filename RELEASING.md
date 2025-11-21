# Releasing Paykit

This document describes the process for creating a new release of Paykit.

## Pre-Release Checklist

Before creating a release, ensure:

- [ ] All tests pass: `cargo test --all-features`
- [ ] Code is formatted: `cargo fmt --all -- --check`
- [ ] No clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Documentation builds: `cargo doc --no-deps --all-features`
- [ ] CHANGELOG.md is updated with release notes
- [ ] Version numbers are bumped in all Cargo.toml files
- [ ] Security audit passes: `cargo audit`
- [ ] All blocking issues are resolved

## Version Numbering

Paykit follows [Semantic Versioning](https://semver.org/):

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality, backwards compatible
- **PATCH**: Backwards compatible bug fixes

## Release Process

### 1. Update Version Numbers

Update version in all `Cargo.toml` files:

```bash
# Example: Updating to v0.4.0
sed -i '' 's/version = "0.3.0"/version = "0.4.0"/g' */Cargo.toml
sed -i '' 's/version = "0.3.0"/version = "0.4.0"/g' Cargo.toml
```

### 2. Update CHANGELOG.md

Add a new section for the release:

```markdown
## [0.4.0] - 2025-11-21

### Added
- Feature X
- Feature Y

### Changed
- Improvement A
- Improvement B

### Fixed
- Bug fix 1
- Bug fix 2
```

### 3. Commit Changes

```bash
git add -A
git commit -m "Bump version to v0.4.0"
```

### 4. Create Git Tag

```bash
git tag -a v0.4.0 -m "Paykit v0.4.0"
```

### 5. Run Final Verification

```bash
./audit-paykit.sh
```

### 6. Push Changes

```bash
git push origin main
git push origin v0.4.0
```

### 7. Publish to crates.io (if ready)

```bash
cd paykit-lib && cargo publish
cd ../paykit-interactive && cargo publish
cd ../paykit-subscriptions && cargo publish
```

## Post-Release

- [ ] Create GitHub release with changelog
- [ ] Announce on social media/forums
- [ ] Update documentation website (if applicable)
- [ ] Close milestone in issue tracker

## Hotfix Process

For urgent fixes:

1. Create hotfix branch from latest release tag
2. Apply fix
3. Follow release process with PATCH version bump
4. Merge hotfix back to main

## Emergency Rollback

If a critical issue is discovered:

```bash
git revert <commit-hash>
git tag -a v0.4.1 -m "Emergency rollback"
git push origin main v0.4.1
```

## Release Cadence

- **Major releases**: As needed for breaking changes
- **Minor releases**: Monthly (feature additions)
- **Patch releases**: As needed (bug fixes)
- **Security releases**: Immediately upon discovery

## Contact

For release-related questions, contact the maintainers.

