<script lang="ts">
	import { AudioLines, ChevronLeft, ChevronRight, ChevronsRight, Clock, Copy, DoorOpen, EyeOff, ImagePlus, Languages, LogOut, MapPin, Maximize2, MessageCircle, Minimize2, Pencil, Play, RefreshCw, ScrollText, Search, Send, Square, Volume2, VolumeX } from "@lucide/svelte";
	import type { PlayAction } from "./playUtils";
	import { HtmlView } from "$shared/ui";
	import { wholeHtmlBlock } from "$shared/html/blocks";
	import type { Choice, StageLine } from "$features/game/types";
	import { isNarration } from "./playUtils";

	// 화면 아래 대사창: 대사 읽기 → 다 읽으면 선택지 / 대화 입력
	type Props = {
		/** 대사창(독) 높이를 알림: 좁은 화면에서 캐릭터를 그 위에 세우려고 */
		onDockHeight?: (h: number) => void;
		show: boolean;
		line: StageLine | null;
		speakerName: string | null;
		sceneDescription: string | null;
		choices: Choice[];
		npcChoices: string[];
		canSendInput: boolean;
		canStopTalk: boolean;
		/** 할 일이 없을 때(대사·선택지·입력 없음) 보여줄 이동·행동 목록 */
		actions?: PlayAction[];
		/** 열린 씬을 끝낼 수 있는가 (씬 입력을 쓸 수 있는 씬) */
		canEndScene?: boolean;
		inputPlaceholder?: string;
		inputValue: string;
		lineIndex: number;
		lineCount: number;
		scriptFinished: boolean;
		isReadingScript: boolean;
		isLoading: boolean;
		/** 응답을 기다리는 동안 받은 미리보기 글 */
		streamingText?: string;
		onStageClick: () => void;
		onPrevious: () => void;
		onAdvance: () => void;
		onChoose: (choiceId: string) => void;
		onNpcChoice: (text: string) => void;
		onInputChange: (value: string) => void;
		onSubmitInput: () => void;
		onStopTalk: () => void;
		onEndScene?: () => void;
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
		// ---- 코어 플러그인 AI 도구 (켜진 것만 버튼이 보임) ----
		onTranslate?: () => void;
		translating?: boolean;
		/** 지금 줄이 번역문으로 보이는 중 */
		translated?: boolean;
		onSpeak?: () => void;
		speaking?: boolean;
		onDraw?: () => void;
		drawing?: boolean;
	};
	let p: Props = $props();
	let dockH = $state(0);
	$effect(() => p.onDockHeight?.(p.show ? dockH : 0));
	const actionIcon = { talk: MessageCircle, look: Search, move: MapPin, wait: Clock } as const;

	const narration = $derived(isNarration(p.line));
	/** 이 줄이 통째로 HTML 코드 블록이면 그 코드 (백엔드 대본 파서가 코드 블록을 한 줄로 둔다) */
	const htmlCode = $derived(p.line ? wholeHtmlBlock(p.line.text) : null);
	const text = $derived(p.line ? p.line.text || "…" : (p.sceneDescription ?? "장면이 시작됨"));
	const showLine = $derived(!p.scriptFinished && (p.line || p.sceneDescription));

	function submit(e: SubmitEvent) {
		e.preventDefault();
		p.onSubmitInput();
	}
</script>

