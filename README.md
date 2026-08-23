# Interview Buddy

A private Windows and macOS interview companion for screenshots, dual-source transcription, and LLM-assisted answers.

一个面向 Windows 与 macOS 的私人面试伙伴：截图、双路语音转写与 LLM 回答。

## Features / 功能

- Capture-protected, always-on-top overlay / 屏幕共享不可见的置顶悬浮窗
- Full-screen and coordinate-based region capture / 全屏与坐标矩形截图
- Microphone + system audio transcription (WASAPI / ScreenCaptureKit) / 麦克风与系统音频双路转写
- Manual and continuous listening modes / 手动与自动监听
- OpenAI-compatible LLM and DashScope ASR / OpenAI 兼容 LLM 与百炼语音识别

## Use / 使用

1. Download the Windows installer/EXE or macOS DMG from [Releases](https://github.com/Oooorca/interview_buddy/releases).
2. Open **Settings**, then enter the API Base URL, API Key, and model names.
3. Add screenshots, text, or transcripts to the left pane. Press `Ctrl+Shift+H` to send.

1. 从 [Releases](https://github.com/Oooorca/interview_buddy/releases) 下载 Windows 安装包/EXE 或 macOS DMG。
2. 打开**设置**，填写 API Base URL、API Key 和模型名称。
3. 将截图、文字或转写内容加入左栏，按 `Ctrl+Shift+H` 发送。

| Shortcut / 快捷键 | Action / 功能 |
| --- | --- |
| `Mod+Shift+S` | Full-screen capture / 全屏截图 |
| `Mod+Shift+1`, `Mod+Shift+2` | Mark region and capture / 标记矩形并截图 |
| `Mod+Shift+,`, `Mod+Shift+.` | Start and stop listening / 开始与结束监听 |
| `Mod+Shift+L`, `Mod+Shift+K` | Start and stop auto mode / 开启与关闭自动模式 |
| `Mod+Shift+H` | Send / 发送 |
| `Mod+Shift+X` | Clear / 清空 |
| `Mod+Shift+Space` | Hide or show / 隐藏或显示 |
| `Mod+Q` | Quit / 退出 |

`Mod` is `Ctrl` on Windows and `⌘` on macOS. / `Mod` 在 Windows 上是 `Ctrl`，在 macOS 上是 `⌘`。

### macOS

Requires macOS 13+. On first use, allow **Microphone** and **Screen & System Audio Recording** in System Settings → Privacy & Security, then restart the app. System-audio capture is an initial ScreenCaptureKit implementation and still needs hardware testing.

需要 macOS 13+。首次使用时，请在“系统设置 → 隐私与安全性”中允许**麦克风**和**屏幕与系统音频录制**，然后重启应用。系统音频目前是 ScreenCaptureKit 初版实现，仍需在真机上继续完善。

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

API keys are stored only in the local app configuration directory and are ignored by Git. Unsigned builds may trigger Windows SmartScreen or macOS Gatekeeper.

API Key 仅保存在本机应用配置目录中，不进入 Git。未签名版本可能触发 Windows SmartScreen 或 macOS Gatekeeper 提示。

## License

MIT. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
