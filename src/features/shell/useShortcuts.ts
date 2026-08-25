import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";

export type ShortcutActions = {
  capture: () => void;
  clear: () => void;
  toggleListening: () => void;
  toggleAutoAnswer: () => void;
  send: () => void;
};

export function useShortcuts(actions: ShortcutActions) {
  const actionsRef = useRef(actions);
  actionsRef.current = actions;

  useEffect(() => {
    let active = true;
    let dispose: (() => void) | undefined;
    void listen<string>("shortcut-action", ({ payload }) => {
      const current = actionsRef.current;
      switch (payload) {
        case "capture-region": current.capture(); break;
        case "clear": current.clear(); break;
        case "listening-toggle": current.toggleListening(); break;
        case "answer-toggle": current.toggleAutoAnswer(); break;
        case "send": current.send(); break;
      }
    }).then((unlisten) => {
      if (active) dispose = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      dispose?.();
    };
  }, []);
}
