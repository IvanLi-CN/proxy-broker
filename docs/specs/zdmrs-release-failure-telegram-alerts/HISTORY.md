# History

## Lifecycle

This topic remains the active contract for repository-local release-failure
notifications.

## Compatibility

- The original wrapper delegated to
  `IvanLi-CN/github-workflows/.github/workflows/release-failure-telegram.yml@main`
  and forwarded the `SHOUTRRR_URL` secret plus separate notification metadata.
- The current wrapper delegates to Oidrune's pinned `notify.yml` contract and
  folds the same project metadata into the caller-owned summary. The release
  failure filter and manual smoke path remain separate and unchanged in
  meaning.

## Boundary Notes

The Oidrune revision was rechecked against live `main` and the latest release
before migration. Gateway endpoint and audience selection remain Oidrune-owned;
the repository intentionally does not duplicate those values.
