# Interview Buddy architecture and source-layout contract

This document is the decision framework for both new modules and refactoring existing modules. It describes how to choose boundaries; it is not a permanent whitelist of modules that may have a `platform/` directory.

## 1. General rules

- Organize source by product domain first: settings, capture, audio, answering, storage, window management, and similar capabilities.
- A composition root coordinates domains but does not own their implementation. Rust `lib.rs` registers Tauri infrastructure and commands; React `App.tsx` composes controllers and workspaces.
- Keep public boundaries narrow. Re-export the minimum stable API from each domain's `mod.rs` or `index.ts`.
- Put code where its reason to change lives. UI, persistence, protocol clients, native integration, and orchestration are different reasons to change.
- Do not create catch-all `utils`, `helpers`, `common`, or `misc` modules. Name shared code after the capability it implements.
- Do not split a cohesive small file only to satisfy a visual tree. Split when responsibilities, dependencies, tests, or platform implementations are independently changeable.
- Keep tests next to the implementation they verify. Platform-only tests belong in the relevant platform implementation.

## 2. When to create a Rust `platform/` boundary

Create `<domain>/platform/` when the domain has a stable shared use case but at least one implementation detail genuinely varies by operating system. One or more of the following is sufficient:

1. It calls operating-system APIs or platform-specific framework crates.
2. It has different permission, signing, sandbox, lifecycle, window, device, process, filesystem, or security semantics by OS.
3. A dependency or type can compile only for selected targets.
4. Correct behavior requires different algorithms or setup sequences on Windows, macOS, or an unsupported target.
5. `#[cfg(target_os = ...)]` would otherwise leak into shared commands, models, state, startup, or business logic.

Do not create `platform/` when:

- The code is only expected to become platform-specific someday but has no real difference today.
- The variation is a remote service or wire protocol, such as OpenAI versus DashScope. Those are provider/protocol adapters, not OS adapters.
- The difference is ordinary feature configuration, locale text, a model choice, or a one-off constant that can be represented as data.
- The directory would contain empty or identical implementations solely for symmetry.

When a platform boundary is warranted, use this reference shape:

```text
<domain>/
├─ mod.rs
├─ commands.rs       # optional Tauri IPC boundary
├─ model.rs          # optional platform-neutral data
├─ ...               # shared domain logic
└─ platform/
   ├─ mod.rs         # target selection and narrow re-export only
   ├─ windows.rs
   ├─ macos.rs
   └─ unsupported.rs
```

Additional private implementation helpers may live inside `platform/` when several targets share native behavior, for example `non_macos.rs`. Do not prefix files with `system_` inside a directory already named `platform`; `platform/windows.rs` is the canonical naming style.

### Rust platform contract

- `platform/mod.rs` is normally the only production file in the domain containing target-selection `#[cfg(target_os = ...)]` attributes.
- `windows.rs`, `macos.rs`, and `unsupported.rs` expose the same minimum interface to the domain facade.
- Shared callers depend on the domain facade, never on `platform::windows` or `platform::macos` directly.
- Keep platform-only context and native handles inside the selected implementation. Do not add OS-only fields to global application state or shared request models.
- `unsupported.rs` must fail explicitly and safely for unavailable behavior; it must not silently pretend a security, capture, or audio operation succeeded.
- Avoid reverse dependencies. A native security provider must not depend on the settings store merely to reuse a filesystem helper; put the helper with its owning operation or extract a capability-focused shared module.
- `cfg(test)` is allowed beside tested code. Target-specific tests should live in their platform file so target conditions do not leak into shared modules.

The expected dependency direction is:

```text
lib/startup or IPC command
        ↓
domain facade and shared coordinator
        ↓
selected platform contract
        ↓
native OS APIs
```

## 3. Rust source placement

The current tree is a reference implementation of the rules, not a frozen list:

```text
src-tauri/src/
├─ lib.rs                    # Tauri composition and command registration only
├─ main.rs                   # executable entry point only
├─ app_state.rs              # cross-domain, platform-neutral runtime state
├─ error.rs                  # stable IPC error envelope and codes
├─ startup/                  # bootstrap, main window, tray, shortcuts
├─ audio/                    # shared PCM plus selected native audio implementation
│  └─ platform/
├─ capture/                  # shared commands/model/crop plus native capture implementation
│  └─ platform/
├─ security/                 # shared encryption envelope plus OS key provider
│  └─ platform/
├─ storage/                  # storage policy, cleanup, migration, commands
│  └─ platform/
├─ window/                   # shared sizing/lifecycle/commands plus native window behavior
│  └─ platform/
├─ settings/                 # models, defaults, migration, encrypted store, IPC
├─ llm/                      # LLM protocol client and stream parser
└─ transcription/            # transcription provider adapters
```

