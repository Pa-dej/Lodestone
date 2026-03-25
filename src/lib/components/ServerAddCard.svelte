<script lang="ts">
  interface Props {
    onClick: () => void;
  }

  let { onClick }: Props = $props();
</script>

<button type="button" class="add-card" onclick={onClick}>
  <span class="add-icon">+</span>
  <span class="add-label">Добавить сервер</span>
</button>

<style>
  .add-card {
    border: 1px dashed var(--border);
    border-radius: var(--r-lg);
    background: transparent;
    color: var(--text-muted);
    min-height: 180px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    cursor: pointer;
    transition: border-color var(--tr), color var(--tr), background var(--tr);
  }

  .add-card:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-bg);
  }

  .add-icon {
    width: 36px;
    height: 36px;
    border: 0.5px dashed currentColor;
    border-radius: var(--r-md);
    display: grid;
    place-items: center;
    font-size: 22px;
    line-height: 1;
  }

  .add-label {
    font-size: 13px;
    letter-spacing: 0.03em;
  }
</style>
