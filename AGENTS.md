# Interview Buddy agent instructions

Before changing application source, architecture, module boundaries, or platform-specific behavior, read [`.agent/ARCHITECTURE.md`](.agent/ARCHITECTURE.md) completely and follow it as the repository's source-layout and code-organization contract.

Key requirements:

- Inspect the real source tree and Git status before editing; do not infer structure from an old plan.
- Preserve existing behavior, IPC command names, persisted-setting compatibility, security migration behavior, shortcuts, and bilingual UI unless the task explicitly changes them.
- Introduce a `platform/` boundary based on the decision rules in `.agent/ARCHITECTURE.md`, not from a hard-coded module list and not merely for directory symmetry.
- Keep Rust `lib.rs` and React `App.tsx` as composition roots. New feature logic belongs in its domain module.
- Keep platform detection out of React feature props. Use `src/platform/`.
- Add or move tests with the code they cover, and run the validation matrix documented in `.agent/ARCHITECTURE.md`.
- Do not edit generated output under `dist/`, `node_modules/`, `src-tauri/target/`, or `src-tauri/gen/`.
