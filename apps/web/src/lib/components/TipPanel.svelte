<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Accessible name for the trigger control. */
    label: string;
    /** Prefer right alignment in the header end cluster. */
    align?: 'start' | 'end';
    trigger: Snippet;
    children: Snippet;
  }

  let { label, align = 'end', trigger, children }: Props = $props();

  let open = $state(false);
  let root: HTMLDivElement | null = $state(null);

  function toggle() {
    open = !open;
  }

  function close() {
    open = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && open) {
      e.stopPropagation();
      close();
    }
  }

  $effect(() => {
    if (typeof document === 'undefined' || !open) return;

    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (target && root && !root.contains(target)) close();
    };

    document.addEventListener('pointerdown', onPointerDown);
    return () => document.removeEventListener('pointerdown', onPointerDown);
  });
</script>

<div class="tip" class:align-end={align === 'end'} class:align-start={align === 'start'} bind:this={root}>
  <button
    type="button"
    class="tip-trigger"
    aria-label={label}
    aria-expanded={open}
    aria-haspopup="dialog"
    onclick={toggle}
  >
    {@render trigger()}
  </button>

  {#if open}
    <div class="tip-panel" role="dialog" aria-label={label}>
      {@render children()}
    </div>
  {/if}
</div>

<svelte:window onkeydown={onKeydown} />

<style>
  .tip {
    position: relative;
    display: inline-flex;
    flex-shrink: 0;
  }

  .tip-trigger {
    display: inline-flex;
    align-items: center;
    margin: 0;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    line-height: 1;
    cursor: pointer;
    border-radius: 8px;
  }

  .tip-trigger:focus-visible {
    outline: 2px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    outline-offset: 2px;
  }

  .tip-panel {
    position: absolute;
    top: calc(100% + 0.4rem);
    z-index: 40;
    min-width: 14.5rem;
    max-width: min(20rem, calc(100vw - 1.5rem));
    padding: 0.7rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--surface);
    box-shadow: 0 8px 24px color-mix(in srgb, #000 18%, transparent);
    color: var(--text);
    text-align: left;
  }

  .align-end .tip-panel {
    right: 0;
  }

  .align-start .tip-panel {
    left: 0;
  }
</style>
