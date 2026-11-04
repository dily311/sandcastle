<script lang="ts">
	import { gameStore } from "$/lib/game/gameStore.svelte";

	const standings = $derived(Object.entries(gameStore.stage?.standings ?? {}));
</script>

<div class="game-stage" aria-hidden="true">
	{#if gameStore.bg}
		<img class="background" src={gameStore.bg} alt="" />
	{:else}
		<div class="background-placeholder"></div>
	{/if}
	<div class="stage-shade"></div>
	<div class="characters">
		{#each standings as [characterId, standing] (characterId)}
			{@const character = gameStore.view?.characters.find((item) => item.id === characterId)}
			<img
				class="character-sprite"
				src={gameStore.getNpcAssets(characterId)}
				alt={character ? `${character.name} (${standing.emotion})` : standing.emotion}
				loading="eager"
			/>
		{/each}
	</div>
</div>

<style>
	.game-stage { position: absolute; inset: 0; overflow: hidden; background: #27242a; }
	.background, .background-placeholder { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
	.background-placeholder { background: radial-gradient(ellipse at 50% 35%, #66556a, #302b35 68%, #211f22); }
	.stage-shade { position: absolute; inset: 0; background: linear-gradient(180deg, rgba(8,7,9,.22), transparent 28%, transparent 52%, rgba(8,7,9,.3)); }
	.characters { position: absolute; inset: 0; display: flex; align-items: flex-end; justify-content: center; gap: clamp(0px, 4vw, 70px); padding: 0 8% 0; pointer-events: none; }
	.character-sprite { display: block; max-width: min(42%, 440px); max-height: 91%; width: auto; height: auto; object-fit: contain; object-position: bottom; filter: drop-shadow(0 12px 24px rgba(0,0,0,.22)); }
	@media (max-width: 620px) { .characters { gap: 0; padding: 0 2%; } .character-sprite { max-width: 52%; max-height: 76%; } }
</style>
