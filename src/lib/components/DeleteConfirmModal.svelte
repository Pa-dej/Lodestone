<script lang="ts">
  import { t } from "$lib/stores/i18n.svelte";

  interface Props {
    open: boolean;
    title: string;
    message: string;
    onCancel: () => void;
    onConfirm: () => void;
    disabled?: boolean;
  }

  let { open, title, message, onCancel, onConfirm, disabled = false }: Props = $props();
</script>

{#if open}
  <div class="modal-overlay" role="presentation">
    <div class="modal panel" role="dialog" aria-modal="true" aria-label={title}>
      <header class="modal-header">
        <h2 class="panel-title">{title}</h2>
        <button type="button" class="btn-icon" onclick={onCancel} disabled={disabled}>
          ✕
        </button>
      </header>

      <p class="modal-message">{message}</p>

      <footer class="modal-footer">
        <button type="button" class="btn btn-ghost" onclick={onCancel} disabled={disabled}>
          {t("action_cancel")}
        </button>
        <button type="button" class="btn btn-danger" onclick={onConfirm} disabled={disabled}>
          {t("action_delete")}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: var(--overlay-bg);
    display: grid;
    place-items: center;
    z-index: 260;
    padding: 16px;
  }

  .modal {
    width: min(420px, 100%);
    display: grid;
    gap: 14px;
    padding: 16px;
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 10px;
  }

  .modal-message {
    color: var(--text-muted);
    font-size: 13px;
    line-height: 1.45;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
