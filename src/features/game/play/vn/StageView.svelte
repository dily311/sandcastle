<script lang="ts">
	import { assetUrl } from "$features/game/runtime/gameStore.svelte";
	import { gameStore } from "$features/game/runtime/gameStore.svelte";
	import { runtimeStore } from "$features/game/runtime/runtimeStore.svelte";
	import type { AssetRef } from "$features/game/types";
	import StageEffects from "./StageEffects.svelte";
	import { stageFx } from "./stageFx.svelte";
	type DisplayStanding = { emotion: string; asset: AssetRef };
	type Props = {
		onAdvance: () => void;
		standingOverrides: Record<string, DisplayStanding | null>;
		focusedCharacterId: string | null;
	};
	let { onAdvance, standingOverrides, focusedCharacterId }: Props = $props();
	// 화면 떨림 (5-E-22): 효과가 올 때마다(키) 무대를 잠깐 흔든다. 움직임 줄이기 설정이면 하지 않음
	let stageEl: HTMLDivElement | undefined = $state();
	$effect(() => {
		const shot = stageFx.shot.shake;
		if (!shot || !stageEl || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
		const steps = [0, -8, 7, -5, 4, 0].map((x, i) => ({ transform: `translate(${x}px, ${[0, 3, -4, 2, -2, 0][i]}px)` }));
		stageEl.animate(steps, { duration: shot.ms, easing: "ease-in-out" });
	});
	const standings = $derived.by(() => {
		const inScene = Boolean(runtimeStore.currentSceneId);
		const presentNpcIds = new Set(runtimeStore?.presentNpcs ?? []);
		const localStandings: Record<string, DisplayStanding | null> = {};
		if (!inScene) {
			for (const characterId of presentNpcIds) {
				const asset = gameStore.view?.assets.characters.find(
					(candidate) => candidate.characterId === characterId && candidate.usage === "standing" && candidate.emotion.toLowerCase() === "normal",
				) ?? gameStore.view?.assets.characters.find(
					(candidate) => candidate.characterId === characterId && candidate.usage === "standing",
				);
				if (asset) localStandings[characterId] = { emotion: asset.emotion, asset };
			}
		}
		const merged: Record<string, DisplayStanding | null> = {
			...localStandings,
			...(runtimeStore.stage?.standings ?? {}),
			...standingOverrides,
		};
		return Object.entries(merged).filter(
			(entry): entry is [string, DisplayStanding] =>
				entry[1] !== null && (inScene || presentNpcIds.has(entry[0])),
		);
	});
</script>

<div class="game-stage" bind:this={stageEl}>
	{#if runtimeStore.currentBg}
		<button class="background-hitbox" onclick={onAdvance} aria-label="다음 대사">
			<img class="background" src={assetUrl(runtimeStore.currentBg.path)} alt="" />
		</button>
	{:else}
		<button class="background-hitbox background-placeholder" onclick={onAdvance} aria-label="다음 대사"></button>
	{/if}
	<div class="stage-shade"></div>
	<div class="characters">
		{#each standings as [characterId, standing] (characterId)}
			{@const character = gameStore.view?.characters.find((item) => item.id === characterId)}
			{@const motion = stageFx.motions[characterId]}
			{#key motion?.key ?? 0}
				<img
					class="character-sprite {motion ? `m-${motion.name}` : ''}"
					class:unfocused={focusedCharacterId && focusedCharacterId !== characterId}
					src={assetUrl(standing.asset.path)}
					alt={character ? `${character.name} (${standing.emotion})` : standing.emotion}
					loading="eager"
				/>
			{/key}
		{/each}
	</div>
	<StageEffects />
</div>

<style>
	.game-stage {
		position: absolute;
		inset: 0;
		z-index: 0;
		overflow: hidden;
		background: var(--game-stage-bg);
	}
	.background-hitbox {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		padding: 0;
		border: 0;
		appearance: none;
		background: transparent;
		cursor: pointer;
	}
	.background {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	/* 캐릭터 모션 (5-E-23) */
	.m-jump {
		animation: m-jump 0.5s ease-out;
	}
	.m-bounce {
		animation: m-jump 0.35s ease-out 2;
	}
	.m-shake {
		animation: m-shake 0.45s ease-in-out;
	}
	.m-nod {
		animation: m-nod 0.6s ease-in-out;
	}
	.m-sway {
		animation: m-sway 1.4s ease-in-out;
		transform-origin: bottom center;
	}
	.m-shrink {
		animation: m-shrink 0.6s ease-in-out;
		transform-origin: bottom center;
	}
	.m-slide_in {
		animation: m-slide-in 0.5s ease-out;
	}
	.m-slide_out {
		animation: m-slide-out 0.5s ease-in forwards;
	}
	@keyframes m-jump {
		0%,
		100% {
			transform: translateY(0);
		}
		40% {
			transform: translateY(-9%);
		}
	}
	@keyframes m-shake {
		0%,
		100% {
			transform: translateX(0);
		}
		25% {
			transform: translateX(-3%);
		}
		50% {
			transform: translateX(3%);
		}
		75% {
			transform: translateX(-2%);
		}
	}
	@keyframes m-nod {
		0%,
		60%,
		100% {
			transform: translateY(0);
		}
		30% {
			transform: translateY(2.5%);
		}
		80% {
			transform: translateY(1.5%);
		}
	}
	@keyframes m-sway {
		0%,
		100% {
			transform: rotate(0);
		}
		25% {
			transform: rotate(-2.5deg);
		}
		75% {
			transform: rotate(2.5deg);
		}
	}
	@keyframes m-shrink {
		0%,
		100% {
			transform: scale(1);
		}
		40% {
			transform: scale(0.92);
		}
	}
	@keyframes m-slide-in {
		from {
			transform: translateX(40%);
			opacity: 0;
		}
		to {
			transform: translateX(0);
			opacity: 1;
		}
	}
	@keyframes m-slide-out {
		to {
			transform: translateX(-40%);
			opacity: 0;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.character-sprite {
			animation: none !important;
		}
	}
	.background-placeholder {
		background: var(--game-stage-placeholder);
	}
	.stage-shade {
		position: absolute;
		inset: 0;
		background: var(--game-stage-shade);
		pointer-events: none;
	}
	.characters {
		position: absolute;
		inset: 0;
		z-index: var(--z-above);
		display: flex;
		align-items: flex-end;
		justify-content: center;
		gap: clamp(0px, 4vw, 70px);
		padding: 0 8% 0;
		pointer-events: none;
	}
	.character-sprite {
		display: block;
		max-width: min(42%, 440px);
		max-height: 91%;
		width: auto;
		height: auto;
		object-fit: contain;
		object-position: bottom;
		filter: var(--game-character-shadow);
		transition:
			opacity 0.25s ease,
			filter 0.25s ease;
	}
	.character-sprite.unfocused {
		opacity: 0.36;
		filter: var(--game-character-dim-shadow);
	}
	@media (max-width: 620px) {
		/* 대사창이 화면 아래를 덮으므로 캐릭터는 그 위에서 시작 (--dock-h는 VisualNovelTemplatePlay가 잼) */
		.characters {
			bottom: calc(var(--dock-h, 0px) + 10px);
			gap: 0;
			padding: 0 2%;
		}
		.character-sprite {
			max-width: 52%;
			max-height: 100%;
		}
	}
</style>