{#if p.show}
	<div class="dock" bind:offsetHeight={dockH}>
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
		{:else if p.isLoading && p.streamingText?.trim()}
			<!-- 응답 원문에는 "이름:" 접두가 들어 있어서 화자 표시는 따로 하지 않는다 -->
			<p class="line streaming" aria-busy="true">{p.streamingText.trim()}</p>
		{:else if showLine}
			{#if !narration && p.speakerName}<span class="speaker">{p.speakerName}</span>{/if}
			{#if htmlCode}<div class="html-line"><HtmlView code={htmlCode} /></div>{/if}
			<button class="line" type="button" onclick={p.onStageClick} disabled={!p.isReadingScript} title="다음 (Space)">
				<span class:narration>{htmlCode ? "" : text}</span>
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
						placeholder={p.inputPlaceholder ?? "하고 싶은 말 입력"}
						aria-label={p.inputPlaceholder ?? "대화 입력"}
						disabled={p.isLoading}
					/>
					<button class="g-btn primary" type="submit" disabled={p.isLoading || !p.inputValue.trim()}><Send />보내기</button>
				</form>
			{:else if !p.choices.length && !p.npcChoices.length}
				{#if p.actions?.length}
					<ul class="acts" aria-label="다음에 할 일">
						{#each p.actions as action (action.id)}
							{@const Icon = actionIcon[action.kind]}
							<li>
								<button class="act-chip" type="button" disabled={p.isLoading || action.disabled} title={action.hint} onclick={action.run}>
									<Icon /><span>{action.label}</span>{#if action.hint}<small>{action.hint}</small>{/if}
								</button>
							</li>
						{/each}
					</ul>
				{:else}
					<p class="idle">지금 할 수 있는 것이 없음</p>
				{/if}
			{/if}
		{:else if p.isLoading}
			<p class="idle">장면을 불러오는 중…</p>
		{/if}

		{#if p.lineCount > 0 || p.canStopTalk || p.canEndScene}
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
				{#if p.canEndScene && p.onEndScene}<button class="g-btn stop" type="button" disabled={p.isLoading} onclick={p.onEndScene} title="AI 서술을 멈추고 이 장면을 마침"><DoorOpen />장면 끝내기</button>{/if}
			</div>
		{/if}
	</section>
	<!-- 대사창 아래 동작 줄 (메시지 도구처럼) -->
	<div class="actions" role="toolbar" aria-label="대사 동작">
		<button class="act" title="이 대사 복사" aria-label="이 대사 복사" disabled={!showLine} onclick={p.onCopy}><Copy /></button>
		<button class="act" title="이 응답 메시지 고치기" aria-label="메시지 고치기" disabled={!p.canEditMessage || p.editing} onclick={p.onEdit}><Pencil /></button>
		<button class="act" title="이 응답 다시 만들기" aria-label="다시 만들기" disabled={!p.canEditMessage} onclick={p.onRegenerate}><RefreshCw /></button>
		{#if p.onTranslate}
			<button class="act" class:on={p.translated} title={p.translated ? "원문 보기" : "이 대사 번역"} aria-label="대사 번역" aria-pressed={p.translated} disabled={!showLine || p.translating} onclick={p.onTranslate}><Languages /></button>
		{/if}
		{#if p.onSpeak}
			<button class="act" class:on={p.speaking} title={p.speaking ? "읽기 멈추기" : "이 대사 소리 내어 읽기"} aria-label="대사 읽기" aria-pressed={p.speaking} disabled={!showLine} onclick={p.onSpeak}><AudioLines /></button>
		{/if}
		{#if p.onDraw}
			<button class="act" title="이 장면 그리기" aria-label="장면 그리기" disabled={!showLine || p.drawing} onclick={p.onDraw}><ImagePlus /></button>
		{/if}
		<button class="act" class:on={p.autoAdvance} title={p.autoAdvance ? "자동 넘김 멈추기" : "자동 넘김"} aria-label="자동 넘김" aria-pressed={p.autoAdvance} onclick={p.onToggleAuto}>
			{#if p.autoAdvance}<Square />{:else}<Play />{/if}
		</button>
		<button class="act" title="이 장면 대사 끝까지 넘기기" aria-label="끝까지 넘기기" disabled={!p.isReadingScript} onclick={p.onSkip}><ChevronsRight /></button>
		<span class="gap"></span>
		<button class="act" title="지난 대사 (L)" aria-label="지난 대사" onclick={p.onLog}><ScrollText /></button>
		<button class="act" title="대사창 숨기기 (H) · 무대를 누르면 다시 보임" aria-label="대사창 숨기기" onclick={p.onHide}><EyeOff /></button>
		<button class="act" class:on={p.bgmMuted} title={p.bgmMuted ? "소리 켜기 (배경음·효과음)" : "소리 끄기 (배경음·효과음)"} aria-label="소리" aria-pressed={p.bgmMuted} onclick={p.onToggleMute}>
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
		z-index: calc(var(--z-stage-ui) + 10);
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
		color: var(--icon-static-white);
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
	.html-line {
		max-height: 50vh;
		overflow: auto;
	}
	.line.streaming {
		margin: 0;
		cursor: default;
		opacity: 0.85;
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
	.acts {
		display: flex;
		flex-direction: column;
		gap: 6px;
		margin: 4px 0 6px;
		padding: 0;
		list-style: none;
	}
	.act-chip {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		min-height: 40px;
		padding: 8px 14px;
		border: 1px solid var(--game-border);
		border-radius: 12px;
		background: var(--surface-hover);
		color: var(--game-text-main);
		font: inherit;
		font-size: 15px;
		text-align: left;
		cursor: pointer;
	}
	.act-chip :global(svg) {
		width: 16px;
		height: 16px;
		flex: none;
		color: var(--game-text-muted);
	}
	.act-chip span {
		flex: 1;
		min-width: 0;
	}
	.act-chip small {
		flex: none;
		color: var(--game-text-muted);
		font-size: 12px;
	}
	.act-chip:hover:not(:disabled) {
		border-color: var(--game-text-muted);
	}
	.act-chip:disabled {
		opacity: 0.5;
		cursor: default;
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
