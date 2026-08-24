# Interview Buddy

English | [简体中文](README.zh-CN.md)

A private Windows and macOS interview companion for screenshots, dual-source transcription, and LLM-assisted answers.

## Features

- Capture-protected, always-on-top overlay
- Protected drag-to-select region capture
- Microphone and system-audio transcription through WASAPI or ScreenCaptureKit
- Natural-pause VAD transcription with optional automatic answers
- Streaming answers with stop control and non-streaming fallback
- Safe Markdown, highlighted code, copy buttons, tables, and LaTeX
- In-memory conversation context and up to 30 navigable answers
- Persistent background, editable speaker-separated transcripts, and per-turn input
- OpenAI-compatible LLM and DashScope ASR
- Chinese (`zh-CN`) and English (`en-US`) interface and answer languages
- Movable unified storage root with safe WebView2 cache cleanup

## Use

1. Download the Windows installer/EXE or macOS DMG from [Releases](https://github.com/Oooorca/interview_buddy/releases).
2. Open **Settings**, then enter the API Base URL, API Key, and model names.
3. Add an optional persistent background, review live transcripts, then enter the current question or attach screenshots. Press `Ctrl+Shift+I` to send.

| Shortcut | Action |
| --- | --- |
| `Mod+Shift+S` | Drag to select and capture a region (`Esc` cancels) |
| `Mod+Shift+L` | Toggle continuous transcription |
| `Mod+Shift+A` | Toggle automatic answers; starts transcription when needed |
| `Mod+Shift+I` | Send |
| `Mod+Shift+C` | Clear the current input and screenshots |
| `Mod+Shift+Space` | Hide or show the application |
| `Mod+Q` | Quit |

`Mod` is `Ctrl` on Windows and `⌘` on macOS.

## Behavior

### Languages

The General settings page uses a responsive two-column layout for language, audio, storage, and cleanup controls. It controls the interface language and answer language independently: the interface can follow the operating system or use `zh-CN`/`en-US`, while answer language can follow the interface or stay fixed. Changing the interface language applies immediately, while saved settings take effect on the next launch. Default Prompts follow the answer language, but custom Prompts are preserved unchanged. Transcription languages remain independent in the General page's Audio section; English is stored as `en-US` and converted to the provider's expected code only when a request is sent.

Existing settings without language fields migrate to Chinese to preserve their previous behavior. New installations follow the system language and use the same language for answers. Windows installers use the operating-system language when it is Chinese or English, and macOS permission descriptions are localized for both languages.

### Prompts

Each Prompt setting has three explicit modes:

- **Recommended Default** follows the latest built-in Prompt after upgrades.
- **Custom** stores a non-empty user override.
- **Disabled** sends no corresponding Prompt.

Legacy empty values and known historical defaults migrate to Recommended Default. Other existing text is preserved as Custom.

### Shortcuts

Global shortcuts register independently. If another application already owns one or more combinations, Interview Buddy still starts and reports the unavailable shortcuts. The corresponding on-screen buttons remain usable.

### Transcription and answers

Transcription keeps both audio sources recording while an adaptive voice-activity detector submits each channel after a natural pause, with maximum-length and idle-buffer safeguards. Automatic answers are an independent policy layered on the same recording session: enabling them starts transcription when needed, while disabling them keeps transcription running.

The Audio section under General lets you independently enable and select the microphone and system-output devices. Your language and the other party's language can each use automatic detection or a different fixed language.

Transcripts are stored separately from the current input and can be edited, deleted, pinned into answer context, or copied into the current turn. Manual sending and automatic answers combine persistent background, recent or pinned transcripts, the current question, screenshots, and bounded in-memory Q/A history.

A successful manual send clears only the submitted current input and screenshots. Failures and input added during generation are preserved. **Clear Transcripts** affects only live transcripts, while **New Session** clears transcripts, current input, screenshots, and answer history but keeps the persistent background.

Answers stream into the response pane and are formatted only through React nodes; raw model HTML is ignored. Completed answers remain in memory for follow-up context and navigation but are not written to disk.

## Storage and security

Settings, WebView data, and other persistent app data use a unified storage root. The default is `.interview-buddy` inside the platform local app-data directory; development builds use `.interview-buddy-dev`. This keeps signed macOS bundles and protected install locations read-only.

The **Storage & Cleanup** section under General can move the data root, restore the default, show disk usage, and schedule safe cache cleanup. A small encrypted `storage-location.secure.json` bootstrap file remains in the platform config directory when a custom path is used. Upgrades copy managed data from the old identifier directory or an EXE-adjacent legacy `cache`; that legacy directory can be deleted after the encrypted settings have been successfully loaded from the new location.

The complete persisted settings document is encrypted as `settings.secure.json` with AES-256-GCM and an independently generated nonce on every save. The WebView receives only public settings and whether an API Key exists; it never receives the saved key itself. Windows protects the vault key with current-user DPAPI, while macOS stores it as a non-synchronizing Generic Password in the default login Keychain. Encrypted settings, backups, storage pointers, and vault keys are excluded from safe cache cleanup.

## macOS

Requires macOS 13+, Node.js, pnpm, Rust, and Xcode Command Line Tools. From a fresh clone, build the signed app bundle and start it with:

```bash
pnpm mac:start
```

On first use, click **Listen** or **Capture** to trigger the native prompts. Allow **Microphone** and **Screen & System Audio Recording** in System Settings → Privacy & Security, then fully quit and reopen Interview Buddy. The generated app is at `src-tauri/target/release/bundle/macos/Interview Buddy.app`.

The macOS bundle includes the audio-input entitlement required by hardened-runtime builds. Local builds use an ad-hoc signature by default; a real identity can override it through `APPLE_SIGNING_IDENTITY`.

> Local ad-hoc builds get a version-specific identity. If permissions stop working after rebuilding, run `pnpm mac:reset-permissions`, relaunch with `pnpm mac:start`, and grant both permissions again. Use an Apple Development or Developer ID identity to preserve permissions across builds.

## Develop

Requirements: Node.js, pnpm, Rust, plus Visual Studio Build Tools on Windows or Xcode Command Line Tools on macOS.

```powershell
pnpm install
pnpm desktop:dev
```

`desktop:dev` uses `com.oooorca.interview-buddy.dev`, the product name **Interview Buddy Dev**, an isolated `.interview-buddy-dev` storage root, and a separate system key. It cannot read release settings or API keys. Plain `pnpm dev` is a browser-only preview and starts with non-secret defaults.

Build the native installer (`.exe` on Windows or `.dmg` on macOS):

```powershell
pnpm desktop:build
```

API keys exist only inside the encrypted settings vault under the selected storage root. They are never returned through the settings IPC. Unsigned builds may trigger Windows SmartScreen or macOS Gatekeeper.

## License

MIT. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
