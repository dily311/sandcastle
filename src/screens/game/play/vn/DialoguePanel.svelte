<script lang="ts">
	import { ArrowRight } from "@lucide/svelte";
	import type { Choice, StageLine } from "$/lib/type/game";
	import { isNarration } from "./playUtils";

	type Props = {
		show: boolean;
		line: StageLine | null;
		speakerName: string | null;
		sceneDescription: string | null;
		choices: Choice[];
		lineIndex: number;
		lineCount: number;
		isReadingScript: boolean;
		isLoading: boolean;
		onStageClick: () => void;
		onPrevious: () => void;
		onAdvance: () => void;
		onChoose: (choiceId: string) => void;
	};
	let { show, line, speakerName, sceneDescription, choices, lineIndex, lineCount, isReadingScript, isLoading, onStageClick, onPrevious, onAdvance, onChoose }: Props = $props();
</script>

{#if show}
	<section class="dialogue-panel" aria-live="polite">
		<button class="line-content" type="button" onclick={onStageClick} disabled={!isReadingScript}>
			{#if !isNarration(line) && speakerName}<span class="speaker">{speakerName}</span>{/if}
			<span class="dialogue-text" class:narration={isNarration(line)}>{line ? line.text || "…" : sceneDescription ?? "장면을 시작합니다."}</span>
			{#if lineCount > 0}<span class="line-progress">{lineIndex + 1} / {lineCount}</span>{/if}
		</button>
		{#if choices.length && !isReadingScript}
			<div class="choice-list">
				{#each choices as choice (choice.id)}
					<button class="choice-button" disabled={isLoading} onclick={() => onChoose(choice.id)}>{choice.text}</button>
				{/each}
			</div>
		{:else if isLoading}<div class="loading-label">장면을 불러오는 중…</div>{/if}
		{#if lineCount > 0}
			<div class="line-navigation">
				<button class="previous-button" onclick={onPrevious} disabled={isLoading || lineIndex === 0}>이전</button>
				{#if isReadingScript}<button class="next-button" onclick={onAdvance} disabled={isLoading}>다음 <ArrowRight size={16} /></button>{/if}
			</div>
		{/if}
		{#if lineCount === 0 && !isLoading}<p class="script-status">{choices.length ? "이 장면은 대사 대신 선택지로 진행됩니다." : "현재 씬에서 복원할 StageLine 대본을 찾지 못했습니다."}</p>{/if}
	</section>
{/if}

<style>
	.dialogue-panel { position: absolute; z-index: 100; left: 50%; bottom: 26px; width: min(880px, calc(100% - 40px)); padding: 20px 24px 18px; transform: translateX(-50%); border: 1px solid var(--game-border); border-radius: 16px; color: var(--game-text-main); background: var(--game-dialogue-bg); box-shadow: var(--game-shadow-dialogue); backdrop-filter: blur(14px); pointer-events: auto; }
	.line-content { display: block; width: 100%; padding: 0; border: 0; color: inherit; background: transparent; text-align: left; cursor: pointer; }
	.line-content:disabled { cursor: default; opacity: 1; }
	.speaker { display: block; margin-bottom: 9px; color: var(--game-accent); font-size: 14px; font-weight: 700; }
	.dialogue-text { display: block; min-height: 48px; margin: 0; color: var(--game-text-primary); font-size: 17px; line-height: 1.75; white-space: pre-wrap; }
	.dialogue-text.narration { color: var(--game-text-secondary); font-size: 15px; }
	.line-progress { display: block; margin-top: 8px; color: var(--game-text-muted); font-size: 11px; text-align: right; }
	.choice-list { display: grid; gap: 9px; margin-top: 17px; }
	.choice-button { width: 100%; padding: 12px 15px; border: 1px solid var(--game-accent-border); border-radius: var(--radius-lg); color: var(--game-text-primary); background: var(--game-accent-soft); text-align: left; cursor: pointer; transition: background .15s, transform .15s; }
	.choice-button:hover:not(:disabled) { background: var(--game-accent-hover); transform: translateY(-1px); }
	.choice-button:disabled { opacity: .55; cursor: wait; }
	.line-navigation { display: flex; justify-content: space-between; gap: 8px; margin-top: 12px; }
	.next-button, .previous-button { display: flex; align-items: center; gap: 8px; padding: 8px 12px; border: 0; border-radius: var(--radius-md); color: var(--game-accent-strong); background: var(--game-border-subtle); cursor: pointer; }
	.next-button { margin-left: auto; }
	.previous-button { color: var(--game-text-secondary); }
	.previous-button:disabled { opacity: .4; cursor: default; }
	.loading-label, .script-status { color: var(--game-text-muted); font-size: 13px; }
	.script-status { margin-top: 12px; font-size: 12px; line-height: 1.5; }
	@media (max-width: 620px) { .dialogue-panel { bottom: 12px; width: calc(100% - 20px); padding: 16px; } .dialogue-text { font-size: 15px; } }
</style>
