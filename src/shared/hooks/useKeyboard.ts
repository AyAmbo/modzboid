import { useEffect } from "react";

type KeyHandler = (e: KeyboardEvent) => void;

export function useKeyboard(
  key: string,
  handler: KeyHandler,
  modifiers?: { ctrl?: boolean; shift?: boolean; alt?: boolean }
) {
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const ctrlMatch = modifiers?.ctrl ? e.ctrlKey || e.metaKey : !e.ctrlKey && !e.metaKey;
      const shiftMatch = modifiers?.shift ? e.shiftKey : !e.shiftKey;
      const altMatch = modifiers?.alt ? e.altKey : !e.altKey;
      if (
        e.key.toLowerCase() === key.toLowerCase() &&
        ctrlMatch &&
        shiftMatch &&
        altMatch
      ) {
        e.preventDefault();
        handler(e);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [key, handler, modifiers]);
}
