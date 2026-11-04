<script lang="ts">
	import { ChevronLeft, Clock3, BookOpen, Package, Volume2, VolumeX, Map as MapIcon } from "@lucide/svelte";
	import Header from "$/components/Header.svelte";
	import { formatGameTime } from "./playUtils";

	type Props = {
		title: string;
		locationName: string;
		gameTime?: number;
		canWait: boolean;
		isLoading: boolean;
		showWaitMenu: boolean;
		isBgmMuted: boolean;
		onBack: () => void;
		onToggleWaitMenu: () => void;
		onWait: (hours: number) => void;
		onBacklog: () => void;
		onInventory: () => void;
		onToggleMute: () => void;
		onMap: () => void;
	};
	let {
		title,
		locationName,
		gameTime,
		canWait,
		isLoading,
		showWaitMenu,
		isBgmMuted,
		onBack,
		onToggleWaitMenu,
		onWait,
		onBacklog,
		onInventory,
		onToggleMute,
		onMap,
	}: Props = $props();
	const waitDurations = [1, 3, 6, 12, 24];
</script>

<Header class="topbar">
	<button class="quiet-button" onclick={onBack} aria-label="뒤로 가기"><ChevronLeft size={20} /></button>
	<div class="game-heading">{title}</div>
	<div class="top-actions">
		<button
			class="time-chip"
			onclick={onToggleWaitMenu}
			disabled={!canWait || isLoading}
			aria-label="대기 시간 선택"
			aria-expanded={showWaitMenu}
			title={canWait ? "대기 시간 선택" : "씬이 진행 중일 때는 시간을 보낼 수 없습니다."}
		>
			<Clock3 size={15} />
			{formatGameTime(gameTime)}
		</button>
		{#if showWaitMenu}
			<div class="wait-menu" aria-label="대기 시간 선택">
				<strong>얼마나 기다릴까요?</strong>
				{#each waitDurations as hours (hours)}
					<button disabled={isLoading} onclick={() => onWait(hours)}>{hours}시간</button>
				{/each}
			</div>
		{/if}
		<button class="quiet-button" onclick={onBacklog} aria-label="대화 기록"><BookOpen size={18} /></button>
		<button class="quiet-button" onclick={onInventory} aria-label="보유 아이템 보기"><Package size={18} /></button>
		<button
			class="quiet-button"
			onclick={onToggleMute}
			aria-label={isBgmMuted ? "배경음 켜기" : "배경음 끄기"}
			aria-pressed={isBgmMuted}
		>
			{#if isBgmMuted}<VolumeX size={18} />{:else}<Volume2 size={18} />{/if}
		</button>
		<button class="quiet-button" onclick={onMap} aria-label="지도 보기"><MapIcon size={18} /></button>
	</div>
</Header>

<style>
	:global(.vn-player > .header.topbar) {
		height: 62px;
		flex: 0 0 62px;
		display: flex;
		align-items: center;
		gap: 14px;
		padding: 0 max(7rem, calc(env(safe-area-inset-right, 0px) + 7rem)) 0 max(22px, env(safe-area-inset-left, 0px));
		background: var(--game-topbar-bg);
		border-bottom: 1px solid var(--game-border-subtle);
		position: relative;
		top: auto;
		z-index: 31;
	}
	:global(body.is-frameless .vn-player > .header.topbar) {
		height: var(--header-height);
		flex-basis: var(--header-height);
	}
	.quiet-button {
		display: inline-grid;
		place-items: center;
		width: 38px;
		height: 38px;
		border: 0;
		border-radius: 50%;
		color: var(--game-text-main);
		background: transparent;
		cursor: pointer;
	}
	.quiet-button:hover {
		background: var(--game-surface-hover);
	}
	.game-heading {
		display: flex;
		align-items: center;
		gap: 8px;
		min-width: 0;
	}
	.game-heading :global(strong) {
		font-size: 14px;
		font-weight: 650;
	}
	.game-heading :global(span) {
		overflow: hidden;
		color: var(--game-text-muted);
		font-size: 12px;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.top-actions {
		position: relative;
		display: flex;
		align-items: center;
		gap: 7px;
		margin-left: auto;
	}
	.time-chip {
		display: flex;
		align-items: center;
		gap: 7px;
		margin-right: 8px;
		padding: 5px 8px;
		border: 0;
		border-radius: var(--radius-md);
		color: var(--game-text-secondary);
		background: transparent;
		font-size: 13px;
		cursor: pointer;
	}
	.time-chip:hover:not(:disabled) {
		background: var(--game-surface-hover);
	}
	.time-chip:disabled {
		opacity: 0.65;
		cursor: default;
	}
	.wait-menu {
		position: absolute;
		z-index: 140;
		top: calc(100% + 8px);
		right: 130px;
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: 6px;
		width: 220px;
		padding: 12px;
		border: 1px solid var(--game-border);
		border-radius: var(--radius-lg);
		background: var(--game-panel-bg);
		box-shadow: var(--game-shadow-panel);
		backdrop-filter: blur(14px);
	}
	.wait-menu strong {
		grid-column: 1 / -1;
		padding: 3px 5px 6px;
		color: var(--game-text-main);
		font-size: 12px;
	}
	.wait-menu button {
		padding: 9px 8px;
		border: 1px solid var(--game-border-subtle);
		border-radius: var(--radius-md);
		color: var(--game-text-primary);
		background: var(--game-border-subtle);
		cursor: pointer;
	}
	.wait-menu button:hover:not(:disabled) {
		color: var(--game-accent-strong);
		background: var(--game-accent-soft);
	}
	.wait-menu button:disabled {
		opacity: 0.55;
		cursor: wait;
	}
	@media (max-width: 620px) {
		:global(.vn-player > .header.topbar) {
			padding: 0 max(7rem, calc(env(safe-area-inset-right, 0px) + 7rem)) 0
				max(10px, env(safe-area-inset-left, 0px));
			gap: 7px;
		}
		.top-actions {
			gap: 0;
		}
		.time-chip {
			margin-right: 0;
		}
	}
</style>
