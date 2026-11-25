<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronDown, ChevronLeft, ChevronRight, RotateCcw } from "@lucide/svelte";
	import { Button, Field, IconButton, Input, Segmented, Spinner, Switch, Textarea } from "$shared/ui";
	import { BUILTIN_THEMES, COLOR_VARS, builtinId, defaultColors, type ColorMap, type Mode, type ThemeColors, withLegacy } from "$shared/ui/themes";
	import { theme } from "$app/theme.svelte";
	import { ACCENTS, accentValue, isHexColor } from "$shared/ui/accents";
	import { GRAYS } from "$shared/ui/grays";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";

	let { editKey }: { editKey: string } = $props();
	const src = { source: "theme" };

	let filename = $state<string | null>(null);
	let name = $state("");
	let author = $state("");
	let version = $state("1.0.0");
	let colors = $state<ThemeColors>({ light: {}, dark: {} });
	let css = $state("");
	/** 이 테마가 정할 강조색·회색 톤·대화 표시 설정 (처음엔 지금 설정, 5-E-14) */
	let accent = $state<string>(theme.accentId);
	let gray = $state<string>(theme.setting.grayTone ?? "slate");
	let showTimestamps = $state<boolean>(theme.setting.chat?.showTimestamps ?? true);
	let showAvatars = $state<boolean>(theme.setting.chat?.showAvatars ?? true);
	let showMultiCharAvatars = $state<boolean>(theme.setting.chat?.showMultiCharAvatars ?? false);
	let loaded = $state(false);
	let original = "";
	let saving = $state(false);
	let mode = $state<Mode>(theme.dark ? "dark" : "light");
	let cssOpen = $state(false);

	const snapshot = () => JSON.stringify({ name, author, version, colors: clean(colors), css, accent, gray, showTimestamps, showAvatars, showMultiCharAvatars });
	function clean(c: ThemeColors): ThemeColors {
		const f = (m: ColorMap) => Object.fromEntries(Object.entries(m).filter(([, v]) => v?.trim()));
		return { light: f(c.light), dark: f(c.dark) };
	}

	onMount(async () => {
		// svelte-ignore state_referenced_locally
		const key = editKey;
		const bid = builtinId(key);
		if (key === "new") {
			name = $_("settings.more3.newTheme");
		} else if (bid) {
			const b = BUILTIN_THEMES.find((t) => t.id === bid);
			name = $_("settings.more.copySuffix", { values: { name: b?.name ?? $_("settings.more3.themeLabel") } });
			colors = structuredClone(b?.colors ?? { light: {}, dark: {} });
		} else {
			const t = await call(commands.loadTheme(key), src).catch(() => null);
			if (!t) return back(true);
			filename = key;
			name = t.name;
			author = t.author;
			version = t.version || "1.0.0";
			colors = withLegacy({ light: { ...(t.colors.light ?? {}) }, dark: { ...(t.colors.dark ?? {}) } }, t.cssContent ?? "");
			css = t.cssContent ?? "";
			cssOpen = !!css.trim();
			if (t.accentColor) accent = t.accentColor;
			if (t.grayTone) gray = t.grayTone;
			if (t.chat) {
				showTimestamps = t.chat.showTimestamps;
				showAvatars = t.chat.showAvatars;
				showMultiCharAvatars = t.chat.showMultiCharAvatars;
			}
		}
		original = key === "new" || bid ? "" : snapshot();
		loaded = true;
	});

	const dirty = $derived(loaded && snapshot() !== original);
	// 색은 아래 예시 화면에만 보인다 (앱 전체를 바꾸면 고르는 동안 설정 화면까지 바뀌어 헷갈림, 5-E-14). 직접 쓴 CSS는 저장한 뒤에 적용
	onDestroy(() => theme.preview(null));
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: $_("settings.more3.unsavedTitle"), message: $_("settings.more3.discardThemeMsg"), confirmLabel: $_("settings.more3.discardBtn") }));
	});

	function back(force = false) {
		if (force) nav.clearGuard();
		void nav.replace({ page: "settings", tab: "theme" });
	}
	async function save() {
		if (!name.trim()) return toasts.error($_("settings.more3.enterNameErr"));
		saving = true;
		try {
			const file = filename ?? `theme-${Date.now().toString(36)}.json`;
			await call(
				commands.saveTheme(file, {
					name: name.trim(),
					author: author.trim(),
					version: version.trim() || "1.0.0",
					cssContent: css,
					colors: clean(colors),
					accentColor: accent,
					grayTone: gray,
					chat: { showTimestamps, showAvatars, showMultiCharAvatars },
				}),
				src,
			);
			theme.preview(null);
			// 새로 만든 테마는 바로 적용, 쓰는 중인 테마는 다시 읽음
			if (!filename || theme.setting.selectTheme === file) await theme.update({ selectTheme: file });
			toasts.success($_("settings.more3.themeSavedToast"));
			back(true);
		} finally {
			saving = false;
		}
	}

	const base = $derived(defaultColors(mode, gray, accent));
	const value = (k: string) => colors[mode][k]?.trim() || "";
	const shown = (k: string) => value(k) || base[k] || "";
	const isHex = (v: string) => /^#[0-9a-f]{6}$/i.test(v);
	function set(k: string, v: string) {
		colors[mode] = { ...colors[mode], [k]: v };
	}
	function reset(k: string) {
		const { [k]: _drop, ...rest } = colors[mode];
		colors[mode] = rest;
	}
	function resetAll() {
		colors[mode] = {};
	}
	function copyFromOther() {
		const other = mode === "light" ? "dark" : "light";
		colors[mode] = { ...colors[other] };
	}
	const preview = $derived(Object.entries({ ...base, ...clean(colors)[mode] }).map(([k, v]) => `${k}: ${v}`).join("; "));
	const changed = $derived(Object.keys(clean(colors)[mode]).length);
