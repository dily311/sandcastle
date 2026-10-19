<script lang="ts">
	let { avatar }: { avatar: string | null } = $props();
</script>

{#if avatar}
	<div class="chat-avatar">
		{#if avatar?.startsWith("http")}
			<img src={avatar} alt="" />
		{:else}
			{avatar}
		{/if}
	</div>
{/if}

<style>
	.chat-avatar {
		width: 2.5rem;
		height: 2.5rem;
		border-radius: 100%;
        flex: 0 0 auto;
		display: inline-flex;
		justify-content: center;
		align-items: center;
		background: var(--card-bg);
		overflow: hidden;
	}
	.chat-avatar img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
</style>
