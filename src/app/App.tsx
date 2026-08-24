import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAnswerController } from "../features/answer/useAnswerController";
import { useInterviewSession } from "../features/interview/useInterviewSession";
import { useAudioDevices } from "../features/listening/useAudioDevices";
import { useListeningController } from "../features/listening/useListeningController";
import { useAppSettings } from "../features/settings/useAppSettings";
import { useStorageSettings } from "../features/settings/useStorageSettings";
import { ShortcutFooter } from "../features/shell/ShortcutFooter";
import { TitleBar } from "../features/shell/TitleBar";
import { useShortcuts } from "../features/shell/useShortcuts";
import { useWindowSizing } from "../features/window/useWindowSizing";
import { errorMessage } from "../services/backend";
import type { AppStatus } from "../shared/types";
import { InterviewWorkspace } from "./InterviewWorkspace";
import { SettingsWorkspace } from "./SettingsWorkspace";

const IS_MAC = navigator.userAgent.includes("Mac");
const IS_TAURI = "__TAURI_INTERNALS__" in window;
const MODIFIER = IS_MAC ? "⌘⇧" : "Ctrl+Shift+";

function App() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<AppStatus>("ready");
  const settings = useAppSettings(IS_TAURI);
  const windowSizing = useWindowSizing({
    isTauri: IS_TAURI,
    settingsRef: settings.settingsRef,
    setSettings: settings.setSettings,
  });
  const storage = useStorageSettings({
    settingsRef: settings.settingsRef,
    setNotice: settings.setNotice,
  });
  const audioDevices = useAudioDevices(
    settings.settingsOpen && settings.settingsPage === "general",
  );
  const showErrorRef = useRef<(label: string, error: unknown) => void>(() => undefined);
  const interview = useInterviewSession({
    settingsRef: settings.settingsRef,
    setStatus,
    setNotice: settings.setNotice,
    showErrorRef,
  });
  const onAnswerIdleRef = useRef<() => void>(() => undefined);
  const answer = useAnswerController({
    settingsRef: settings.settingsRef,
    apiKeyConfigured: settings.apiKeyConfigured,
    securityIssue: settings.securityIssue,
    interview,
    setSettingsOpen: settings.setSettingsOpen,
    setStatus,
    setNotice: settings.setNotice,
    onIdleRef: onAnswerIdleRef,
  });
  const listening = useListeningController({
    isMac: IS_MAC,
    settingsRef: settings.settingsRef,
    apiKeyConfigured: settings.apiKeyConfigured,
    securityIssue: settings.securityIssue,
    interview,
    answer,
    setSettingsOpen: settings.setSettingsOpen,
    setStatus,
    setNotice: settings.setNotice,
    onAnswerIdleRef,
  });

  showErrorRef.current = (label, error) => {
    answer.setOutput(`${label}: ${errorMessage(error)}`);
    setStatus("error");
    settings.setNotice(label);
  };

  function clearTranscripts() {
    listening.clearPendingAuto();
    interview.clearTranscripts();
  }

  function startNewSession() {
    if (answer.answering) {
      settings.setNotice(t("notices.stopFirst"));
      return;
    }
    listening.resetSession();
    interview.resetSession();
    answer.reset();
    settings.setNotice(t("notices.newSession"));
  }

  useShortcuts({
    capture: () => { void interview.takeRegionScreenshot(); },
    clear: interview.clearCurrentInput,
    toggleListening: () => {
      void (listening.listening ? listening.stop() : listening.start());
    },
    toggleAutoAnswer: () => { void listening.toggleAutoAnswer(); },
    send: () => { void answer.sendCurrentTurn(); },
  });

  return <main className={`app-shell ${IS_MAC ? "platform-mac" : "platform-windows"}`}>
    <TitleBar status={status} listening={listening.listening} autoAnswer={listening.autoAnswer}
      settingsOpen={settings.settingsOpen} securityLocked={Boolean(settings.securityIssue)}
      notice={settings.notice} shortcutIssue={settings.shortcutIssue}
      modifier={MODIFIER} isMac={IS_MAC}
      onCapture={() => void interview.takeRegionScreenshot()}
      onListeningToggle={() => void (listening.listening ? listening.stop() : listening.start())}
      onAutoAnswerToggle={() => void listening.toggleAutoAnswer()}
      onSettingsToggle={() => settings.setSettingsOpen((open) => !open)} />

    {settings.settingsOpen
      ? <SettingsWorkspace controller={settings} storage={storage}
          audioDevices={audioDevices} windowSizing={windowSizing}
          showError={(label, error) => showErrorRef.current(label, error)} />
      : <InterviewWorkspace settings={settings} interview={interview} answer={answer}
          listening={listening} status={status} modifier={MODIFIER}
          onClearTranscripts={clearTranscripts} onNewSession={startNewSession} />}

    <ShortcutFooter isMac={IS_MAC} />
  </main>;
}

export default App;
