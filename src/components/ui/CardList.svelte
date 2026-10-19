<script lang="ts">
    let { children } = $props();
</script>

<div class="card">
    {@render children()}
</div>

<style>
	.card {
		min-height: 150px;
		padding: 15px;
		display: flex;
		gap: 13px;
		border: 1px solid var(--card-border-color);
		border-radius: 14px;
		background: var(--card-bg);
		transition:
			box-shadow 140ms ease,
			border-color 140ms ease,
			transform 140ms ease;
	}

	.card:hover {
		box-shadow: var(--shadow);
		border-color: transparent;
		transform: translateY(-1px);
	}
</style>
