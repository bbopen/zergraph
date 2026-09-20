# Changelog

## 0.1.0 — private release candidate

- Replace the architecture scaffold with one owned property graph library.
- Add labeled structured edges, deterministic views, LWW membership/properties, and explicit node revival.
- Give each new/forked/restored graph a fresh writer identity.
- Add complete versioned snapshots with deletion state and exact floating-point round trips.
- Add independent merge-law, transport, malformed-snapshot, and lifecycle tests plus two executable examples.
- Remove placeholder network, query, storage-tier, security, recovery, and deployment APIs.
- Align private distribution metadata and disable registry publication.

This is a breaking replacement for the earlier placeholder API. No public release or migration from the experimental repositories is implied.
