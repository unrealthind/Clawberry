# RaaviClaw identity

Current delivery status — October 20, 2026: [eight-phase plan](eight-phase-plan.md). Phases 0–4 are complete for local development acceptance; Phase 5 app completion is next. Rocky on Proxmox is the development environment. Live integrations, external-write activation and production deployment remain gated/deferred. Older dated evidence below is historical, not a current branch or release-status claim.

A split-stem R: two distinct pieces, one coherent identity. A quiet nod to independently scoped assistants, not a literal claw or mascot. Graphite `#17191C` and off-white `#F5F6F7`; no gradients, glow, decorative circuits or sparkle. This is an original geometric design, not a trademark-clearance claim.

Assets live in `app/public/brand`: `logo.svg` is the horizontal lockup, `mark.svg` and `mark-light.svg` are standalone marks, and `icon.svg` is the unmasked full-bleed 1024-unit app-icon master. `app/public/favicon.svg` has a web-only rounded container. The native icon is deliberately unmasked for platform masking. Use the mark at small sizes, never the full wordmark. Keep at least the stem's width of clear space and do not stretch, recolor pieces separately or add effects.

The live app uses system typography and accessible HTML text alongside the decorative mark. The SVG lockup uses an Arial/Helvetica fallback; it is not a font-independent outlined wordmark. The design review influenced centered icon content, strong silhouette, light/dark marks and limited logo repetition. Browser/device visual acceptance remains pending.

Generate PNG exports on macOS with `swift scripts/export-brand-icons.swift`. No image service or runtime dependency is required. The script produces 16/32/192/512/1024 PNGs and a 180px Apple touch icon. Vector files remain authoritative; keep export geometry synchronized if the mark changes.

Product display names are RaaviClaw. The `tinyclaw` Cargo package, CLI executable, configuration paths, service identifiers and security-policy paths remain unchanged for compatibility. Upstream attribution, license notices, historical documentation and test sample text are not erased.
