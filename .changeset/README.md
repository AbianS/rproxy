# Changesets

This project uses [Changesets](https://github.com/changesets/changesets) for version management.

## Adding a changeset

When you make a change that should trigger a release, run:

```bash
pnpm changeset
```

This will prompt you to:
1. Select the packages that changed
2. Choose the version bump type (major, minor, patch)
3. Write a summary of the changes

## Version types

- **patch**: Bug fixes, small changes (0.0.x)
- **minor**: New features, backwards compatible (0.x.0)
- **major**: Breaking changes (x.0.0)

## Release process

1. Create changesets for your changes
2. Open a PR to main
3. After merge, the release workflow creates a "Version Packages" PR
4. Merging that PR triggers the release and publishes Docker images
