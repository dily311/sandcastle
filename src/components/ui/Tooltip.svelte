<script lang="ts">
	import { tooltipStore } from "$/lib/stores/tooltip.svelte";
</script>

{#if tooltipStore.isOpen}
    <div 
        class="tooltip"
        style="top: {tooltipStore.pos.y}px; left: {tooltipStore.pos.x}px;"
        bind:this={tooltipStore.tooltipEl}
    >
        <span>{tooltipStore.label}</span>
    </div>
{/if}

<style>
    .tooltip {
        background: #000;
        color: white;
        padding: 0.2rem 0.4rem;
        border-radius: 0.4rem;
        white-space: nowrap;
        position: absolute;
        top: 0;
        left: 0;
        z-index: 100;
        font-size: 12px;
    }
</style>