# Interview Buddy

[English](README.md) | 简体中文

一个面向 Windows 与 macOS 的私人面试伙伴，提供截图、双路语音转写和 LLM 辅助回答。

## 功能

- 启用系统捕获保护的置顶悬浮窗
- 受保护的拖拽框选截图
- 通过 WASAPI 或 ScreenCaptureKit 实现麦克风与系统音频双路转写
- 基于自然停顿的 VAD 听写与可选自动回答
- 可停止的流式回答与普通请求回退
- 安全 Markdown、代码高亮与复制、表格和 LaTeX
- 仅驻内存的会话上下文与最多 30 条可回看回答
- 持久固定背景、分角色可编辑转写与本轮输入
- OpenAI 兼容 LLM 与百炼语音识别
- 中文（`zh-CN`）和英文（`en-US`）界面及回答语言
- 可迁移的统一存储目录与 WebView2 安全缓存清理

## 使用

1. 从 [Releases](https://github.com/Oooorca/interview_buddy/releases) 下载 Windows 安装包/EXE 或 macOS DMG。
2. 打开**设置**，填写 API Base URL、API Key 和模型名称。
3. 可填写长期保留的固定背景，检查实时转写，然后输入本轮问题或加入截图；按 `Ctrl+Shift+I` 发送。

| 快捷键 | 功能 |
| --- | --- |
| `Mod+Shift+S` | 拖拽框选截图（`Esc` 取消） |
| `Mod+Shift+L` | 开启或关闭听写 |
| `Mod+Shift+A` | 开启或关闭自动回答；必要时自动开始听写 |
| `Mod+Shift+I` | 发送 |
| `Mod+Shift+C` | 清空本轮输入与截图 |
| `Mod+Shift+Space` | 隐藏或显示应用 |
| `Mod+Q` | 退出 |

`Mod` 在 Windows 上是 `Ctrl`，在 macOS 上是 `⌘`。

## 行为说明

### 语言

“设置 → 通用”采用响应式双列布局，统一容纳语言、音频、存储和清理设置。界面语言和回答语言仍可分别控制：界面可跟随系统或固定为 `zh-CN`/`en-US`；回答可跟随界面或固定为其中一种语言。界面切换立即生效，保存后下次启动会继续使用；推荐默认 Prompt 跟随回答语言，自定义 Prompt 原样保留。转写语言在通用页的“音频输入与输出”区域独立配置，其中英文在设置中统一保存为 `en-US`，仅在发送请求时转换成服务商要求的代码。


### Prompt

每项 Prompt 设置都有三种明确模式：

- **推荐默认**：升级后使用最新内置 Prompt。
- **自定义**：保存用户填写的非空覆盖内容。
- **禁用**：不发送对应 Prompt。


### 快捷键

全局快捷键会逐项独立注册。如果一个或多个组合键已被其他程序占用，Interview Buddy 仍会正常启动并提示不可用项；对应的界面按钮仍可点击使用。

### 听写与回答

听写会保持两路音频录制，并通过自适应语音活动检测在各自的自然停顿处提交转写，同时具有最长句段和空闲缓冲保护。自动回答是叠加在同一录音会话上的独立策略：开启时如有需要会自动启动听写，关闭后听写仍会继续。

“设置 → 通用 → 音频输入与输出”中可以分别启用并选择麦克风和系统输出设备，也可以将“我的语言”和“对方语言”分别设为自动检测或不同的固定语言。

转写与本轮输入分开保存，并支持编辑、删除、固定到回答上下文或加入本轮输入。手动发送和自动回答会组合固定背景、近期或固定转写、当前问题、截图及有界的内存问答历史。

手动发送成功后只清除已经提交的本轮输入和截图；失败或生成期间新增的输入会保留。**清空转写**只影响实时转写，**新会话**会清除转写、本轮输入、截图和回答历史，但保留固定背景。

回答会流式进入右栏，并且只通过 React 节点安全渲染，模型返回的原始 HTML 会被忽略。已完成回答会留在内存中用于追问与回看，但不会写入磁盘。

## 存储与安全

设置、WebView 数据及其他持久化内容使用统一的数据根目录。正式版默认保存在系统本地应用数据目录下的 `.interview-buddy`，开发版使用 `.interview-buddy-dev`，从而避免修改已签名的 macOS 应用包或受保护的安装目录。

“设置 → 通用 → 存储与清理”可以迁移数据、恢复默认目录、查看占用并安排安全缓存清理。使用自定义目录时，系统配置目录只保留一个很小的加密 `storage-location.secure.json` 引导文件。

完整持久化设置以 AES-256-GCM 加密为 `settings.secure.json`，每次保存都会重新生成 nonce。WebView 只能获取公开设置和“是否已配置 API Key”，不会取回已保存的 Key。Windows 使用当前用户 DPAPI 包装主密钥；macOS 将主密钥作为不参与 iCloud 同步的 Generic Password 存入默认登录 Keychain。安全缓存清理不会删除加密设置、备份、存储指针或主密钥。

## macOS

需要 macOS 13+、Node.js、pnpm、Rust 和 Xcode Command Line Tools。首次克隆后，用下面的命令构建带签名的应用并启动：

```bash
pnpm mac:start
```

首次使用时，点击**听写**或**截图**触发系统提示，在“系统设置 → 隐私与安全性”中允许**麦克风**和**屏幕与系统音频录制**，然后彻底退出并重新打开 Interview Buddy。生成的应用位于 `src-tauri/target/release/bundle/macos/Interview Buddy.app`。

macOS 安装包已加入 hardened runtime 所需的音频输入 entitlement。本地构建默认使用临时签名；如有正式证书，可通过 `APPLE_SIGNING_IDENTITY` 覆盖。

> macOS 临时签名只标识当前这一版程序。重新构建后若权限失效，请运行 `pnpm mac:reset-permissions`，再用 `pnpm mac:start` 启动并重新授权。若要让权限跨构建保持有效，请使用 Apple Development 或 Developer ID 签名。

## 开发

需要 Node.js、pnpm、Rust；Windows 还需要 Visual Studio Build Tools，macOS 还需要 Xcode Command Line Tools。

```powershell
pnpm install
pnpm desktop:dev
```

`desktop:dev` 使用独立标识 `com.oooorca.interview-buddy.dev`、产品名 **Interview Buddy Dev**、独立的 `.interview-buddy-dev` 存储根目录和系统密钥，不能读取正式版设置或 API Key。普通 `pnpm dev` 仅用于浏览器预览，并以不含秘密的默认设置启动。

构建当前平台的原生安装包（Windows 为 `.exe`，macOS 为 `.dmg`）：

```powershell
pnpm desktop:build
```

API Key 只存在于所选存储根目录的加密设置保险库中，设置 IPC 不会将其回传。未签名版本可能触发 Windows SmartScreen 或 macOS Gatekeeper 提示。

## 许可证

MIT，详见 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。
