<script lang="ts">
	import { BookOpen, ChevronDown, Clock3, CloudSun, Maximize2, MapPin, Minimize2, Music, Volume2, VolumeX } from "@lucide/svelte";
	import { formatGameTime } from "./playUtils";

	// 플레이 화면 위쪽 도구줄: 왼쪽은 지금 상태(장소·날씨·시각을 한 상자에), 오른쪽은 보기 도구
	// 지도·소지품·이동/행동은 오른쪽 패널로 옮겼음
	type Props = {
		locationName: string;
		weatherName: string;
		gameTime?: number;
		canWait: boolean;
		isLoading: boolean;
		showWaitMenu: boolean;
		isBgmMuted: boolean;
		bgmBlocked: boolean;
		focusMode: boolean;
		/** 지난 대사 창이 열려 있음 */
		backlogOpen: boolean;
		onToggleWaitMenu: () => void;
		onWait: (hours: number) => void;
		onBacklog: () => void;
		onToggleMute: () => void;
		onResumeBgm: () => void;
		onToggleFocus: () => void;
	};
	let p: Props = $props();
	const waitDurations = [1, 3, 6, 12, 24];
</script>

<header class="toolbar">
	<div class="status">
		<div class="state-box">
			<span class="item" title="지금 장소"><MapPin /><b>{p.locationName}</b></span>
			{#if p.weatherName}<span class="sep" aria-hidden="true"></span><span class="item" title="날씨"><CloudSun />{p.weatherName}</span>{/if}
			<span class="sep" aria-hidden="true"></span>
			<div class="wait">
				<button
					class="item time"
					onclick={p.onToggleWaitMenu}
					disabled={!p.canWait || p.isLoading}
					aria-expanded={p.showWaitMenu}
					title={p.canWait ? "시간 보내기" : "씬이 진행 중이면 시간을 보낼 수 없음"}
				>
					<Clock3 />{formatGameTime(p.gameTime)}{#if p.canWait}<ChevronDown class="more" />{/if}
				</button>
				{#if p.showWaitMenu}
					<div class="wait-menu" role="menu" aria-label="얼마나 기다릴지">
						<span>얼마나 기다릴까</span>
						{#each waitDurations as hours (hours)}
							<button role="menuitem" disabled={p.isLoading} onclick={() => p.onWait(hours)}>{hours}시간</button>
						{/each}
					</div>
				{/if}
			</div>
		</div>
		{#if p.bgmBlocked}<button class="g-chip unlock" onclick={p.onResumeBgm}><Music />배경음 재생</button>{/if}
	</div>

	<div class="tools">
		<button class="g-icon" aria-label="지난 대사 (L)" title="지난 대사 (L)" aria-pressed={p.backlogOpen} onclick={p.onBacklog}><BookOpen /></button>
		<button class="g-icon" aria-label={p.isBgmMuted ? "배경음 켜기" : "배경음 끄기"} title={p.isBgmMuted ? "배경음 켜기" : "배경음 끄기"} aria-pressed={p.isBgmMuted} onclick={p.onToggleMute}>
			{#if p.isBgmMuted}<VolumeX />{:else}<Volume2 />{/if}
		</button>
		<button class="g-icon" aria-label={p.focusMode ? "집중 모드 끄기" : "집중 모드 (사이드바 숨김)"} title={p.focusMode ? "집중 모드 끄기" : "집중 모드 (사이드바 숨김)"} aria-pressed={p.focusMode} onclick={p.onToggleFocus}>
			{#if p.focusMode}<Minimize2 />{:else}<Maximize2 />{/if}
		</button>
	</div>
</header>

<style>
	.toolbar {
		position: relative;
		z-index: 80;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		min-height: 52px;
		padding: 8px 14px;
		border-bottom: 1px solid var(--game-border-subtle);
		background: var(--game-topbar-bg);
		backdrop-filter: blur(12px);
	}
	.status,
	.tools {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
	}
	.status {
		flex-wrap: wrap;
	}
	.wait {
		position: relative;
	}
	.wait-menu {
		position: absolute;
		top: calc(100% + 8px);
		left: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 150px;
		padding: 8px;
		border: 1px solid var(--game-border);
		border-radius: 12px;
		background: var(--game-panel-bg);
		box-shadow: var(--game-shadow-panel);
	}
	.wait-menu span {
		padding: 4px 8px 6px;
		color: var(--game-text-muted);
		font-size: 12px;
	}
	.wait-menu button {
		height: 32px;
		padding: 0 10px;
		border: none;
		border-radius: 8px;
		background: transparent;
		color: var(--game-text-main);
		font: inherit;
		font-size: 14px;
		text-align: left;
		cursor: pointer;
	}
	.wait-menu button:hover:not(:disabled) {
		background: var(--game-accent-soft);
	}
	.unlock {
		border-color: var(--game-accent-border);
		color: var(--game-accent-strong);
	}
	/* 장소 · 날씨 · 시각을 한 상자에 */
	.state-box {
		display: flex;
		align-items: center;
		min-width: 0;
		height: 34px;
		padding: 0 4px;
		border: 1px solid var(--game-border);
		border-radius: 999px;
		background: var(--game-scene-label-bg);
		color: var(--game-text-main);
		font-size: 13px;
	}
	.item {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		height: 28px;
		padding: 0 8px;
		border: none;
		border-radius: 999px;
		background: transparent;
		color: inherit;
		font: inherit;
		white-space: nowrap;
	}
	.item b {
		font-weight: 600;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.item :global(svg) {
		flex: none;
		width: 14px;
		height: 14px;
		color: var(--game-text-muted);
	}
	.item :global(.more) {
		width: 12px;
		height: 12px;
	}
	button.item {
		cursor: pointer;
		font-variant-numeric: tabular-nums;
	}
	button.item:hover:not(:disabled) {
		background: var(--game-surface-hover);
	}
	button.item:disabled {
		cursor: default;
	}
	.sep {
		flex: none;
		width: 1px;
		height: 14px;
		background: var(--game-border);
	}
</style>
