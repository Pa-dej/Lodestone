<script lang="ts">
  import Toggle from "$lib/components/Toggle.svelte";
  import CustomSelect from "$lib/components/CustomSelect.svelte";
  import PropertyValueInput from "$lib/components/PropertyValueInput.svelte";
  import {
    getServerProperties,
    saveServerProperties,
    serverState,
  } from "$lib/stores/servers.svelte";
  import { saveSettings, settingsState, updateSettings } from "$lib/stores/settings.svelte";
  import { i18nState, t } from "$lib/stores/i18n.svelte";
  import type { AppSettings, ServerPropertyEntry } from "$lib/types";

  const defaultServerProperties: ServerPropertyEntry[] = [
    { key: "motd", value: "A Lodestone Minecraft Server" },
    { key: "online-mode", value: "true" },
    { key: "pvp", value: "true" },
    { key: "difficulty", value: "normal" },
    { key: "gamemode", value: "survival" },
    { key: "max-players", value: "20" },
    { key: "view-distance", value: "10" },
    { key: "simulation-distance", value: "10" },
    { key: "spawn-protection", value: "16" },
    { key: "allow-flight", value: "false" },
    { key: "allow-nether", value: "true" },
    { key: "white-list", value: "false" },
    { key: "enforce-whitelist", value: "false" },
    { key: "enable-command-block", value: "false" },
    { key: "spawn-monsters", value: "true" },
    { key: "spawn-animals", value: "true" },
    { key: "spawn-npcs", value: "true" },
    { key: "enable-status", value: "true" },
    { key: "hardcore", value: "false" },
  ];

  const gamemodeOptions = $derived([
    { value: "survival", label: i18nState.language === "ru" ? "Выживание" : "Survival" },
    { value: "creative", label: i18nState.language === "ru" ? "Креатив" : "Creative" },
    { value: "adventure", label: i18nState.language === "ru" ? "Приключение" : "Adventure" },
    { value: "spectator", label: i18nState.language === "ru" ? "Наблюдатель" : "Spectator" },
  ]);

  const difficultyOptions = $derived([
    { value: "peaceful", label: i18nState.language === "ru" ? "Мирная" : "Peaceful" },
    { value: "easy", label: i18nState.language === "ru" ? "Лёгкая" : "Easy" },
    { value: "normal", label: i18nState.language === "ru" ? "Нормальная" : "Normal" },
    { value: "hard", label: i18nState.language === "ru" ? "Сложная" : "Hard" },
  ]);

  const serverOptions = $derived(
    serverState.servers.map((server) => ({
      value: server.id,
      label: `${server.name} · ${server.core} ${server.version}`,
    })),
  );

  let selectedServerId = $state("");
  let properties = $state<ServerPropertyEntry[]>([]);
  let propertiesLoading = $state(false);
  let propertiesSaving = $state(false);
  let propertiesError = $state<string | null>(null);
  let propertiesSuccessMessage = $state<string | null>(null);
  let loadedForServer = $state<string | null>(null);

  async function onSaveSettings(): Promise<void> {
    await saveSettings();
  }

  function updateAndPersistSettings(patch: Partial<AppSettings>): void {
    updateSettings(patch);
    void saveSettings();
  }

  function findPropertyIndex(key: string): number {
    const needle = key.trim().toLowerCase();
    return properties.findIndex((entry) => entry.key.trim().toLowerCase() === needle);
  }

  function setProperty(key: string, value: string): void {
    const normalizedKey = key.trim();
    if (!normalizedKey) {
      return;
    }

    const nextValue = value.replaceAll("\n", " ").replaceAll("\r", " ").trim();
    const index = findPropertyIndex(normalizedKey);
    if (index >= 0) {
      properties = properties.map((entry, currentIndex) =>
        currentIndex === index ? { ...entry, key: normalizedKey, value: nextValue } : entry,
      );
      return;
    }
    properties = [...properties, { key: normalizedKey, value: nextValue }];
  }

  function getPropertyValue(key: string, fallback = ""): string {
    const index = findPropertyIndex(key);
    if (index < 0) {
      return fallback;
    }
    return properties[index].value;
  }

  function getPropertyNumber(key: string, fallback: number, min?: number, max?: number): number {
    const parsed = Number.parseInt(getPropertyValue(key, String(fallback)), 10);
    const safeValue = Number.isFinite(parsed) ? parsed : fallback;
    const withMin = min !== undefined ? Math.max(min, safeValue) : safeValue;
    return max !== undefined ? Math.min(max, withMin) : withMin;
  }

  function getPropertyBoolean(key: string, fallback: boolean): boolean {
    const raw = getPropertyValue(key, fallback ? "true" : "false").toLowerCase();
    return ["1", "true", "on", "yes"].includes(raw);
  }

  function removeProperty(index: number): void {
    properties = properties.filter((_, currentIndex) => currentIndex !== index);
  }

  function addProperty(): void {
    properties = [...properties, { key: "new-property", value: "" }];
  }

  function normalizeProperties(entries: ServerPropertyEntry[]): ServerPropertyEntry[] {
    const map = new Map<string, string>();
    for (const entry of entries) {
      const key = entry.key.replaceAll("=", "").trim();
      if (!key) {
        continue;
      }
      const value = entry.value.replaceAll("\n", " ").replaceAll("\r", " ").trim();
      map.set(key, value);
    }
    return [...map.entries()]
      .map(([key, value]) => ({ key, value }))
      .sort((a, b) => a.key.localeCompare(b.key));
  }

  async function reloadSelectedServerProperties(): Promise<void> {
    if (!selectedServerId) {
      properties = [];
      loadedForServer = null;
      return;
    }

    propertiesLoading = true;
    propertiesError = null;
    propertiesSuccessMessage = null;
    const loaded = await getServerProperties(selectedServerId);
    properties = loaded.length > 0 ? normalizeProperties(loaded) : [...defaultServerProperties];
    loadedForServer = selectedServerId;
    propertiesLoading = false;
  }

  async function saveSelectedServerProperties(): Promise<void> {
    if (!selectedServerId) {
      return;
    }

    propertiesSaving = true;
    propertiesError = null;
    propertiesSuccessMessage = null;
    const success = await saveServerProperties(selectedServerId, normalizeProperties(properties));
    propertiesSaving = false;

    if (success) {
      propertiesSuccessMessage = t("settings_props_saved");
    } else {
      propertiesError = serverState.error ?? t("error_title");
    }
  }

  $effect(() => {
    if (serverOptions.length === 0) {
      selectedServerId = "";
      properties = [];
      loadedForServer = null;
      return;
    }

    const exists = serverOptions.some((option) => option.value === selectedServerId);
    if (!exists) {
      selectedServerId = serverOptions[0]?.value ?? "";
    }
  });

  $effect(() => {
    if (selectedServerId && loadedForServer !== selectedServerId) {
      void reloadSelectedServerProperties();
    }
  });
