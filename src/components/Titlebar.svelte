<script lang="ts">
	import { X } from "@lucide/svelte";
	import { getCurrentWindow } from "@tauri-apps/api/window";
	import { router } from 'svelte-spa-router'
	const appWindow = getCurrentWindow();
</script>

<div class="titlebar" data-tauri-drag-region>
	<div class="titlebar-label">
		{router.location}
	</div>
	<div class="controls">
		<button id="titlebar-minimize" title="최소화" onclick={() => appWindow.minimize()}>
			<svg
				xmlns="http://w3.org"
				viewBox="0 0 24 24"
				width="24"
				height="24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<line x1="0" y1="12" x2="24" y2="12"></line>
			</svg>
		</button>
		<button id="titlebar-maximize" title="최대화" onclick={() => appWindow.toggleMaximize()}>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				width="24"
				height="24"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
			</svg>
		</button>
		<button id="titlebar-close" title="닫기" onclick={() => appWindow.close()}>
			<X  />
		</button>
	</div>
</div>

<style>
	*[data-tauri-drag-region] {
		width: 100%;
	}
	.titlebar {
		display: flex;
		height: var(--titlebar-height);
		user-select: none;
		background: transparent;
		border-bottom: 1px solid var(--border-color);
		position: relative;
		pointer-events: none;
	}
	:global(body.is-frameless .titlebar) {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 30;
		height: var(--header-height);
		border: none;
		background: transparent;
	}
	.titlebar-label {
		display: none;
	}
	.controls {
		display: flex;
		height: var(--titlebar-height);
		position: fixed;
		top: 0;
		right: 0;
		z-index: 40;
		pointer-events: all;
	}
	.titlebar button {
		appearance: none;
		padding: 0;
		margin: 0;
		border: none;
		display: inline-flex;
		justify-content: center;
		align-items: center;
		width: 30px;
		background-color: transparent;
	}
	.titlebar button:hover {
		background: var(--color-30);
	}
	.titlebar button:last-child:hover {
		color: white;
		background: var(--color-red);
	}
	.titlebar button svg {
		width: 12px;
		height: 12px;
		color: var(--icon-color);
	}
</style>
