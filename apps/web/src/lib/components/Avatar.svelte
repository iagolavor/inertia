<script lang="ts">
  import { identiconDataUrl } from '$lib/identicon';

  interface Props {
    seed: string;
    alt: string;
    src?: string | null;
    size?: number;
    class?: string;
  }

  let { seed, alt, src = null, size = 40, class: className = '' }: Props = $props();

  const fallbackSrc = $derived(identiconDataUrl(seed, size * 2));
</script>

<img
  class="avatar {className}"
  src={src || fallbackSrc}
  {alt}
  width={size}
  height={size}
  style:width="{size}px"
  style:height="{size}px"
/>

<style>
  .avatar {
    display: block;
    border-radius: 50%;
    object-fit: cover;
    flex-shrink: 0;
    border: 1px solid var(--border);
    background: var(--surface);
  }
</style>
