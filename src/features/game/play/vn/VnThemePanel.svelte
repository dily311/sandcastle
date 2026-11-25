<script lang="ts">
	import { RotateCcw } from "@lucide/svelte";
	import { Button, Segmented, Slider } from "$shared/ui";
	import { VN_ASPECTS, VN_BOX_STYLES, VN_PRESETS, vnTheme } from "./vnTheme.svelte";

	const ACCENT_PRESETS: { value: string; label: string }[] = [
		{ value: "#ec4899", label: "로즈 핑크" },
		{ value: "#818cf8", label: "라벤더" },
		{ value: "#38bdf8", label: "스카이 블루" },
		{ value: "#f59e0b", label: "웜 골드" },
		{ value: "#10b981", label: "에메랄드" },
		{ value: "#f43f5e", label: "체리 레드" },
	];

	const cfg = $derived(vnTheme.config);
	const isCustomAccent = $derived(!!cfg.accentColor && !ACCENT_PRESETS.some((p) => p.value.toLowerCase() === cfg.accentColor?.toLowerCase()));

	let opacityDraft = $state<string | null>(null);
	const opacityVal = () => opacityDraft ?? String(Math.round(cfg.dialogueOpacity * 100));
	const setOpacityDraft = (v: string) => {
		opacityDraft = v;
		const n = Number(v);
		if (!Number.isNaN(n)) vnTheme.update({ dialogueOpacity: Math.min(1, Math.max(0.3, n / 100)) });
	};
	const commitOpacity = (v: string) => {
		opacityDraft = null;
		const n = v.trim() === "" ? 90 : Math.min(100, Math.max(30, Number(v) || 90));
		vnTheme.update({ dialogueOpacity: n / 100 });
	};

	let fontDraft = $state<string | null>(null);
	const fontVal = () => fontDraft ?? String(cfg.fontSize);
	const setFontDraft = (v: string) => {
		fontDraft = v;
		const n = Number(v);
		if (!Number.isNaN(n) && n >= 13 && n <= 24) vnTheme.update({ fontSize: Math.round(n) });
	};
	const commitFont = (v: string) => {
		fontDraft = null;
		const n = v.trim() === "" ? 17 : Math.min(24, Math.max(13, Math.round(Number(v) || 17)));
		vnTheme.update({ fontSize: n });
	};

	let scaleDraft = $state<string | null>(null);
	const scaleVal = () => scaleDraft ?? String(Math.round(cfg.spriteScale * 100));
	const setScaleDraft = (v: string) => {
		scaleDraft = v;
		const n = Number(v);
		if (!Number.isNaN(n)) vnTheme.update({ spriteScale: Math.min(1.2, Math.max(0.7, n / 100)) });
	};
	const commitScale = (v: string) => {
		scaleDraft = null;
		const n = v.trim() === "" ? 100 : Math.min(120, Math.max(70, Number(v) || 100));
		vnTheme.update({ spriteScale: n / 100 });
	};
</script>

