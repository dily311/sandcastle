<script lang="ts">
	import { ChevronLeft, ChevronRight, LogOut, Send } from "@lucide/svelte";
	import type { Choice, StageLine } from "$features/game/types";
	import { isNarration } from "./playUtils";

	// 화면 아래 대사창: 대사 읽기 → 다 읽으면 선택지 / 대화 입력
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
	let p: Props = $props();

	const narration = $derived(isNarration(p.line));
	const text = $derived(p.line ? p.line.text || "…" : (p.sceneDescription ?? "장면이 시작됨"));
	const showLine = $derived(!p.scriptFinished && (p.line || p.sceneDescription));

	function submit(e: SubmitEvent) {
		e.preventDefault();
		p.onSubmitInput();
	}
</script>

{#if p.show}
	<section class="dialogue" aria-live="polite">
		{#if showLine}
			{#if !narration && p.speakerName}<span class="speaker">{p.speakerName}</span>{/if}
			<button class="line" type="button" onclick={p.onStageClick} disabled={!p.isReadingScript} title="다음 (Space)">
				<span class:narration>{text}</span>
				{#if p.isReadingScript}<span class="next-mark" aria-hidden="true">▼</span>{/if}
			</button>
		{:else if p.scriptFinished}
			{#if p.choices.length}
				<ol class="choices" aria-label="선택지">
					{#each p.choices as choice, i (choice.id)}
						<li>
							<button class="choice" disabled={p.isLoading} onclick={() => p.onChoose(choice.id)}>
								<kbd>{i + 1}</kbd><span>{choice.text}</span>
							</button>
						</li>
					{/each}
				</ol>
			{/if}
			{#if p.npcChoices.length}
				<ol class="choices suggest" aria-label="이렇게 말해 보기">
					{#each p.npcChoices as choice, i (`${i}:${choice}`)}
						<li>
							<button class="choice" disabled={p.isLoading} onclick={() => p.onNpcChoice(choice)}>
								<kbd>{p.choices.length + i + 1}</kbd><span>{choice}</span>
							</button>
						</li>
					{/each}
				</ol>
			{/if}
			{#if p.canSendInput}
				<form class="say" onsubmit={submit}>
					<input
						value={p.inputValue}
						oninput={(e) => p.onInputChange(e.currentTarget.value)}
						placeholder="하고 싶은 말 입력"
						aria-label="대화 입력"
						disabled={p.isLoading}
					/>
					<button class="g-btn primary" type="submit" disabled={p.isLoading || !p.inputValue.trim()}><Send />보내기</button>
				</form>
			{:else if !p.choices.length && !p.npcChoices.length}
				<p class="idle">위쪽 <b>이동 / 행동</b>에서 다음 할 일을 고름</p>
			{/if}
		{:else if p.isLoading}
			<p class="idle">장면을 불러오는 중…</p>
		{/if}

		{#if p.lineCount > 0 || p.canStopTalk}
			<div class="foot">
				{#if p.lineCount > 0}
					<button class="g-icon" aria-label="이전 대사 (←)" title="이전 대사 (←)" onclick={p.onPrevious} disabled={p.isLoading || (p.lineIndex === 0 && !p.scriptFinished)}><ChevronLeft /></button>
					<span class="progress" aria-label="대사 진행">
						<span class="bar"><i style:width="{(Math.min(p.lineIndex + 1, p.lineCount) / p.lineCount) * 100}%"></i></span>
						{Math.min(p.lineIndex + 1, p.lineCount)} / {p.lineCount}
					</span>
					<button class="g-icon" aria-label="다음 대사 (Space)" title="다음 대사 (Space)" onclick={p.onAdvance} disabled={p.isLoading || !p.isReadingScript}><ChevronRight /></button>
				{/if}
				<span class="sp"></span>
				{#if p.canStopTalk}<button class="g-btn stop" type="button" disabled={p.isLoading} onclick={p.onStopTalk}><LogOut />대화 끝내기</button>{/if}
			</div>
		{/if}
	</section>
{/if}

<style>
	.dialogue {
		position: absolute;
		z-index: 40;
		left: 50%;
		bottom: 18px;
		transform: translateX(-50%);
		width: min(920px, calc(100% - 28px));
		padding: 18px 20px 12px;
		border: 1px solid var(--game-border);
		border-radius: 18px;
		background: var(--game-dialogue-bg);
		box-shadow: var(--game-shadow-dialogue);
		backdrop-filter: blur(14px);
	}
	.speaker {
		position: absolute;
		top: -14px;
		left: 20px;
		padding: 4px 14px;
		border: 1px solid var(--game-accent-border);
		border-radius: 999px;
		background: color-mix(in oklab, var(--accent) 35%, #141518);
		color: var(--game-text-primary);
		font-size: 14px;
		font-weight: 700;
		letter-spacing: 0.01em;
	}
	.line {
		position: relative;
		display: block;
		width: 100%;
		min-height: 74px;
		padding: 4px 22px 4px 0;
		border: none;
		background: none;
		color: var(--game-text-primary);
		font: inherit;
		font-size: 17px;
		line-height: 1.75;
		text-align: left;
		white-space: pre-wrap;
		cursor: pointer;
	}
	.line:disabled {
		cursor: default;
	}
	.narration {
		color: var(--game-text-secondary);
		font-style: italic;
	}
	.next-mark {
		position: absolute;
		right: 0;
		bottom: 4px;
		color: var(--game-accent);
		font-size: 11px;
		animation: bob 1.2s ease-in-out infinite;
	}
	@keyframes bob {
		50% {
			transform: translateY(3px);
			opacity: 0.6;
		}
	}
	.choices {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.choices + .choices,
	.choices + .say {
		margin-top: 10px;
	}
	.choice {
		display: flex;
		align-items: center;
		gap: 12px;
		width: 100%;
		min-height: 44px;
		padding: 8px 14px;
		border: 1px solid var(--game-border);
		border-radius: 12px;
		background: rgb(255 255 255 / 0.04);
		color: var(--game-text-main);
		font: inherit;
		font-size: 15px;
		text-align: left;
		cursor: pointer;
		transition:
			background 120ms ease,
			border-color 120ms ease;
	}
	.choice:hover:not(:disabled) {
		border-color: var(--game-accent-border);
		background: var(--game-accent-soft);
		color: var(--game-text-primary);
	}
	.suggest .choice {
		border-style: dashed;
	}
	kbd {
		display: grid;
		place-items: center;
		min-width: 22px;
		height: 22px;
		border-radius: 6px;
		background: rgb(255 255 255 / 0.08);
		color: var(--game-accent);
		font-family: var(--font-mono);
		font-size: 12px;
		font-weight: 700;
	}
	.say {
		display: flex;
		gap: 8px;
	}
	.say input {
		flex: 1;
		height: 40px;
		padding: 0 14px;
		border: 1px solid var(--game-border);
		border-radius: 10px;
		background: rgb(0 0 0 / 0.25);
		color: var(--game-text-primary);
		font: inherit;
		font-size: 15px;
		outline: none;
	}
	.say input:focus {
		border-color: var(--game-accent-border);
	}
	.say .g-btn {
		height: 40px;
	}
	.idle {
		margin: 4px 0 6px;
		color: var(--game-text-muted);
		font-size: 14px;
	}
	.idle b {
		color: var(--game-text-secondary);
	}
	.foot {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-top: 8px;
		padding-top: 8px;
		border-top: 1px solid var(--game-border-subtle);
	}
	.progress {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--game-text-dim);
		font-size: 12px;
		font-variant-numeric: tabular-nums;
	}
	.bar {
		width: 90px;
		height: 3px;
		border-radius: 3px;
		background: rgb(255 255 255 / 0.1);
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
		background: var(--game-accent);
	}
	.sp {
		flex: 1;
	}
	.stop {
		height: 30px;
		font-size: 13px;
	}
	@media (max-width: 620px) {
		.dialogue {
			bottom: 10px;
			padding: 16px 14px 10px;
		}
		.line {
			font-size: 15px;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.next-mark {
			animation: none;
		}
	}
</style>
