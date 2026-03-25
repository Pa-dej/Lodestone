<script lang="ts">
  interface Props {
    checked: boolean;
    disabled?: boolean;
    label?: string;
    description?: string;
    onToggle?: (value: boolean) => void;
  }

  let {
    checked,
    disabled = false,
    label = "",
    description = "",
    onToggle = () => {},
  }: Props = $props();

  function toggleValue(): void {
    if (disabled) {
      return;
    }
    onToggle(!checked);
  }
</script>

<div class="toggle-row" class:disabled>
  {#if label}
    <div class="toggle-info">
      <span class="toggle-name">{label}</span>
      {#if description}
        <span class="toggle-desc">{description}</span>
      {/if}
    </div>
  {/if}
  <button
    type="button"
    class="toggle"
    class:on={checked}
    aria-pressed={checked}
    aria-label={label || "Toggle"}
    onclick={toggleValue}
    disabled={disabled}
  ></button>
</div>

<style>
  .disabled {
    opacity: 0.6;
    pointer-events: none;
  }

  .toggle {
    appearance: none;
  }
</style>
