import { useTranslation } from "react-i18next";

export function ShortcutFooter({ isMac }: { isMac: boolean }) {
  const { t } = useTranslation();
  return <footer className="shortcut-hint">
    <span>⇧S {t("footer.capture")}</span><span>⇧L {t("footer.listen")}</span><span>⇧A {t("footer.autoAnswer")}</span>
    <span>⇧I {t("footer.send")}</span><span>⇧C {t("footer.clear")}</span><span>{isMac ? "⌘Q" : "Ctrl+Q"} {t("footer.quit")}</span>
  </footer>;
}
