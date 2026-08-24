# Interview Buddy

A private Windows and macOS interview companion for screenshots, dual-source transcription, and LLM-assisted answers.

一个面向 Windows 与 macOS 的私人面试伙伴：截图、双路语音转写与 LLM 回答。

## Features / 功能

- Capture-protected, always-on-top overlay / 屏幕共享不可见的置顶悬浮窗
- Protected drag-to-select region capture / 受保护拖拽框选截图
- Microphone + system audio transcription (WASAPI / ScreenCaptureKit) / 麦克风与系统音频双路转写
- Natural-pause VAD transcription with optional automatic answers / 基于自然停顿的双路 VAD 听写与可选自动回答
- Streaming answers with stop control and non-streaming fallback / 可停止的流式回答与普通请求回退
- Safe Markdown, highlighted code, copy buttons, tables, and LaTeX / 安全 Markdown、代码高亮复制、表格与 LaTeX
- In-memory conversation context and up to 30 navigable answers / 仅驻内存的会话上下文与最多 30 条可回看回答
- Persistent background, editable speaker-separated transcripts, and per-turn input / 持久固定背景、分角色可编辑转写与本轮输入
- OpenAI-compatible LLM and DashScope ASR / OpenAI 兼容 LLM 与百炼语音识别
- Portable storage root with safe WebView2 cache cleanup / 可迁移统一存储目录与 WebView2 安全缓存清理

## Use / 使用