</script>

<section class="settings-page">
  {#if settingsState.error}
    <div class="alert alert-danger">
      <span class="alert-icon">✕</span>
      <div class="alert-text">
        <div class="alert-title">{t("error_title")}</div>
        <div class="alert-sub">{settingsState.error}</div>
      </div>
    </div>
  {/if}

  {#if settingsState.successMessage}
    <div class="alert alert-success">
      <span class="alert-icon">✓</span>
      <div class="alert-text">
        <div class="alert-title">{t("save_success_title")}</div>
        <div class="alert-sub">{t("save_success_settings")}</div>
      </div>
    </div>
  {/if}

  {#if propertiesError}
    <div class="alert alert-danger">
      <span class="alert-icon">✕</span>
      <div class="alert-text">
        <div class="alert-title">{t("error_title")}</div>
        <div class="alert-sub">{propertiesError}</div>
      </div>
    </div>
  {/if}

  {#if propertiesSuccessMessage}
    <div class="alert alert-success">
      <span class="alert-icon">✓</span>
      <div class="alert-text">
        <div class="alert-title">{t("save_success_title")}</div>
        <div class="alert-sub">{propertiesSuccessMessage}</div>
      </div>
    </div>
  {/if}

  <div class="settings-grid">
    <section class="panel">
      <header class="panel-header">
        <h2 class="panel-title">{t("settings_application")}</h2>
      </header>
      <div class="panel-body settings-form">
        <Toggle
          label={t("settings_minimize_to_tray")}
          description={t("settings_minimize_to_tray_desc")}
          checked={settingsState.settings.minimize_to_tray}
          onToggle={(value) => updateAndPersistSettings({ minimize_to_tray: value })}
        />
        <Toggle
          label={t("settings_autostart_servers")}
          description={t("settings_autostart_servers_desc")}
          checked={settingsState.settings.autostart_servers}
          onToggle={(value) => updateAndPersistSettings({ autostart_servers: value })}
        />
        <Toggle
          label={t("settings_kill_server_processes_on_exit")}
          description={t("settings_kill_server_processes_on_exit_desc")}
          checked={settingsState.settings.kill_server_processes_on_exit}
          onToggle={(value) => updateAndPersistSettings({ kill_server_processes_on_exit: value })}
        />
      </div>
    </section>
  </div>

  <section class="panel server-properties">
    <header class="panel-header server-properties-header">
      <div>
        <h2 class="panel-title">{t("settings_server_properties")}</h2>
        <p class="panel-subtitle">{t("settings_server_properties_desc")}</p>
      </div>

      <div class="server-picker">
        <span class="field-label">{t("settings_server_profile")}</span>
        <CustomSelect
          value={selectedServerId}
          options={serverOptions}
          placeholder={t("settings_server_profile")}
          onChange={(value) => {
            selectedServerId = value;
          }}
        />
      </div>
    </header>

    {#if !selectedServerId}
      <div class="empty-properties">{t("settings_no_server_selected")}</div>
    {:else if propertiesLoading}
      <div class="empty-properties">{t("loading")}</div>
    {:else}
      <div class="panel-body server-properties-content">
        <div class="quick-grid">
          <label class="field span-2">
            <span class="field-label">MOTD</span>
            <input
              class="input"
              value={getPropertyValue("motd", "A Lodestone Minecraft Server")}
              oninput={(event) => setProperty("motd", (event.currentTarget as HTMLInputElement).value)}
            />
          </label>

          <label class="field">
            <span class="field-label">{t("field_gamemode")}</span>
            <CustomSelect
              value={getPropertyValue("gamemode", "survival")}
              options={gamemodeOptions}
              onChange={(value) => setProperty("gamemode", value)}
            />
          </label>

          <label class="field">
            <span class="field-label">{t("field_difficulty")}</span>
            <CustomSelect
              value={getPropertyValue("difficulty", "normal")}
              options={difficultyOptions}
              onChange={(value) => setProperty("difficulty", value)}
            />
          </label>

          <label class="field">
            <span class="field-label">{t("field_max_players")}</span>
            <input
              class="input"
              type="number"
              min={1}
              max={500}
              value={getPropertyNumber("max-players", 20, 1, 500)}
              oninput={(event) =>
                setProperty("max-players", (event.currentTarget as HTMLInputElement).value)}
            />
          </label>

          <label class="field">
            <span class="field-label">{t("field_port")}</span>
            <input
              class="input"
              type="number"
              min={1}
              max={65535}
              value={getPropertyNumber("server-port", 25565, 1, 65535)}
              oninput={(event) =>
                setProperty("server-port", (event.currentTarget as HTMLInputElement).value)}
            />
          </label>

          <label class="field">
            <span class="field-label">{t("field_view_distance")}</span>
            <div class="slider-wrap">
              <input
                class="ram-slider"
                type="range"
                min={2}
                max={32}
                step={1}
                value={getPropertyNumber("view-distance", 10, 2, 32)}
                oninput={(event) =>
                  setProperty("view-distance", (event.currentTarget as HTMLInputElement).value)}
              />
              <span class="tag">{getPropertyNumber("view-distance", 10, 2, 32)}</span>
            </div>
          </label>

          <label class="field">
            <span class="field-label">{t("field_simulation_distance")}</span>
            <div class="slider-wrap">
              <input
                class="ram-slider"
                type="range"
                min={2}
                max={32}
                step={1}
                value={getPropertyNumber("simulation-distance", 10, 2, 32)}
                oninput={(event) =>
                  setProperty("simulation-distance", (event.currentTarget as HTMLInputElement).value)}
              />
              <span class="tag">{getPropertyNumber("simulation-distance", 10, 2, 32)}</span>
            </div>
          </label>

          <label class="field span-2">
            <span class="field-label">{t("field_spawn_protection")}</span>
            <div class="slider-wrap">
              <input
                class="ram-slider"
                type="range"
                min={0}
                max={32}
                step={1}
                value={getPropertyNumber("spawn-protection", 16, 0, 32)}
                oninput={(event) =>
                  setProperty("spawn-protection", (event.currentTarget as HTMLInputElement).value)}
              />
              <span class="tag">{getPropertyNumber("spawn-protection", 16, 0, 32)}</span>
            </div>
          </label>

          <div class="field span-2 toggles-grid">
            <Toggle
              label={t("field_online_mode")}
              checked={getPropertyBoolean("online-mode", true)}
              onToggle={(value) => setProperty("online-mode", value ? "true" : "false")}
            />
            <Toggle
              label={t("field_pvp")}
              checked={getPropertyBoolean("pvp", true)}
              onToggle={(value) => setProperty("pvp", value ? "true" : "false")}
            />
            <Toggle
              label={t("settings_whitelist")}
              checked={getPropertyBoolean("white-list", false)}
              onToggle={(value) => setProperty("white-list", value ? "true" : "false")}
            />
            <Toggle
              label={t("settings_allow_flight")}
              checked={getPropertyBoolean("allow-flight", false)}
              onToggle={(value) => setProperty("allow-flight", value ? "true" : "false")}
            />
            <Toggle
              label={t("settings_command_blocks")}
              checked={getPropertyBoolean("enable-command-block", false)}
              onToggle={(value) => setProperty("enable-command-block", value ? "true" : "false")}
            />
          </div>
        </div>

        <div class="properties-header-row">
          <h3>{t("settings_advanced_group")}</h3>
          <button type="button" class="btn btn-ghost btn-sm" onclick={addProperty}>
            {t("settings_add_property")}
          </button>
        </div>

        <div class="properties-table">
          <div class="properties-head">
            <span>{t("settings_key")}</span>
            <span>{t("settings_value")}</span>
            <span></span>
          </div>

          <div class="properties-body">
            {#each properties as entry, index (index)}
              <div class="property-row">
                <input
                  class="input"
                  value={entry.key}
                  oninput={(event) => {
                    const value = (event.currentTarget as HTMLInputElement).value;
                    properties = properties.map((item, currentIndex) =>
                      currentIndex === index ? { ...item, key: value } : item,
                    );
                  }}
                />
                <PropertyValueInput
                  value={entry.value}
                  propertyKey={entry.key}
                  onChange={(value) => {
                    properties = properties.map((item, currentIndex) =>
                      currentIndex === index ? { ...item, value } : item,
                    );
                  }}
                />
                <button
                  type="button"
                  class="btn btn-danger btn-sm"
                  onclick={() => removeProperty(index)}
                >
                  {t("settings_remove_property")}
                </button>
              </div>
            {/each}
          </div>
        </div>

        <div class="server-properties-actions">
          <button
            type="button"
            class="btn btn-secondary"
            onclick={() => {
              void reloadSelectedServerProperties();
            }}
            disabled={propertiesLoading || propertiesSaving}
          >
            {t("settings_reload_props")}
          </button>
          <button
            type="button"
            class="btn btn-primary"
            onclick={() => {
              void saveSelectedServerProperties();
            }}
            disabled={propertiesLoading || propertiesSaving}
          >
            {propertiesSaving ? t("action_saving") : t("settings_save_props")}
          </button>
        </div>
      </div>
    {/if}
  </section>

  <div class="settings-actions">
    <button type="button" class="btn btn-primary" disabled={settingsState.saving} onclick={onSaveSettings}>
      {settingsState.saving ? t("action_saving") : t("action_save")}
    </button>
  </div>
</section>

<style>
  .settings-page {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
  }

  .settings-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .span-2 {
    grid-column: span 2;
  }

  .slider-wrap {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .ram-slider {
    width: 100%;
    accent-color: var(--accent);
  }

  .settings-actions {
    display: flex;
    justify-content: flex-end;
  }

  .server-properties {
    display: grid;
    gap: 12px;
  }

  .server-properties-header {
    align-items: flex-end;
    gap: 14px;
  }

  .panel-subtitle {
    font-size: 12px;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .server-picker {
    width: min(360px, 100%);
    display: grid;
    gap: 6px;
  }

  .empty-properties {
    color: var(--text-muted);
    font-size: 13px;
    padding: 8px 0;
  }

  .server-properties-content {
    display: grid;
    gap: 16px;
  }

  .quick-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .toggles-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 10px;
  }

  .properties-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .properties-header-row h3 {
    font-size: 13px;
    color: var(--text);
    font-weight: 500;
  }

  .properties-table {
    border: 0.5px solid var(--border);
    border-radius: var(--r-md);
    overflow: hidden;
  }

  .properties-head {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 8px;
    padding: 8px 10px;
    background: var(--surface-2);
    font-size: 11px;
    color: var(--text-hint);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .properties-body {
    max-height: 340px;
    overflow-y: auto;
    display: grid;
    gap: 8px;
    padding: 8px;
  }

  .property-row {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .server-properties-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  @media (max-width: 860px) {
    .server-properties-header {
      align-items: stretch;
      flex-direction: column;
    }

    .server-picker {
      width: 100%;
    }

    .quick-grid {
      grid-template-columns: 1fr;
    }

    .span-2 {
      grid-column: auto;
    }

    .toggles-grid {
      grid-template-columns: 1fr;
    }

    .properties-head,
    .property-row {
      grid-template-columns: 1fr;
    }
  }
</style>
