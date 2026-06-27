<script lang="ts">
  let { children } = $props();
</script>

<div class="welcome-shell">
  <main class="welcome-main">
    {@render children()}
  </main>
</div>

<style>
  .welcome-shell {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
    width: 100%;
    background: var(--bg);
  }

  .welcome-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    max-width: 480px;
    margin: 0 auto;
    padding: max(1.5rem, var(--safe-top)) max(1.25rem, var(--safe-right))
      max(1.5rem, var(--safe-bottom)) max(1.25rem, var(--safe-left));
  }
</style>
