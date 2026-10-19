<script>
  export let count = 0;
  export let max = 99;
  export let dot = false;

  $: displayCount = count > max ? `${max}+` : count;
</script>

{#if count > 0 || dot}
  <span class="badge" class:dot class:overflow={count > max}>
    {#if !dot}
      {displayCount}
    {/if}
  </span>
{/if}

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    font-size: 0.75rem;
    font-weight: 600;
    color: #ffffff;
    background-color: #ef4444;
    border-radius: 9999px;
    line-height: 1;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition: transform 0.2s ease-in-out;
  }

  .badge.dot {
    min-width: 8px;
    width: 8px;
    height: 8px;
    padding: 0;
  }
</style>