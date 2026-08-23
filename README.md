# Interview Buddy

A private Windows interview companion for screenshots, dual-source transcription, and LLM-assisted answers.

一个面向 Windows 的私人面试伙伴：截图、双路语音转写与 LLM 回答。

## Features / 功能

- Capture-protected, always-on-top overlay / 屏幕共享不可见的置顶悬浮窗
- Full-screen and coordinate-based region capture / 全屏与坐标矩形截图
- Microphone + system audio transcription / 麦克风与系统音频双路转写
- Manual and continuous listening modes / 手动与自动监听
- OpenAI-compatible LLM and DashScope ASR / OpenAI 兼容 LLM 与百炼语音识别

## Use / 使用

1. Download the installer or portable EXE from [Releases](https://github.com/Oooorca/interview_buddy/releases).
2. Open **Settings**, then enter the API Base URL, API Key, and model names.
3. Add screenshots, text, or transcripts to the left pane. Press `Ctrl+Shift+H` to send.

1. 从 [Releases](https://github.com/Oooorca/interview_buddy/releases) 下载安装包或便携 EXE。
2. 打开**设置**，填写 API Base URL、API Key 和模型名称。
3. 将截图、文字或转写内容加入左栏，按 `Ctrl+Shift+H` 发送。

| Shortcut / 快捷键 | Action / 功能 |
| --- | --- |
| `Ctrl+Shift+S` | Full-screen capture / 全屏截图 |
| `Ctrl+Shift+1`, `Ctrl+Shift+2` | Mark region and capture / 标记矩形并截图 |
| `Ctrl+Shift+,`, `Ctrl+Shift+.` | Start and stop listening / 开始与结束监听 |
| `Ctrl+Shift+L`, `Ctrl+Shift+K` | Start and stop auto mode / 开启与关闭自动模式 |
| `Ctrl+Shift+H` | Send / 发送 |
| `Ctrl+Shift+X` | Clear / 清空 |
| `Ctrl+Shift+Space` | Hide or show / 隐藏或显示 |
| `Ctrl+Q` | Quit / 退出 |

## Develop / 开发

Requirements: Node.js, pnpm, Rust, WebView2, and Visual Studio Build Tools with C++ desktop support.

需要 Node.js、pnpm、Rust、WebView2，以及包含 C++ 桌面开发组件的 Visual Studio Build Tools。

```powershell
pnpm install
pnpm desktop:dev
```

Build the Windows installer locally / 本地构建 Windows 安装包：

```powershell
pnpm desktop:build
```

API keys are stored only in the local app configuration directory and are ignored by Git. Unsigned builds may trigger a Windows SmartScreen warning.

API Key 仅保存在本机应用配置目录中，不进入 Git。未签名版本可能触发 Windows SmartScreen 提示。

## License

MIT. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
