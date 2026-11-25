<script lang="ts">
	import { onMount, tick } from "svelte";
	import { interactiveManager } from "$/lib/interaction/interactive-registry";
	import { WidgetGame } from "./pixi/WidgetGame";
	import { mascotUi } from "./mascotFeature.svelte";
	import type { MascotOverlayState } from "./simejiType";

	const PROMPT_REGION_ID = "widget.mascot.prompt";

	let container: HTMLDivElement;
	let promptEl = $state<HTMLDivElement | undefined>(undefined);
	let inputEl = $state<HTMLInputElement | undefined>(undefined);
	let game: WidgetGame | undefined;

	let overlays = $state<MascotOverlayState[]>([]);
	let promptText = $state("");
	let sending = $state(false);

	const activeOverlay = $derived(
		mascotUi.activePrompt ? overlays.find((o) => o.id === mascotUi.activePrompt?.mascotId) ?? null : null,
	);

	onMount(() => {
		game = new WidgetGame((items) => {
			overlays = items;
		});
		void game.init(container);

		return () => {
			interactiveManager.unregisterInteractiveElement(PROMPT_REGION_ID);
			game?.destroy();
		};
	});

	$effect(() => {
		const open = Boolean(mascotUi.activePrompt && activeOverlay);
		if (open) {
			void tick().then(() => {
				if (promptEl) {
					interactiveManager.registerInteractiveElement(PROMPT_REGION_ID, promptEl);
				}
				inputEl?.focus();
			});
		} else {
			interactiveManager.unregisterInteractiveElement(PROMPT_REGION_ID);
			promptText = "";
		}
	});

	async function submitPrompt(e: SubmitEvent) {
		e.preventDefault();
		if (!game || !mascotUi.activePrompt || sending) return;
		const text = promptText.trim();
		if (!text) return;

		const { mascotId, mode } = mascotUi.activePrompt;
		promptText = "";
		mascotUi.close();
		sending = true;
		try {
			if (mode === "chat") {
				await game.sendChatToMascot(mascotId, text);
			} else {
				await game.giveGiftToMascot(mascotId, text);
			}
		} finally {
			sending = false;
		}
	}

	function clampX(x: number) {
		if (typeof window === "undefined") return x;
		return Math.max(130, Math.min(window.innerWidth - 130, x));
	}
</script>

<div bind:this={container} class="simeji"></div>

<div class="simeji-overlay" aria-live="polite">
	{#each overlays as item (item.id)}
		{#if item.bubbleText}
			<div
				class="bubble"
				class:happy={item.bubbleMood === "happy"}
				class:annoyed={item.bubbleMood === "annoyed"}
				class:notify={item.bubbleMood === "notify"}
				class:peer={item.bubbleMood === "peer"}
				style="left: {clampX(item.x)}px; top: {item.topY}px;"
			>
				{#if item.bubbleSpeaker}
					<div class="bubble-speaker">{item.bubbleSpeaker}</div>
				{/if}
				<div class="bubble-text">{item.bubbleText}</div>
			</div>
		{/if}
	{/each}

	{#if mascotUi.activePrompt && activeOverlay}
		<div
			bind:this={promptEl}
			class="prompt-box"
			style="left: {clampX(activeOverlay.x)}px; top: {Math.max(76, activeOverlay.topY - 12)}px;"
		>
			<form onsubmit={submitPrompt}>
				<input
					bind:this={inputEl}
					bind:value={promptText}
					type="text"
					placeholder={mascotUi.activePrompt.mode === "chat" ? `${activeOverlay.name}에게 말 걸기...` : "줄 선물 입력 (예: 딸기 케이크)"}
					onkeydown={(e) => {
						if (e.key === "Escape") mascotUi.close();
					}}
				/>
				<button type="submit" disabled={!promptText.trim() || sending}>
					{mascotUi.activePrompt.mode === "chat" ? "전송" : "주기"}
				</button>
				<button type="button" class="close-btn" onclick={() => mascotUi.close()} aria-label="닫기">×</button>
			</form>
		</div>
	{/if}
</div>

<style>
	.simeji {
		position: fixed;
		inset: 0;
		overflow: hidden;
		/* z-index를 다른 위젯보다 높게 설정 금지 */
	}
	:global(.simeji canvas) {
		display: block;
		width: 100%;
		height: 100%;
	}
	.simeji-overlay {
		position: fixed;
		inset: 0;
		pointer-events: none;
		z-index: 20;
	}
	.bubble {
		position: fixed;
		transform: translate(-50%, -100%);
		max-width: 240px;
		min-width: 80px;
		padding: 8px 12px;
		border-radius: 14px;
		background: color-mix(in srgb, var(--surface, #fffaf3) 94%, transparent);
		color: var(--text, #2b2118);
		border: 1px solid var(--border-strong, rgba(0, 0, 0, 0.14));
		box-shadow: 0 6px 18px rgba(0, 0, 0, 0.16);
		font-size: 12.5px;
		line-height: 1.45;
		word-break: keep-all;
		overflow-wrap: break-word;
		backdrop-filter: blur(8px);
		animation: bubble-pop 0.18s ease-out;
	}
	.bubble::after {
		content: "";
		position: absolute;
		left: 50%;
		bottom: -6px;
		width: 10px;
		height: 10px;
		transform: translateX(-50%) rotate(45deg);
		background: inherit;
		border-right: 1px solid var(--border-strong, rgba(0, 0, 0, 0.14));
		border-bottom: 1px solid var(--border-strong, rgba(0, 0, 0, 0.14));
	}
	.bubble.happy {
		border-color: var(--accent, #cf6a43);
	}
	.bubble.annoyed {
		border-color: var(--danger, #d94c4c);
	}
	.bubble.notify {
		border-color: var(--info, #3b82f6);
	}
	.bubble.peer {
		border-color: color-mix(in srgb, var(--accent, #cf6a43) 65%, var(--border-strong, #999));
	}
	.bubble-speaker {
		font-size: 10.5px;
		font-weight: 600;
		color: var(--text-muted, #6b5d50);
		margin-bottom: 2px;
	}
	.bubble-text {
		white-space: pre-wrap;
	}
	.prompt-box {
		position: fixed;
		transform: translate(-50%, -100%);
		pointer-events: auto;
		background: var(--surface, #fffaf3);
		border: 1px solid var(--border-strong, rgba(0, 0, 0, 0.2));
		border-radius: 12px;
		padding: 6px;
		box-shadow: 0 8px 24px rgba(0, 0, 0, 0.22);
		z-index: 30;
	}
	.prompt-box form {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.prompt-box input {
		width: 190px;
		padding: 6px 9px;
		border-radius: 8px;
		border: 1px solid var(--border, rgba(0, 0, 0, 0.14));
		background: var(--bg, #f7f2eb);
		color: var(--text, #2b2118);
		font-size: 12.5px;
		outline: none;
	}
	.prompt-box button {
		padding: 6px 10px;
		border-radius: 8px;
		border: none;
		background: var(--accent, #cf6a43);
		color: #fff;
		font-size: 12px;
		font-weight: 600;
		cursor: pointer;
	}
	.prompt-box button:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.prompt-box .close-btn {
		padding: 4px 8px;
		background: transparent;
		color: var(--text-muted, #6b5d50);
		font-size: 15px;
	}
	@keyframes bubble-pop {
		from {
			opacity: 0;
			transform: translate(-50%, -88%) scale(0.94);
		}
		to {
			opacity: 1;
			transform: translate(-50%, -100%) scale(1);
		}
	}
</style>
