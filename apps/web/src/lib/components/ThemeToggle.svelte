<script lang="ts">
  import {
    activeVariantLabel,
    paletteLabels,
    setPalette,
    setTheme,
    themeState,
    type Palette,
    type Theme
  } from '$lib/theme.svelte';

  const palettes: Palette[] = ['sandstone', 'midnight'];
  const modes: Theme[] = ['light', 'dark'];
</script>

<div class="theme-settings">
  <div class="group">
    <span class="group-label">Style</span>
    <div class="toggle" role="group" aria-label="Color palette">
      {#each palettes as palette}
        <button
          type="button"
          class:active={themeState.palette === palette}
          onclick={() => setPalette(palette)}
        >
          {paletteLabels[palette].name}
        </button>
      {/each}
    </div>
  </div>

  <div class="group">
    <span class="group-label">Mode</span>
    <div class="toggle" role="group" aria-label="Light or dark mode">
      {#each modes as mode}
        <button
          type="button"
          class:active={themeState.mode === mode}
          onclick={() => setTheme(mode)}
        >
          {mode === 'light' ? 'Light' : 'Dark'}
        </button>
      {/each}
    </div>
  </div>

  <p class="active-variant">
    Active:
    <strong>{activeVariantLabel(themeState.palette, themeState.mode)}</strong>
    ({paletteLabels[themeState.palette].name}
    {themeState.mode})
  </p>
</div>

<style>
  .theme-settings {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .group-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .toggle {
    display: inline-flex;
    align-self: flex-start;
    padding: 0.2rem;
    border-radius: 999px;
    border: 1px solid var(--border);
    background: var(--bg);
    gap: 0.15rem;
  }

  .toggle button {
    border: none;
    background: transparent;
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.35rem 0.75rem;
    border-radius: 999px;
    line-height: 1.2;
  }

  .toggle button:hover {
    color: var(--text);
  }

  .toggle button.active {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }

  .active-variant {
    margin: 0;
    font-size: 0.8125rem;
    color: var(--muted);
  }

  .active-variant strong {
    color: var(--text);
    font-weight: 600;
  }
</style>
