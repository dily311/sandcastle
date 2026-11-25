<script lang="ts">
	import { onMount } from "svelte";
	import { ExternalLink, Monitor, Moon, Sun } from "@lucide/svelte";
	import type { CssThemeMeta_Serialize as ThemeMeta } from "$ipc/bindings";
	import { nav } from "$app/nav.svelte";
	import { theme } from "$app/theme.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles } from "$shared/ipc/dialog";
	import { Button, Segmented, Slider } from "$shared/ui";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import { BUILTIN_THEMES, defaultColors, withLegacy, type ColorMap } from "$shared/ui/themes";
	import ScrimColorPicker from "$features/settings/ScrimColorPicker.svelte";

	let customThemes = $state<ThemeMeta[]>([]);
	onMount(async () => {
		const list = await call(commands.getThemeList(), { source: "theme", silent: true }).catch(() => []);
		for (const t of list) {
			const empty = !Object.keys(t.colors?.light ?? {}).length && !Object.keys(t.colors?.dark ?? {}).length;
			if (!empty || !t.filename) continue;
			const full = await call(commands.loadTheme(t.filename), { source: "theme", silent: true }).catch(() => null);
			if (full?.cssContent) t.colors = withLegacy({ light: {}, dark: {} }, full.cssContent);
		}
		customThemes = [...list];
	});

	const mode = $derived(theme.dark ? "dark" : "light");
	const base = $derived(defaultColors(mode, theme.setting.grayTone ?? "slate", theme.accentId));
	const swatchColors = (c: { light?: ColorMap; dark?: ColorMap } | undefined): ColorMap => {
		const over = Object.fromEntries(Object.entries(c?.[mode] ?? {}).filter(([, v]) => v?.trim()));
		return { ...base, ...over };
	};
	const selected = $derived(theme.setting.selectTheme || "default");
	const hasBg = $derived(!!theme.setting.backgroundImage);

	let bgScrimDraft = $state<string | null>(null);
	const bgScrimVal = () => bgScrimDraft ?? String(Math.round(Math.min(0.85, Math.max(0, theme.setting.backgroundOpacity ?? 0.35)) * 100));
	const setBgScrimDraft = (v: string) => {
		bgScrimDraft = v;
		const n = Number(v);
		if (!Number.isNaN(n)) {
			document.documentElement.style.setProperty("--chat-bg-scrim-alpha", String(Math.min(0.85, Math.max(0, n / 100))));
		}
	};
	async function commitBgScrim(v: string) {
		bgScrimDraft = null;
		document.documentElement.style.removeProperty("--chat-bg-scrim-alpha");
		const n = v.trim() === "" ? 35 : Math.min(85, Math.max(0, Number(v) || 0));
		await theme.update({ backgroundOpacity: n / 100 });
	}

	async function uploadBackgrounds() {
		const paths = await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true);
		await theme.addBackgrounds(paths, true);
	}
</script>

<div class="panel-body theme-panel">
	<section>
		<h4>화면 모드</h4>
		<Segmented
			label="화면 모드"
			value={theme.setting.mode}
			options={[
				{ value: "system", label: "시스템", icon: Monitor },
				{ value: "light", label: "밝게", icon: Sun },
				{ value: "dark", label: "어둡게", icon: Moon },
			]}
			onchange={(v) => theme.update({ mode: v })}
		/>
	</section>

	<section>
		<h4>
			<span>배경 이미지</span>
		</h4>
		<BackgroundPicker
			label="대화 배경"
			value={theme.setting.backgroundImage ?? null}
			images={theme.setting.backgrounds ?? []}
			onpick={(v) => theme.setBackground(v)}
			onupload={uploadBackgrounds}
			onremove={(p) => theme.removeBackground(p)}
		/>
		{#if hasBg}
			<div class="sub-ctrl">
				<Slider
					label="배경 가림막 (%)"
					min={0}
					max={85}
					step={5}
					fallback={35}
					unsetLabel="35%"
					bind:value={bgScrimVal, setBgScrimDraft}
					oncommit={commitBgScrim}
				/>
			</div>
			<div class="sub-ctrl">
				<span class="sub-label">가림막 색상</span>
				<ScrimColorPicker />
			</div>
		{/if}
	</section>

	<section>
		<h4>
			<span>테마</span>
			<button type="button" class="open-settings" onclick={() => nav.go({ page: "settings", tab: "theme" })}>
				상세 설정 <ExternalLink />
			</button>
		</h4>
		<div class="theme-grid" role="radiogroup" aria-label="테마 선택">
			{#each BUILTIN_THEMES as t (t.id)}
				{@const key = t.id === "default" ? "default" : `builtin:${t.id}`}
				{@const c = swatchColors(t.colors)}
				<button
					type="button"
					role="radio"
					class="tcard"
					class:on={selected === key}
					aria-checked={selected === key}
					onclick={() => theme.update({ selectTheme: key })}
				>
					<span class="preview" style:background={c["--bg"]}>
						<span class="bar" style:background={c["--surface"]}></span>
						<span class="pill" style:background={c["--accent"]}></span>
					</span>
					<span class="tname">{t.name}</span>
				</button>
			{/each}
			{#each customThemes as t (t.filename ?? t.name)}
				{@const key = t.filename ?? "default"}
				{@const c = swatchColors(t.colors)}
				<button
					type="button"
					role="radio"
					class="tcard"
					class:on={selected === key}
					aria-checked={selected === key}
					onclick={() => theme.update({ selectTheme: key })}
				>
					<span class="preview" style:background={c["--bg"]}>
						<span class="bar" style:background={c["--surface"]}></span>
						<span class="pill" style:background={c["--accent"]}></span>
					</span>
					<span class="tname">{t.name}</span>
				</button>
			{/each}
		</div>
	</section>

	<div class="foot">
		<Button size="sm" variant="secondary" onclick={() => nav.go({ page: "settings", tab: "theme" })}>
			{#snippet icon()}<ExternalLink />{/snippet}테마 설정 전체 보기
		</Button>
	</div>
</div>

<style>
	.sub-ctrl {
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
	.open-settings {
		display: inline-flex;
		align-items: center;
		gap: 3px;
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		font-size: var(--text-xs);
		font-weight: 500;
		cursor: pointer;
	}
	.open-settings:hover {
		text-decoration: underline;
	}
	.open-settings :global(svg) {
		width: 11px;
		height: 11px;
	}
	.theme-grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-2);
	}
	.tcard {
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
	.tcard:hover {
		border-color: var(--border-strong);
		background: var(--surface-hover);
	}
	.tcard.on {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent);
	}
	.preview {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 4px;
		height: 30px;
		padding: 4px;
		border-radius: var(--radius-xs);
		border: 1px solid var(--border);
		overflow: hidden;
	}
	.preview .bar {
		flex: 1;
		height: 100%;
		border-radius: 3px;
	}
	.preview .pill {
		width: 14px;
		height: 14px;
		border-radius: 999px;
		flex: none;
	}
	.tname {
		font-size: var(--text-xs);
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.foot {
		display: flex;
		justify-content: stretch;
	}
	.foot :global(button) {
		width: 100%;
		justify-content: center;
	}
</style>
