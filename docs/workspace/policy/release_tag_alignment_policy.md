# Release Tag Alignment Policy

## Purpose

Prevent version/tag ambiguity between Git tags and GitHub release metadata.

## Current Decision

- `v0.3.13` is treated as a metadata alias state until a distinct release commit is cut.
- Until divergence exists, release notes must explicitly state when two tags point to the same commit.

## Required Controls

1. Before publish, compare intended release tag commit SHA to latest published release SHA.
2. If the SHAs match but tags differ:
- Mark the new tag as alias/metadata-only in release notes.
- Do not represent it as a new binary/runtime release.
3. If the SHAs differ:
- Publish as a normal release with full proof-pack linkage.

## Audit Trail

Release notes must include:

1. `release_tag`
2. `release_commit_sha`
3. `latest_published_release_tag`
4. `latest_published_release_sha`
5. `alignment_decision` (`distinct_release` or `metadata_alias`)

