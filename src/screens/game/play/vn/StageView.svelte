<script lang="ts">
	import type { AssetRef } from "$/lib/type/game";
	import { gameStore } from "$/lib/game/gameStore.svelte";
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type Props = {
		onAdvance: () => void;
		standingOverrides: Record<string, DisplayStanding | null>;
		focusedCharacterId: string | null;
	};
	let { onAdvance, standingOverrides, focusedCharacterId }: Props = $props();
	const standings = $derived.by(() => {
		const merged: Record<string, DisplayStanding | null> = {
			...(gameStore.stage?.standings ?? {}),
			...standingOverrides,
		};
		return Object.entries(merged).filter((entry): entry is [string, DisplayStanding] => entry[1] !== null);
	});
</script>

<div class="game-stage">
	{#if gameStore.bg}
		<button class="background-hitbox" onclick={onAdvance} aria-label="다음 대사">
			<img class="background" src={gameStore.bg} alt="" />
		</button>
	{:else}
		<button class="background-hitbox background-placeholder" onclick={onAdvance} aria-label="다음 대사"></button>
	{/if}
	<div class="stage-shade"></div>
	<div class="characters">
		{#each standings as [characterId, standing] (characterId)}
			{@const character = gameStore.view?.characters.find((item) => item.id === characterId)}
			<img
				class="character-sprite"
				class:unfocused={focusedCharacterId && focusedCharacterId !== characterId}
				src={gameStore.getAssetUrl(standing.asset)}
				alt={character ? `${character.name} (${standing.emotion})` : standing.emotion}
				loading="eager"
			/>
		{/each}
	</div>
</div>

<style>
	.game-stage { position: absolute; inset: 0; z-index: 0; overflow: hidden; background: var(--game-stage-bg); }
	.background-hitbox { position: absolute; inset: 0; width: 100%; height: 100%; padding: 0; border: 0; appearance: none; background: transparent; cursor: pointer; }
	.background { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
	.background-placeholder { background: var(--game-stage-placeholder); }
	.stage-shade { position: absolute; inset: 0; background: var(--game-stage-shade); pointer-events: none; }
	.characters { position: absolute; inset: 0; z-index: 1; display: flex; align-items: flex-end; justify-content: center; gap: clamp(0px, 4vw, 70px); padding: 0 8% 0; pointer-events: none; }
	.character-sprite { display: block; max-width: min(42%, 440px); max-height: 91%; width: auto; height: auto; object-fit: contain; object-position: bottom; filter: var(--game-character-shadow); transition: opacity .25s ease, filter .25s ease; }
	.character-sprite.unfocused { opacity: .36; filter: var(--game-character-dim-shadow); }
	@media (max-width: 620px) { .characters { gap: 0; padding: 0 2%; } .character-sprite { max-width: 52%; max-height: 76%; } }
</style>