</script>

{#if !loaded}
	<div class="center"><Spinner /></div>
{:else}
	<div class="crumb">
		<IconButton label={$_("settings.more3.backToThemeAria")} size="sm" onclick={() => back()}><ChevronLeft /></IconButton>
		<span>{$_("settings.more3.themeLabel")}</span>
	</div>
	<header class="head">
		<h1>{filename ? $_("settings.more3.editTheme") : $_("settings.more3.newTheme")}</h1>
		<div class="btns">
			{#if dirty}<span class="dirty">{$_("settings.more3.dirty")}</span>{/if}
			<Button variant="ghost" onclick={() => back()}>{$_("common.cancel")}</Button>
			<Button variant="primary" loading={saving} disabled={!name.trim() || (!!filename && !dirty)} onclick={save}>{$_("common.save")}</Button>
		</div>
	</header>
	<p class="lead">{$_("settings.more3.themeEditorLead")}</p>

	<div class="card info">
		<Field label={$_("settings.more3.name")} required><Input bind:value={name} /></Field>
		<Field label={$_("settings.more3.author")}><Input bind:value={author} /></Field>
		<Field label={$_("settings.more3.version")}><Input bind:value={version} /></Field>
	</div>

	<div class="card info">
		<Field label={$_("settings.themeSec.accent")}>
			<div class="accents" role="radiogroup" aria-label={$_("settings.more3.themeAccentAria")}>
				{#each ACCENTS as a (a.id)}
					{@const alabel = $_(`uiPolish.accents.${a.id}`, { default: a.label })}
					<button type="button" role="radio" aria-checked={accent === a.id} aria-label={alabel} title={alabel} class:on={accent === a.id} style:background={mode === "dark" ? a.dark : a.light} onclick={() => (accent = a.id)}></button>
				{/each}
				<input type="color" class:on={isHexColor(accent)} value={accentValue(accent, mode === "dark")} aria-label={$_("settings.more3.themeAccentCustomAria")} oninput={(e) => (accent = e.currentTarget.value)} />
			</div>
		</Field>
		<Field label={$_("settings.themeSec.grayTone")}>
			<div class="accents" role="radiogroup" aria-label={$_("settings.more3.themeGrayAria")}>
				{#each GRAYS as g (g.id)}
					{@const sc = mode === "dark" ? g.dark : g.light}
					{@const glabel = $_(`uiPolish.grays.${g.id}`, { default: g.label })}
					<button type="button" role="radio" aria-checked={gray === g.id} aria-label={glabel} title={glabel} class:on={gray === g.id} style:background="linear-gradient(135deg, {sc[2]} 0 50%, {sc[10]} 50% 100%)" onclick={() => (gray = g.id)}></button>
				{/each}
			</div>
		</Field>
	</div>

	<div class="card chat-opts">
		<div class="chat-opt">
			<div class="txt"><b>{$_("settings.themeSec.showTimestamps")}</b><span class="faint small">{$_("settings.themeSec.showTimestampsDesc")}</span></div>
			<Switch checked={showTimestamps} label={$_("settings.themeSec.showTimestamps")} hideLabel onchange={(v) => (showTimestamps = v)} />
		</div>
		<div class="chat-opt">
			<div class="txt"><b>{$_("settings.themeSec.showAvatars")}</b><span class="faint small">{$_("settings.themeSec.showAvatarsDesc")}</span></div>
			<Switch checked={showAvatars} label={$_("settings.themeSec.showAvatars")} hideLabel onchange={(v) => (showAvatars = v)} />
		</div>
		<div class="chat-opt">
			<div class="txt"><b>{$_("settings.themeSec.multiCharAvatars")}</b><span class="faint small">{$_("settings.themeSec.multiCharAvatarsDesc")}</span></div>
			<Switch checked={showMultiCharAvatars} label={$_("settings.themeSec.multiCharAvatars")} hideLabel onchange={(v) => (showMultiCharAvatars = v)} />
		</div>
	</div>

	<div class="colors-head">
		<h3>{$_("settings.more3.colorsHeading")}</h3>
		<Segmented label={$_("settings.more3.editModeAria")} bind:value={mode} options={[{ value: "light", label: $_("settings.more3.lightScreen") }, { value: "dark", label: $_("settings.more3.darkScreen") }]} />
	</div>
	<p class="lead">{$_("settings.more3.colorsLead")}{#if mode !== (theme.dark ? "dark" : "light")}{$_("settings.more3.colorsLeadOtherMode", { values: { mode: theme.dark ? $_("settings.more3.modeDarkWord") : $_("settings.more3.modeLightWord") } })}{/if}</p>

	<div class="mock" style={preview} aria-label={$_("settings.more3.previewAria")}>
		<div class="m-side">
			<span class="m-logo"></span>
			<span class="m-item on">{$_("nav.character")}</span>
			<span class="m-item">{$_("nav.persona")}</span>
			<span class="m-item">{$_("nav.settings")}</span>
		</div>
		<div class="m-main">
			<div class="m-group">
				<div class="m-card">
					<b>Seraphina</b>
					<span class="m-muted">{$_("settings.more3.mockCharDesc")}</span>
					<span class="m-faint">{$_("settings.more3.mockTime")}</span>
				</div>
				<div class="m-input">{$_("settings.more3.mockInput")}</div>
				<div class="m-row">
					<span class="m-btn">{$_("settings.more3.mockSend")}</span>
					<span class="m-ghost">{$_("common.cancel")}</span>
				</div>
			</div>
		</div>
	</div>

	<div class="card vars">
		<div class="vars-tools">
			<span class="faint small">{changed ? $_("settings.more3.changedCount", { values: { count: changed } }) : $_("settings.more3.noChangedColors")}</span>
			<Button size="sm" variant="ghost" onclick={copyFromOther}>{$_("settings.more3.copyOtherModeColors", { values: { mode: mode === "light" ? $_("settings.more3.modeDarkWord") : $_("settings.more3.modeLightWord") } })}</Button>
			<Button size="sm" variant="ghost" disabled={!changed} onclick={resetAll}>{#snippet icon()}<RotateCcw />{/snippet}{$_("settings.more3.resetAllDefault")}</Button>
		</div>
		{#each COLOR_VARS as g (g.id)}
			<p class="group">{$_(`uiPolish.colorGroups.${g.id}`, { default: g.group })}</p>
			{#each g.items as v (v.name)}
				{@const vlabel = $_(`uiPolish.colorVars.${v.name}`, { default: v.label })}
				{@const vhint = v.hint ? $_(`uiPolish.colorVars.${v.name}__hint`, { default: v.hint }) : ""}
				<div class="var" class:set={!!value(v.name)}>
					<div class="txt"><b>{vlabel}</b><span class="faint small">{vhint} <code>{v.name}</code></span></div>
					<label class="swatch" style:background={shown(v.name)} title={$_("settings.more3.pickColorTitle")}>
						<input type="color" value={isHex(shown(v.name)) ? shown(v.name) : "#000000"} oninput={(e) => set(v.name, e.currentTarget.value)} aria-label={$_("settings.more3.pickColorNamedAria", { values: { label: vlabel } })} />
					</label>
					<input class="hex" value={value(v.name)} placeholder={base[v.name]} spellcheck="false" aria-label={$_("settings.more3.colorValueNamedAria", { values: { label: vlabel } })} onchange={(e) => (e.currentTarget.value.trim() ? set(v.name, e.currentTarget.value.trim()) : reset(v.name))} />
					<IconButton label={$_("settings.more3.resetColorNamedAria", { values: { label: vlabel } })} size="sm" disabled={!value(v.name)} onclick={() => reset(v.name)}><RotateCcw /></IconButton>
				</div>
			{/each}
		{/each}
	</div>

	<section class="advanced">
		<button class="adv-toggle" aria-expanded={cssOpen} onclick={() => (cssOpen = !cssOpen)}>
			{#if cssOpen}<ChevronDown />{:else}<ChevronRight />{/if}
			{$_("settings.more3.customCssHeading")} <span class="faint small">{$_("settings.more3.customCssHint")}</span>
		</button>
		{#if cssOpen}
			<div class="card">
				<Textarea bind:value={css} mono rows={14} spellcheck={false} placeholder={':root[data-theme="dark"] {\n\t--radius-md: 4px;\n}'} />
			</div>
		{/if}
	</section>
{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		padding: var(--space-8);
	}
	.crumb {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin: calc(var(--space-4) * -1) 0 var(--space-2) calc(var(--space-2) * -1);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	h1 {
		font-size: var(--text-xl);
	}
	.btns {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.dirty {
		font-size: var(--text-sm);
		color: var(--accent-text);
	}
	.lead {
		margin: var(--space-1) 0 var(--space-4);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.card {
		padding: var(--space-4) var(--space-5);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}
	.info {
		display: grid;
		grid-template-columns: 2fr 1fr 1fr;
		gap: var(--space-3);
	}
	.card + .card {
		margin-top: var(--space-3);
	}
	.chat-opts {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.chat-opt {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
	}
	.colors-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-top: var(--space-6);
	}
	.mock {
		display: grid;
		grid-template-columns: 130px 1fr;
		height: 190px;
		margin-bottom: var(--space-3);
		border-radius: var(--radius-lg);
		overflow: hidden;
		box-shadow: 0 0 0 1px var(--border);
		background: var(--bg);
		color: var(--text);
		font-size: 12px;
	}
	.m-side {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 10px 8px;
		border-right: 1px solid var(--border);
		background: var(--bg);
	}
	.m-logo {
		width: 16px;
		height: 16px;
		margin: 0 0 6px 4px;
		border-radius: 4px;
		background: var(--accent);
	}
	.m-item {
		padding: 4px 6px;
		border-radius: 6px;
		color: var(--text-muted);
	}
	.m-item.on {
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
	}
	.m-main {
		padding: 14px;
		background: var(--surface-2);
	}
	.m-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		border-radius: 10px;
		background: var(--bg-sunken);
	}
	.m-card {
		display: flex;
		flex-direction: column;
		padding: 8px 10px;
		border: 1px solid var(--border);
		border-radius: 8px;
		background: var(--surface);
	}
	.m-muted {
		color: var(--text-muted);
	}
	.m-faint {
		color: var(--text-faint);
		font-size: 11px;
	}
	.m-input {
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: 8px;
		background: var(--surface);
		color: var(--text-faint);
	}
	.m-row {
		display: flex;
		gap: 6px;
	}
	.m-btn {
		padding: 4px 10px;
		border-radius: 6px;
		background: var(--accent);
		color: var(--text-on-accent);
		font-weight: 600;
	}
	.m-ghost {
		padding: 4px 10px;
		border-radius: 6px;
		background: var(--surface-hover);
		color: var(--text);
	}
	.vars-tools {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.vars-tools > :first-child {
		flex: 1;
	}
	.group {
		margin: var(--space-4) 0 var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.var {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 30px 110px auto;
		align-items: center;
		gap: var(--space-2);
		padding: 6px 0;
	}
	.var + .var {
		border-top: 1px solid var(--border);
	}
	.txt {
		display: flex;
		flex-direction: column;
	}
	.var.set .txt b::after {
		content: " •";
		color: var(--accent);
	}
	code {
		font-family: var(--font-mono);
		font-size: 11px;
	}
	.small {
		font-size: var(--text-xs);
	}
	.swatch {
		position: relative;
		width: 30px;
		height: 30px;
		border-radius: 8px;
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
		overflow: hidden;
	}
	.swatch input {
		position: absolute;
		inset: 0;
		opacity: 0;
		cursor: pointer;
	}
	.hex {
		height: 30px;
		padding: 0 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-xs);
		background: var(--surface);
		color: var(--text);
		font-family: var(--font-mono);
		font-size: 12px;
	}
	.hex:focus {
		outline: 2px solid var(--focus-ring);
		border-color: var(--accent);
	}
	.advanced {
		margin-top: var(--space-6);
	}
	.adv-toggle {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-3);
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	.adv-toggle :global(svg) {
		width: 16px;
		height: 16px;
		color: var(--text-faint);
	}
	.accents {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.accents button,
	.accents input {
		width: 28px;
		height: 28px;
		padding: 0;
		border: 2px solid transparent;
		border-radius: var(--radius-full, 999px);
		cursor: pointer;
	}
	.accents input {
		background: none;
	}
	.accents .on {
		border-color: var(--text);
	}
</style>
