<script lang="ts">
	import { onMount } from "svelte";
	import { WidgetGame } from "./pixi/WidgetGame";

	let container: HTMLDivElement;
	let game: WidgetGame;
	onMount(() => {
		game = new WidgetGame();
		game.init(container);
        
		return () => {
			game.destroy();
		};
	});
</script>

<div bind:this={container} class="simeji"></div>

<style>
	.simeji {
		position: fixed;
		inset: 0;
		overflow: hidden;
		/* z-index를 다른 위젯보다 높게 설정 금지 
			혼자 독차지하게 됨
		*/
	}
	:global(canvas) {
		display: block;
		width: 100%;
		height: 100%;
	}
</style>
