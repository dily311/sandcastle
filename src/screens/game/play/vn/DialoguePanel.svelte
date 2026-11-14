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
		npcChoices: string[];
		canSendInput: boolean;
		canStopTalk: boolean;
		inputValue: string;
		lineIndex: number;
		lineCount: number;
		scriptFinished: boolean;
		isReadingScript: boolean;
		isLoading: boolean;
		onStageClick: () => void;
		onPrevious: () => void;
		onAdvance: () => void;
		onChoose: (choiceId: string) => void;
		onNpcChoice: (text: string) => void;
		onInputChange: (value: string) => void;
		onSubmitInput: () => void;
		onStopTalk: () => void;
	};
	let {
		show,
		line,
		speakerName,
		sceneDescription,
		choices,
		npcChoices,
		canSendInput,
		canStopTalk,
		inputValue,
		lineIndex,
		lineCount,
		scriptFinished,
		isReadingScript,
		isLoading,
		onStageClick,
		onPrevious,
		onAdvance,
		onChoose,
		onNpcChoice,
		onInputChange,
		onSubmitInput,
		onStopTalk,
	}: Props = $props();

	function handleInputSubmit(event: SubmitEvent) {
		event.preventDefault();
		onSubmitInput();
	}
</script>

{#if show}
	<section class="dialogue-panel" aria-live="polite">
		{#if scriptFinished}
			{#if choices.length}
				<div class="choice-list">
					{#each choices as choice (choice.id)}
						<button class="choice-button" disabled={isLoading} onclick={() => onChoose(choice.id)}
							>{choice.text}</button
						>
					{/each}
				</div>
			{/if}
			{#if npcChoices.length}
				<div class="choice-list npc-choice-list" aria-label="대화 추천 선택지">
					{#each npcChoices as choice, index (`${index}:${choice}`)}
						<button class="choice-button" disabled={isLoading} onclick={() => onNpcChoice(choice)}
							>{choice}</button
						>
					{/each}
				</div>
			{/if}
			{#if canSendInput}
				<form class="free-text-form" onsubmit={handleInputSubmit}>
					<input
						value={inputValue}
						oninput={(event) => onInputChange(event.currentTarget.value)}
						placeholder="하고 싶은 말을 입력하세요…"
						aria-label="대화 입력"
						disabled={isLoading}
					/>
					<button type="submit" disabled={isLoading || !inputValue.trim()}>보내기</button>
				</form>
			{/if}
		{:else if line || sceneDescription}
			<button class="line-content" type="button" onclick={onStageClick} disabled={!isReadingScript}>
				{#if !isNarration(line) && speakerName}<span class="speaker">{speakerName}</span>{/if}
				<span class="dialogue-text" class:narration={isNarration(line)}
					>{line ? line.text || "…" : (sceneDescription ?? "장면을 시작합니다.")}</span
				>
			</button>
		{:else if isLoading}
			<div class="loading-label">장면을 불러오는 중…</div>
		{/if}

		{#if lineCount > 0}
			<div class="line-navigation">
				<button
					class="previous-button"
					onclick={onPrevious}
					disabled={isLoading || (lineIndex === 0 && !scriptFinished)}>이전</button
				>
				<span class="line-progress">{Math.min(lineIndex + 1, lineCount)} / {lineCount}</span>
				<button class="next-button" onclick={onAdvance} disabled={isLoading || !isReadingScript}
					>다음 <ArrowRight size={16} /></button
				>
			</div>
		{/if}
		{#if canStopTalk}
			<button class="stop-talk-button" type="button" disabled={isLoading} onclick={onStopTalk}>대화 종료</button>
		{/if}
	</section>
{/if}

<style>
	.dialogue-panel {
		position: absolute;
		z-index: 100;
		left: 50%;
		bottom: 26px;
		width: min(880px, calc(100% - 40px));
		padding: 20px 24px 18px;
		transform: translateX(-50%);
		border: 1px solid var(--game-border);
		border-radius: 16px;
		color: var(--game-text-main);
		background: var(--game-dialogue-bg);
		box-shadow: var(--game-shadow-dialogue);
		backdrop-filter: blur(14px);
		pointer-events: auto;
	}
	.line-content {
		display: block;
		width: 100%;
		padding: 0;
		border: 0;
		color: inherit;
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.line-content:disabled {
		cursor: default;
		opacity: 1;
	}
	.speaker {
		display: block;
		margin-bottom: 9px;
		color: var(--game-accent);
		font-size: 14px;
		font-weight: 700;
	}
	.dialogue-text {
		display: block;
		min-height: 48px;
		margin: 0;
		color: var(--game-text-primary);
		font-size: 17px;
		line-height: 1.75;
		white-space: pre-wrap;
	}
	.dialogue-text.narration {
		color: var(--game-text-secondary);
		font-size: 15px;
	}
	.line-progress {
		display: block;
		color: var(--game-text-muted);
		font-size: 11px;
		text-align: center;
	}
	.choice-list {
		display: grid;
		gap: 9px;
		margin-top: 17px;
	}
	.choice-button {
		width: 100%;
		padding: 12px 15px;
		border: 1px solid var(--game-accent-border);
		border-radius: var(--radius-lg);
		color: var(--game-text-primary);
		background: var(--game-accent-soft);
		text-align: left;
		cursor: pointer;
		transition:
			background 0.15s,
			transform 0.15s;
	}
	.choice-button:hover:not(:disabled) {
		background: var(--game-accent-hover);
		transform: translateY(-1px);
	}
	.choice-button:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	.npc-choice-list {
		margin-top: 8px;
	}
	.free-text-form {
		display: flex;
		gap: 8px;
		margin-top: 12px;
	}
	.free-text-form input {
		flex: 1;
		min-width: 0;
		padding: 10px 12px;
		border: 1px solid var(--game-border);
		border-radius: var(--radius-md);
		outline: none;
		color: var(--game-text-primary);
		background: var(--game-border-subtle);
		font: inherit;
		font-size: 13px;
	}
	.free-text-form input:focus {
		border-color: var(--game-accent);
	}
	.free-text-form input::placeholder {
		color: var(--game-text-muted);
	}
	.free-text-form button {
		flex: 0 0 auto;
		padding: 9px 13px;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--game-text-primary);
		background: var(--game-accent-soft);
		cursor: pointer;
	}
	.free-text-form button:disabled {
		opacity: 0.5;
		cursor: wait;
	}
	.stop-talk-button {
		display: block;
		margin: 10px 0 0 auto;
		padding: 5px 9px;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--game-text-muted);
		background: transparent;
		font: inherit;
		font-size: 11px;
		cursor: pointer;
	}
	.stop-talk-button:hover:not(:disabled) {
		color: var(--game-text-primary);
		background: var(--game-border-subtle);
	}
	.stop-talk-button:disabled {
		opacity: 0.5;
		cursor: wait;
	}
	.line-navigation {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		gap: 8px;
		margin-top: 12px;
	}
	.next-button,
	.previous-button {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--game-accent-strong);
		background: var(--game-border-subtle);
		cursor: pointer;
	}
	.next-button {
		justify-self: end;
		min-width: 80px;
		justify-content: center;
	}
	.previous-button {
		width: 4rem;
		min-width: 0;
		padding: 5px 8px;
		justify-content: center;
		color: var(--game-text-secondary);
		font-size: 11px;
	}
	.previous-button:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.loading-label {
		color: var(--game-text-muted);
		font-size: 13px;
	}
	@media (max-width: 620px) {
		.dialogue-panel {
			bottom: 12px;
			width: calc(100% - 20px);
			padding: 16px;
		}
		.dialogue-text {
			font-size: 15px;
		}
	}
</style>
