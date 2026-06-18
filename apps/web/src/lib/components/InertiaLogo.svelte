<script lang="ts">
  /** Mutual-link mark — Archivo 07 (orange squares + charcoal bridge). */
  let { size = 28 }: { size?: number } = $props();

  const height = (size * 36) / 50;
</script>

<svg
  class="inertia-logo"
  width={size}
  height={height}
  viewBox="0 0 50 36"
  aria-hidden="true"
>
  <rect
    x="4"
    y="10"
    width="16"
    height="16"
    rx="5"
    fill="none"
    stroke="var(--logo-ring)"
    stroke-width="2"
  />
  <rect
    x="30"
    y="10"
    width="16"
    height="16"
    rx="5"
    fill="none"
    stroke="var(--logo-ring)"
    stroke-width="2"
  />
  <line
    x1="20"
    y1="18"
    x2="30"
    y2="18"
    stroke="var(--logo-link)"
    stroke-width="2"
    stroke-linecap="round"
  />
</svg>

<style>
  .inertia-logo {
    display: block;
    flex-shrink: 0;
  }
</style>
