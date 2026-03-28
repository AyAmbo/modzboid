import { invoke } from "@tauri-apps/api/core";
import { useState, useCallback } from "react";
import type { AppError } from "../types/error";

export function useTauriCommand<T>(command: string) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<AppError | null>(null);
  const [loading, setLoading] = useState(false);

  const execute = useCallback(async (args?: Record<string, unknown>) => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<T>(command, args);
      setData(result);
      return result;
    } catch (e) {
      const appError = e as AppError;
      setError(appError);
      throw appError;
    } finally {
      setLoading(false);
    }
  }, [command]);

  return { data, error, loading, execute };
}
