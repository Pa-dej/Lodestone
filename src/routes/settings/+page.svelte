<script lang="ts">
  import Toggle from "$lib/components/Toggle.svelte";
  import { saveSettings, settingsState, updateSettings } from "$lib/stores/settings.svelte";

  async function onSave(): Promise<void> {
    await saveSettings();
  }

  const minRamMb = 1024;
  const maxRamMb = 32768;
</script>

<section class="settings-page">
  {#if settingsState.error}
    <div class="alert alert-danger">
      <span class="alert-icon">✕</span>
      <div class="alert-text">
        <div class="alert-title">Ошибка</div>
        <div class="alert-sub">{settingsState.error}</div>
      </div>
    </div>
  {/if}

  {#if settingsState.successMessage}
    <div class="alert alert-success">
      <span class="alert-icon">✓</span>
      <div class="alert-text">
        <div class="alert-title">Сохранено</div>
        <div class="alert-sub">{settingsState.successMessage}</div>
      </div>
    </div>
  {/if}

  <div class="settings-grid">
    <section class="panel">
      <header class="panel-header">
        <h2 class="panel-title">Java</h2>
      </header>
      <div class="panel-body settings-form">
        <label class="field">
          <span class="field-label">Java executable path</span>
          <input
            class="input"
            value={settingsState.settings.java_path}
            on:input={(event) =>
              updateSettings({ java_path: (event.currentTarget as HTMLInputElement).value })}
          />
        </label>

        <label class="field">
          <span class="field-label">Max RAM MB</span>
          <div class="slider-wrap">
            <input
              class="ram-slider"
              type="range"
              min={minRamMb}
              max={maxRamMb}
              step={512}
              value={settingsState.settings.max_ram_mb}
              on:input={(event) =>
                updateSettings({
                  max_ram_mb:
                    Number.parseInt((event.currentTarget as HTMLInputElement).value, 10) ||
                    settingsState.settings.max_ram_mb,
                })}
            />
            <span class="tag">{settingsState.settings.max_ram_mb} MB</span>
          </div>
        </label>

        <label class="field">
          <span class="field-label">Extra JVM flags</span>
          <input
            class="input"
            placeholder="-XX:+UseG1GC ..."
            value={settingsState.settings.extra_jvm_flags}
            on:input={(event) =>
              updateSettings({ extra_jvm_flags: (event.currentTarget as HTMLInputElement).value })}
          />
        </label>
      </div>
    </section>

    <section class="panel">
      <header class="panel-header">
        <h2 class="panel-title">Application</h2>
      </header>
      <div class="panel-body settings-form">
        <Toggle
          label="Сворачивать в трей"
          description="При закрытии окна приложение остаётся в системном трее."
          checked={settingsState.settings.minimize_to_tray}
          onToggle={(value) => updateSettings({ minimize_to_tray: value })}
        />
        <Toggle
          label="Автозапуск серверов"
          description="Запускать активные серверы сразу после старта приложения."
          checked={settingsState.settings.autostart_servers}
          onToggle={(value) => updateSettings({ autostart_servers: value })}
        />
      </div>
    </section>
  </div>

  <div class="settings-actions">
    <button type="button" class="btn btn-primary" disabled={settingsState.saving} on:click={onSave}>
      {settingsState.saving ? "Сохранение..." : "Save"}
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
    grid-template-columns: repeat(2, minmax(0, 1fr));
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

  @media (max-width: 900px) {
    .settings-grid {
      grid-template-columns: 1fr;
    }
  }
</style>
