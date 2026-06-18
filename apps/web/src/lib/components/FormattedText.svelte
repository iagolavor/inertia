<script lang="ts">
  import { formatBasicMarkdown } from '$lib/textFormat';

  interface Props {
    text: string;
    class?: string;
  }

  let { text, class: className = '' }: Props = $props();

  const html = $derived(formatBasicMarkdown(text));
</script>

<p class={['formatted-text', className].filter(Boolean).join(' ')}>{@html html}</p>

<style>
  .formatted-text {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .formatted-text :global(strong) {
    font-weight: 700;
  }

  .formatted-text :global(em) {
    font-style: italic;
  }

  .formatted-text :global(del) {
    text-decoration: line-through;
    opacity: 0.75;
  }

  .formatted-text :global(code) {
    font-family: ui-monospace, 'Cascadia Code', 'Segoe UI Mono', monospace;
    font-size: 0.9em;
    padding: 0.1em 0.35em;
    border-radius: 4px;
    background: color-mix(in srgb, var(--border) 40%, transparent);
  }

  .formatted-text :global(a) {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .formatted-text :global(a:hover) {
    text-decoration: none;
  }
</style>