1. Download the Windows installer/EXE or macOS DMG from [Releases](https://github.com/Oooorca/interview_buddy/releases).
2. Open **Settings**, then enter the API Base URL, API Key, and model names.
3. Add optional persistent background, review live transcripts, then enter the current question or attach screenshots. Press `Ctrl+Shift+I` to send.

1. 从 [Releases](https://github.com/Oooorca/interview_buddy/releases) 下载 Windows 安装包/EXE 或 macOS DMG。
2. 打开**设置**，填写 API Base URL、API Key 和模型名称。
3. 可填写长期保留的固定背景，检查实时转写，然后输入本轮问题或加入截图；按 `Ctrl+Shift+I` 发送。

| Shortcut / 快捷键 | Action / 功能 |
| --- | --- |
| `Mod+Shift+S` | Drag to select and capture a region (`Esc` cancels) / 拖拽框选截图（`Esc` 取消） |
| `Mod+Shift+L` | Toggle continuous transcription / 开启或关闭听写 |
| `Mod+Shift+A` | Toggle automatic answers; starts transcription when needed / 开启或关闭自动回答；必要时自动开始听写 |
| `Mod+Shift+I` | Send / 发送 |
| `Mod+Shift+C` | Clear current input and screenshots / 清空本轮输入与截图 |
| `Mod+Shift+Space` | Hide or show / 隐藏或显示 |
| `Mod+Q` | Quit / 退出 |

`Mod` is `Ctrl` on Windows and `⌘` on macOS. / `Mod` 在 Windows 上是 `Ctrl`，在 macOS 上是 `⌘`。

Each Prompt setting has three explicit modes: **Recommended Default** follows the latest built-in Prompt after upgrades, **Custom** stores a non-empty user override, and **Disabled** sends no corresponding Prompt. Legacy empty values and known historical defaults migrate to Recommended Default; other existing text is preserved as Custom.

每项 Prompt 设置都有三种明确模式：**推荐默认**会在升级后使用最新内置 Prompt，**自定义**保存非空的用户覆盖内容，**禁用**则不发送对应 Prompt。旧版空值和已知历史默认值会迁移为“推荐默认”，其他已有内容会保留为“自定义”。

Global shortcuts register independently. If another application already owns one or more combinations, Interview Buddy still starts and reports the unavailable shortcuts; the corresponding on-screen buttons remain usable.

全局快捷键会逐项独立注册。如果一个或多个组合键已被其他程序占用，Interview Buddy 仍会正常启动并提示不可用项；对应的界面按钮仍可点击使用。

Transcription keeps both audio sources recording while an adaptive voice-activity detector submits each channel after a natural pause, with maximum-length and idle-buffer safeguards. Automatic answers are an independent policy layered on the same recording session: enabling them starts transcription when needed, while disabling them keeps transcription running. The Audio settings page lets you independently enable and select the microphone and system-output devices, and set your language and the other party's language to automatic detection or different fixed languages.

听写会保持两路音频录制，并通过自适应语音活动检测在各自的自然停顿处提交转写，同时具有最长句段和空闲缓冲保护。自动回答是叠加在同一录音会话上的独立开关：开启时如有需要会自动启动听写，关闭后听写仍会继续。“设置 → 音频”中可以分别启用并选择麦克风和系统输出设备，也可以将“我的语言”和“对方语言”分别设为自动检测或不同的固定语言。

Transcripts are stored separately from the current input and can be edited, deleted, pinned into answer context, or copied into the current turn. Manual sending and automatic answers combine persistent background, recent/pinned transcripts, the current question, screenshots, and bounded in-memory Q/A history. A successful manual send clears only the submitted current input and screenshots; failures and input added during generation are preserved. **Clear Transcripts** affects only live transcripts, while **New Session** clears transcripts, current input, screenshots, and answer history but keeps the persistent background.

转写与本轮输入分开保存，并支持编辑、删除、固定到回答上下文或加入本轮输入。手动发送和自动回答会组合固定背景、近期或固定转写、当前问题、截图及有界的内存问答历史。手动发送成功后只清除已提交的本轮输入和截图；失败或生成期间新增的输入会保留。**清空转写**只影响实时转写，**新会话**会清除转写、本轮输入、截图和回答历史，但保留固定背景。

Answers stream into the response pane and are formatted only through React nodes; raw model HTML is ignored. Completed answers remain in memory for follow-up context and navigation, but are not written to disk.

回答会流式进入右栏，并且只通过 React 节点安全渲染，模型返回的原始 HTML 会被忽略。已完成回答会留在内存中用于追问与回看，但不会写入磁盘。

Settings, WebView data, and other persistent app data use a unified storage root. The default is the platform app-data directory, keeping signed macOS bundles and protected install locations read-only. The **Storage & Cleanup** page can move the data root, restore the default, show disk usage, and schedule safe cache cleanup. A small `storage-location.json` bootstrap file remains in the platform config directory when a custom path is used.

设置、WebView 数据及其他持久化内容统一保存在数据根目录中。默认使用系统应用数据目录，避免修改已签名的 macOS 应用包或受保护的安装目录。“设置 → 存储与清理”可以迁移数据、恢复默认目录、查看占用并安排安全缓存清理；使用自定义目录时，系统配置目录只保留一个很小的 `storage-location.json` 引导文件。

### macOS

Requires macOS 13+, Node.js, pnpm, Rust, and Xcode Command Line Tools. From a fresh clone, build the signed app bundle and start it with:

需要 macOS 13+、Node.js、pnpm、Rust 和 Xcode Command Line Tools。首次克隆后，用下面的命令构建带签名的应用并启动：

```bash
pnpm mac:start
```

On first use, click **Listen** or **Capture** to trigger the native prompts. Allow **Microphone** and **Screen & System Audio Recording** in System Settings → Privacy & Security, then fully quit and reopen Interview Buddy. The generated app is at `src-tauri/target/release/bundle/macos/Interview Buddy.app`.

首次使用时，点击**听**或**截图**触发系统提示，在“系统设置 → 隐私与安全性”中允许**麦克风**和**屏幕与系统音频录制**，然后彻底退出并重新打开 Interview Buddy。生成的应用位于 `src-tauri/target/release/bundle/macos/Interview Buddy.app`。

The macOS bundle includes the audio-input entitlement required by hardened-runtime builds. Local builds use an ad-hoc signature by default; a real identity can override it through `APPLE_SIGNING_IDENTITY`.

macOS 安装包已加入 hardened runtime 所需的音频输入 entitlement。本地构建默认使用临时签名；如有正式证书，可通过 `APPLE_SIGNING_IDENTITY` 覆盖。

> Local ad-hoc builds get a version-specific identity. If permissions stop working after rebuilding, run `pnpm mac:reset-permissions`, relaunch with `pnpm mac:start`, and grant both permissions again. Use an Apple Development or Developer ID identity to preserve permissions across builds.
>
> macOS 临时签名只标识当前这一版程序。重新构建后若权限失效，请运行 `pnpm mac:reset-permissions`，再用 `pnpm mac:start` 启动并重新授权。若要让权限跨构建保持有效，请使用 Apple Development 或 Developer ID 签名。

## Develop / 开发

Requirements: Node.js, pnpm, Rust, plus Visual Studio Build Tools on Windows or Xcode Command Line Tools on macOS.

需要 Node.js、pnpm、Rust；Windows 还需要 Visual Studio Build Tools，macOS 还需要 Xcode Command Line Tools。

```powershell
pnpm install
pnpm desktop:dev
```

Build the native installer (`.exe` on Windows, `.dmg` on macOS) / 构建当前平台安装包：

```powershell
pnpm desktop:build
```

API keys are stored only in `settings.json` under the selected storage root and are ignored by Git. Unsigned builds may trigger Windows SmartScreen or macOS Gatekeeper.

API Key 仅保存在所选存储根目录下的 `settings.json` 中，不进入 Git。未签名版本可能触发 Windows SmartScreen 或 macOS Gatekeeper 提示。

## License

MIT. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