<div class="panel-body vn-theme-panel">
	<section>
		<h4>화면 비율</h4>
		<Segmented
			label="게임 화면 비율"
			value={cfg.aspect}
			options={VN_ASPECTS}
			onchange={(v) => vnTheme.update({ aspect: v })}
		/>
	</section>

	<section>
		<h4>연애시뮬 테마 프리셋</h4>
		<div class="preset-grid" role="radiogroup" aria-label="게임 UI 테마 프리셋">
			{#each VN_PRESETS as p (p.id)}
				<button
					type="button"
					role="radio"
					class="pcard"
					class:on={cfg.preset === p.id}
					aria-checked={cfg.preset === p.id}
					onclick={() => vnTheme.applyPreset(p.id)}
				>
					<span class="swatch" style:background={p.bg}>
						<span class="box" style:background={p.box} style:border-color={p.accent}>
							<span class="tag" style:background={p.accent}></span>
						</span>
					</span>
					<span class="pname">{p.name}</span>
				</button>
			{/each}
		</div>
	</section>

	<section>
		<h4>대사창 스타일</h4>
		<Segmented
			label="대사창 스타일"
			value={cfg.boxStyle}
			options={VN_BOX_STYLES}
			onchange={(v) => vnTheme.update({ boxStyle: v })}
		/>
	</section>

	<section>
		<h4>세부 커스텀</h4>
		<Slider
			label="대사창 불투명도 (%)"
			min={30}
			max={100}
			step={5}
			fallback={90}
			unsetLabel="90%"
			bind:value={opacityVal, setOpacityDraft}
			oncommit={commitOpacity}
		/>
		<Slider
			label="대사 글자 크기 (px)"
			min={13}
			max={24}
			step={1}
			fallback={17}
			unsetLabel="17px"
			bind:value={fontVal, setFontDraft}
			oncommit={commitFont}
		/>
		<Slider
			label="캐릭터 크기 (%)"
			min={70}
			max={120}
			step={5}
			fallback={100}
			unsetLabel="100%"
			bind:value={scaleVal, setScaleDraft}
			oncommit={commitScale}
		/>
		<div class="accent-row">
			<span class="sub-label">강조 색상</span>
			<div class="colors" role="radiogroup" aria-label="게임 강조 색상">
				<button
					type="button"
					role="radio"
					class="auto"
					class:on={cfg.accentColor === null}
					aria-checked={cfg.accentColor === null}
					onclick={() => vnTheme.update({ accentColor: null })}
				>
					기본
				</button>
				{#each ACCENT_PRESETS as c (c.value)}
					<button
						type="button"
						role="radio"
						class="dot"
						class:on={cfg.accentColor?.toLowerCase() === c.value}
						aria-checked={cfg.accentColor?.toLowerCase() === c.value}
						aria-label={c.label}
						title={c.label}
						style:background={c.value}
						onclick={() => vnTheme.update({ accentColor: c.value })}
					></button>
				{/each}
				<input
					type="color"
					class="custom"
					class:on={isCustomAccent}
					value={cfg.accentColor ?? "#ec4899"}
					aria-label="강조 색상 직접 고르기"
					title="직접 고르기"
					oninput={(e) => vnTheme.update({ accentColor: e.currentTarget.value })}
				/>
			</div>
		</div>
	</section>

	<div class="foot">
		<Button size="sm" variant="secondary" onclick={() => vnTheme.reset()}>
			{#snippet icon()}<RotateCcw />{/snippet}기본값으로 초기화
		</Button>
	</div>
</div>

<style>
	.preset-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-2);
	}
	.pcard {
		display: flex;
		flex-direction: column;
		gap: 5px;
		padding: 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
		transition:
			border-color var(--duration-fast) var(--ease),
			background var(--duration-fast) var(--ease);
	}
	.pcard:hover {
		border-color: var(--border-strong);
		background: var(--surface-hover);
	}
	.pcard.on {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent);
	}
	.swatch {
		position: relative;
		display: flex;
		align-items: flex-end;
		height: 38px;
		padding: 5px;
		border-radius: var(--radius-xs);
		border: 1px solid var(--border);
		overflow: hidden;
	}
	.swatch .box {
		position: relative;
		width: 100%;
		height: 16px;
		border: 1px solid;
		border-radius: 4px;
	}
	.swatch .tag {
		position: absolute;
		top: -4px;
		left: 5px;
		width: 18px;
		height: 6px;
		border-radius: 999px;
	}
	.pname {
		font-size: var(--text-xs);
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.accent-row {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
	}
	.sub-label {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.colors {
		display: flex;
		align-items: center;
		gap: 6px;
		flex-wrap: wrap;
	}
	.auto {
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-full);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.auto.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent-text);
		font-weight: 600;
	}
	.dot,
	.custom {
		width: 22px;
		height: 22px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
		padding: 0;
		flex: none;
	}
	.custom {
		background: conic-gradient(from 0deg, #ef4444, #f59e0b, #22c55e, #3b82f6, #a855f7, #ef4444);
		overflow: hidden;
	}
	.custom::-webkit-color-swatch-wrapper {
		padding: 0;
	}
	.custom::-webkit-color-swatch {
		border: none;
		border-radius: 50%;
		opacity: 0;
	}
	.custom.on::-webkit-color-swatch {
		opacity: 1;
	}
	.dot.on,
	.custom.on {
		box-shadow: 0 0 0 2px var(--accent);
	}
	.foot {
		display: flex;
	}
	.foot :global(button) {
		width: 100%;
		justify-content: center;
	}
</style>
