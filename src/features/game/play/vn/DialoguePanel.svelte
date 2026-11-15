<script lang="ts">
	import { ChevronLeft, ChevronRight, ChevronsRight, Copy, EyeOff, LogOut, Maximize2, Minimize2, Pencil, Play, RefreshCw, ScrollText, Send, Square, Volume2, VolumeX } from "@lucide/svelte";
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
		// ---- 대사창 아래 동작 줄 ----
		autoAdvance: boolean;
		bgmMuted: boolean;
		focusMode: boolean;
		onCopy: () => void;
		onToggleAuto: () => void;
		onSkip: () => void;
		onHide: () => void;
		onLog: () => void;
		onToggleMute: () => void;
		onToggleFocus: () => void;
		// ---- 메시지 고치기 · 다시 만들기 (마지막 AI 응답이 무대에 있을 때) ----
		canEditMessage: boolean;
		editing: boolean;
		editDraft: string;
		onEdit: () => void;
		onEditInput: (value: string) => void;
		onEditSave: () => void;
		onEditCancel: () => void;
		onRegenerate: () => void;
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
	<div class="dock">
	<section class="dialogue" aria-live="polite">
		{#if p.editing}
			<!-- 메시지 전체를 고침: 한 줄에 하나씩, 대사는 `이름: 내용`, 지문은 이름 없이 -->
			<div class="edit">
				<textarea
					value={p.editDraft}
					oninput={(e) => p.onEditInput(e.currentTarget.value)}
					onkeydown={(e) => {
						if (e.key === "Escape") p.onEditCancel();
						if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) p.onEditSave();
					}}
					rows="5"
					aria-label="메시지 고치기"
				></textarea>
				<div class="edit-foot">
					<span>한 줄에 하나씩 · 대사는 <code>이름: 내용</code> · Ctrl+Enter 저장</span>
					<button class="g-btn" type="button" onclick={p.onEditCancel}>취소</button>
					<button class="g-btn primary" type="button" disabled={p.isLoading || !p.editDraft.trim()} onclick={p.onEditSave}>저장</button>
				</div>
			</div>
		{:else if showLine}
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
				<p class="idle">오른쪽 패널의 <b>이동·행동</b>에서 다음 할 일을 고름</p>
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
	<!-- 대사창 아래 동작 줄 (메시지 도구처럼) -->
	<div class="actions" role="toolbar" aria-label="대사 동작">
		<button class="act" title="이 대사 복사" aria-label="이 대사 복사" disabled={!showLine} onclick={p.onCopy}><Copy /></button>
		<button class="act" title="이 응답 메시지 고치기" aria-label="메시지 고치기" disabled={!p.canEditMessage || p.editing} onclick={p.onEdit}><Pencil /></button>
		<button class="act" title="이 응답 다시 만들기" aria-label="다시 만들기" disabled={!p.canEditMessage} onclick={p.onRegenerate}><RefreshCw /></button>
		<button class="act" class:on={p.autoAdvance} title={p.autoAdvance ? "자동 넘김 멈추기" : "자동 넘김"} aria-label="자동 넘김" aria-pressed={p.autoAdvance} onclick={p.onToggleAuto}>
			{#if p.autoAdvance}<Square />{:else}<Play />{/if}
		</button>
		<button class="act" title="이 장면 대사 끝까지 넘기기" aria-label="끝까지 넘기기" disabled={!p.isReadingScript} onclick={p.onSkip}><ChevronsRight /></button>
		<span class="gap"></span>
		<button class="act" title="지난 대사 (L)" aria-label="지난 대사" onclick={p.onLog}><ScrollText /></button>
		<button class="act" title="대사창 숨기기 (H) · 무대를 누르면 다시 보임" aria-label="대사창 숨기기" onclick={p.onHide}><EyeOff /></button>
		<button class="act" class:on={p.bgmMuted} title={p.bgmMuted ? "배경음 켜기" : "배경음 끄기"} aria-label="배경음" aria-pressed={p.bgmMuted} onclick={p.onToggleMute}>
			{#if p.bgmMuted}<VolumeX />{:else}<Volume2 />{/if}
		</button>
		<button class="act" class:on={p.focusMode} title={p.focusMode ? "집중 모드 끄기" : "집중 모드 (사이드바 숨김)"} aria-label="집중 모드" aria-pressed={p.focusMode} onclick={p.onToggleFocus}>
			{#if p.focusMode}<Minimize2 />{:else}<Maximize2 />{/if}
		</button>
	</div>
	</div>
{/if}

<style>
	/* 대사창 + 아래 동작 줄 */
	.dock {
		position: absolute;
		z-index: 40;
		left: 50%;
		bottom: 10px;
		transform: translateX(-50%);
		width: min(920px, calc(100% - 28px));
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.actions {
		display: flex;
		align-items: center;
		gap: 2px;
		padding: 0 6px;
	}
	.gap {
		flex: 1;
	}
	.act {
		display: grid;
		place-items: center;
		width: 30px;
		height: 28px;
		border: none;
		border-radius: 8px;
		background: transparent;
		color: var(--game-text-muted);
		cursor: pointer;
	}
	.act :global(svg) {
		width: 15px;
		height: 15px;
	}
	.act:hover:not(:disabled) {
		background: var(--game-surface-hover);
		color: var(--game-text-main);
	}
	.act.on {
		background: var(--game-accent-soft);
		color: var(--game-accent-strong);
	}
	.act:disabled {
		opacity: 0.4;
		cursor: default;
	}
	.edit {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.edit textarea {
		width: 100%;
		min-height: 110px;
		padding: 10px 12px;
		border: 1px solid var(--game-border);
		border-radius: 10px;
		background: var(--game-surface-hover);
		color: var(--game-text-main);
		font: inherit;
		font-size: 15px;
		line-height: 1.6;
		resize: vertical;
	}
	.edit textarea:focus {
		outline: none;
		border-color: var(--game-accent-border);
	}
	.edit-foot {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.edit-foot span {
		flex: 1;
		font-size: 12px;
		color: var(--game-text-muted);
	}
	.edit-foot code {
		font-family: var(--font-mono);
		font-size: 11px;
	}
	.dialogue {
		position: relative;
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
		background: var(--surface-hover);
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
		background: var(--surface-hover);
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
		background: var(--bg-sunken);
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
		background: var(--surface-hover);
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
