# Publication review

Reviewed October 22, 2026 (America/Edmonton). Source publication and production
activation are separate decisions.

## Chosen publication path

The owner chose a separate repository, **Clawberry**, with a new root commit.
The existing RaaviClaw repository remains private with its original history.
Do not push its branches, tags or Git objects to the public repository. Copy only
reviewed source files; exclude environment files, private runtime data, logs,
archives, local tools and build/test output. Use a GitHub noreply author identity.
Required TinyClaw/ZeroClaw attribution and inherited notices remain.

Clawberry is an early-stage community project. The source release invites people
to help finish it; it does not certify production readiness or activate execution,
watches, cross-agent exchanges, external writes or services.

## Preparation

- Rewrote the README around actual local setup, current scope and contributor work.
- Added contributor starting points and removed obsolete security/test-count claims
  from the main onboarding/security documents.
- Corrected release/Docker executable paths to `tinyclaw`, retained license notices,
  pinned release builds to Rust 1.92 with the committed lockfile, and replaced the
  Compose example's unrelated upstream image with a local build and loopback port.
- Updated `rustls-webpki` to 0.103.16 and `webpki-roots` to 1.0.10 in response to
  RUSTSEC-2026-0382 and RUSTSEC-2026-0381. No advisory was suppressed.
- Current-file and all-ref Gitleaks scans found no leaks under the narrowly scoped
  exact encryption-test fixture allowance. Automated scanning cannot prove absence.
- No ignored files are tracked. Existing private Git history is not being published.

## Release evidence

The session worklog records fresh validation and its limitations. Final-candidate
checks must include the clean public root, not only the private source branch.
Cross-platform release builds and a complete production container/app deployment
remain separate acceptance work. Do not advertise them as verified.

## Before changing visibility

- Review the exact clean import, filenames, author metadata and final secret scan.
- Confirm fresh CI/security checks on the new repository's exact commit.
- Enable private vulnerability reporting and appropriate branch protection.
- Keep the existing private issues, pull requests, Actions logs/artifacts and
  attachments private; none belong in the clean import.
- Confirm publication with the owner after the candidate is reviewable.

An early-stage source repository may be public while deployment remains deferred.
Production capabilities still require the acceptance procedures in
[deployment acceptance](deployment-acceptance.md).
