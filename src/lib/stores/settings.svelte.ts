import { invoke } from "@tauri-apps/api/core";
import type { AppSettings } from "$lib/types";

export const defaultSettings: AppSettings = {
  java_path: "java",
  max_ram_mb: 4096,
  extra_jvm_flags: "",
  minimize_to_tray: false,
  autostart_servers: false,
};

interface SettingsStoreState {
  settings: AppSettings;
  loading: boolean;
  saving: boolean;
  error: string | null;
  successMessage: string | null;
}

export const settingsState = $state<SettingsStoreState>({
  settings: { ...defaultSettings },
  loading: false,
  saving: false,
  error: null,
  successMessage: null,
});

function setSettingsError(error: unknown): void {
  settingsState.error = error instanceof Error ? error.message : String(error);
}

export async function loadSettings(): Promise<void> {
  settingsState.loading = true;
  settingsState.error = null;
  try {
    const loaded = await invoke<AppSettings>("get_settings");
    settingsState.settings = { ...defaultSettings, ...loaded };
  } catch (error) {
    setSettingsError(error);
  } finally {
    settingsState.loading = false;
  }
}

export async function saveSettings(): Promise<boolean> {
  settingsState.saving = true;
  settingsState.error = null;
  settingsState.successMessage = null;
  try {
    await invoke("save_settings", { settings: settingsState.settings });
    settingsState.successMessage = "Настройки сохранены";
    return true;
  } catch (error) {
    setSettingsError(error);
    return false;
  } finally {
    settingsState.saving = false;
  }
}

export function updateSettings(patch: Partial<AppSettings>): void {
  settingsState.settings = { ...settingsState.settings, ...patch };
}
