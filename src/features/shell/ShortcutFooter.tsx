export function ShortcutFooter({ isMac }: { isMac: boolean }) {
  return <footer className="shortcut-hint">
    <span>⇧S 区域截图</span><span>⇧L 听写</span><span>⇧A 自动答</span>
    <span>⇧I 发送</span><span>⇧C 清空</span><span>{isMac ? "⌘Q" : "Ctrl+Q"} 退出</span>
  </footer>;
}