Placement rules:

- Tauri command argument validation and state access belong in `<domain>/commands.rs`.
- Domain models and serialized contracts belong in `<domain>/model.rs` or an existing focused model file.
- Startup wiring belongs in `startup/`; it must call domain APIs rather than implement storage, security, window, or shortcut policy inline.
- Provider differences belong under their protocol domain, such as `transcription/openai.rs` and `transcription/dashscope.rs`.
- `lib.rs` must not accumulate feature logic, migrations, tray handlers, platform branches, or tests.
- Persisted-setting changes require migration and compatibility tests. Secrets must never be returned through public settings IPC or written to logs.

## 4. Frontend platform decisions

The frontend aligns with the backend conceptually, not by duplicating the Rust tree. React components remain shared unless the actual interaction model is fundamentally different.

Use `src/platform/` when runtime UI behavior differs by host platform, such as shortcut notation, root CSS class, permission-error interpretation, or native capability presentation:

```text
src/platform/
├─ index.ts
├─ types.ts
├─ windows.ts
├─ macos.ts
└─ browser.ts
```

Frontend rules:

- Detect the host only in `src/platform/index.ts`. Do not read `navigator.userAgent` in feature components.
- Consume `appPlatform` or another typed platform facade. Do not pass `isMac`, `isWindows`, or shortcut modifiers through component/controller prop chains.
- Keep components shared and use platform data or root CSS classes for small presentation differences.
- Split platform-specific React implementations only after there is a materially different interaction flow that cannot be expressed cleanly through the platform contract. Both implementations must satisfy the same typed feature interface.
- Keep browser/Vite preview behavior explicit through `browser.ts`; native APIs still go through `services/backend.ts`.
- User-visible text belongs in both `i18n/locales/zh-CN.json` and `i18n/locales/en-US.json`, not in platform adapters.
- Remote API providers are not frontend platforms.

## 5. Frontend source placement

```text
src/
├─ main.tsx                  # React entry point
├─ app/                      # top-level composition and cross-feature workspaces
├─ features/<domain>/        # UI and hooks owned by one product domain
├─ platform/                 # runtime host-platform contract
├─ services/backend.ts       # typed Tauri IPC facade
├─ shared/                   # stable cross-domain settings and data types
├─ i18n/                     # locale setup and zh-CN/en-US resources
├─ region/                   # region-selector window entry UI
└─ styles/                   # tokens and domain-oriented stylesheets
```

- `App.tsx` coordinates feature controllers; it must not contain platform detection, backend request details, audio algorithms, or settings persistence.
- Keep feature state and behavior in focused hooks inside that feature.
- Cross-feature coordination may live in `app/`, but reusable domain behavior stays with its owner.
- New backend IPC calls must be added to `services/backend.ts` rather than invoked ad hoc throughout components.
- Avoid duplicating types between Rust IPC snapshots and multiple frontend features; place shared TypeScript contracts in `shared/`.

## 6. Change checklist

### Runtime-thread safety

- Tauri commands run on the main thread unless they are `async` or use `#[tauri::command(async)]`.
- Every command that creates a window or WebView must be asynchronous. On Windows, synchronous
  WebView2 creation can deadlock the event loop.
- Commands that perform filesystem scans, durable writes, key-store operations, device discovery,
  recorder startup/shutdown, or other potentially blocking native work must not run as synchronous
  IPC handlers.
- AppKit objects on macOS must only be read or mutated through Tauri's main-thread dispatcher when
  the caller may be an async command or worker thread.

Before editing:

1. Inspect `git status`, the real source tree, and the relevant domain facade.
2. Identify whether the change is domain logic, protocol variation, runtime UI variation, or native OS variation.
3. Apply the `platform/` decision rules above; do not decide from module names or anticipated future work alone.

While editing:

1. Preserve Tauri command names and serialized field names unless an explicit API migration is part of the task.
2. Keep Windows, macOS, and unsupported contracts aligned when a native interface changes.
3. Update both locales for every user-visible frontend string.
4. Add tests to the owning module and preserve fail-closed security behavior.
5. Do not edit generated output or unrelated user changes.

Before handoff, run the checks proportional to the change. For architecture or platform work, the full local Windows matrix is:

```text
pnpm test
pnpm exec tsc --noEmit
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
pnpm desktop:build
git diff --check
```

Windows local success does not prove macOS runtime behavior. macOS platform changes must at minimum pass the repository's Apple Silicon and Intel `cargo check --all-targets` CI jobs before merge; permissions, audio, capture, signing, and Keychain behavior still require appropriate macOS runtime validation when available.
