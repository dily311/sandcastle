<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { ChevronDown, ChevronLeft, ChevronRight, RotateCcw } from "@lucide/svelte";
	import { Button, Field, IconButton, Input, Segmented, Spinner, Textarea } from "$shared/ui";
	import { BUILTIN_THEMES, COLOR_VARS, builtinId, defaultColors, type ColorMap, type Mode, type ThemeColors, withLegacy } from "$shared/ui/themes";
	import { theme } from "$app/theme.svelte";
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
	let loaded = $state(false);
	let original = "";
	let saving = $state(false);
	let mode = $state<Mode>(theme.dark ? "dark" : "light");
	let cssOpen = $state(false);

	const snapshot = () => JSON.stringify({ name, author, version, colors: clean(colors), css });
	function clean(c: ThemeColors): ThemeColors {
		const f = (m: ColorMap) => Object.fromEntries(Object.entries(m).filter(([, v]) => v?.trim()));
		return { light: f(c.light), dark: f(c.dark) };
	}

	onMount(async () => {
		// svelte-ignore state_referenced_locally
		const key = editKey;
		const bid = builtinId(key);
		if (key === "new") {
			name = "새 테마";
		} else if (bid) {
			const b = BUILTIN_THEMES.find((t) => t.id === bid);
			name = `${b?.name ?? "테마"} 사본`;
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
		}
		original = key === "new" || bid ? "" : snapshot();
		loaded = true;
	});

	const dirty = $derived(loaded && snapshot() !== original);
	// 고치는 동안 앱 전체에 바로 보여줌
	$effect(() => {
		if (!loaded) return;
		theme.preview({ colors: clean(colors), css });
	});
	onDestroy(() => theme.preview(null));
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: "저장하지 않음", message: "바뀐 테마를 버리고 나감", confirmLabel: "버리기" }));
	});

	function back(force = false) {
		if (force) nav.clearGuard();
		void nav.replace({ page: "settings", tab: "theme" });
	}
	async function save() {
		if (!name.trim()) return toasts.error("이름을 입력해야 함");
		saving = true;
		try {
			const file = filename ?? `theme-${Date.now().toString(36)}.json`;
			await call(commands.saveTheme(file, { name: name.trim(), author: author.trim(), version: version.trim() || "1.0.0", cssContent: css, colors: clean(colors) }), src);
			theme.preview(null);
			// 새로 만든 테마는 바로 적용, 쓰는 중인 테마는 다시 읽음
			if (!filename || theme.setting.selectTheme === file) await theme.update({ selectTheme: file });
			toasts.success("테마 저장함");
			back(true);
		} finally {
			saving = false;
		}
	}

	const base = $derived(defaultColors(mode, theme.setting.grayTone ?? "slate", theme.accentId));
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
		<IconButton label="테마 목록으로" size="sm" onclick={() => back()}><ChevronLeft /></IconButton>
		<span>테마</span>
	</div>
	<header class="head">
		<h1>{filename ? "테마 편집" : "새 테마"}</h1>
		<div class="btns">
			{#if dirty}<span class="dirty">바뀐 내용 있음</span>{/if}
			<Button variant="ghost" onclick={() => back()}>취소</Button>
			<Button variant="primary" loading={saving} disabled={!name.trim() || (!!filename && !dirty)} onclick={save}>저장</Button>
		</div>
	</header>
	<p class="lead">고치는 동안 앱 전체에 바로 보임. 저장하지 않고 나가면 원래대로</p>

	<div class="card info">
		<Field label="이름" required><Input bind:value={name} /></Field>
		<Field label="만든 사람"><Input bind:value={author} /></Field>
		<Field label="버전"><Input bind:value={version} /></Field>
	</div>

	<div class="colors-head">
		<h3>색</h3>
		<Segmented label="고칠 화면 모드" bind:value={mode} options={[{ value: "light", label: "밝은 화면" }, { value: "dark", label: "어두운 화면" }]} />
	</div>
	<p class="lead">비워 둔 색은 회색 톤·강조색 설정을 따라감{#if mode !== (theme.dark ? "dark" : "light")}. 앱은 지금 {theme.dark ? "어두운" : "밝은"} 화면이라 아래 미리보기로 확인{/if}</p>

	<div class="mock" style={preview} aria-label="미리보기">
		<div class="m-side">
			<span class="m-logo"></span>
			<span class="m-item on">캐릭터</span>
			<span class="m-item">페르소나</span>
			<span class="m-item">설정</span>
		</div>
		<div class="m-main">
			<div class="m-group">
				<div class="m-card">
					<b>Seraphina</b>
					<span class="m-muted">엘도리아 숲의 수호자</span>
					<span class="m-faint">2분 전</span>
				</div>
				<div class="m-input">메시지 입력</div>
				<div class="m-row">
					<span class="m-btn">보내기</span>
					<span class="m-ghost">취소</span>
				</div>
			</div>
		</div>
	</div>

	<div class="card vars">
		<div class="vars-tools">
			<span class="faint small">{changed ? `${changed}개 바꿈` : "바꾼 색 없음"}</span>
			<Button size="sm" variant="ghost" onclick={copyFromOther}>{mode === "light" ? "어두운" : "밝은"} 화면 색 가져오기</Button>
			<Button size="sm" variant="ghost" disabled={!changed} onclick={resetAll}>{#snippet icon()}<RotateCcw />{/snippet}모두 기본값</Button>
		</div>
		{#each COLOR_VARS as g (g.group)}
			<p class="group">{g.group}</p>
			{#each g.items as v (v.name)}
				<div class="var" class:set={!!value(v.name)}>
					<div class="txt"><b>{v.label}</b><span class="faint small">{v.hint ?? ""} <code>{v.name}</code></span></div>
					<label class="swatch" style:background={shown(v.name)} title="색 고르기">
						<input type="color" value={isHex(shown(v.name)) ? shown(v.name) : "#000000"} oninput={(e) => set(v.name, e.currentTarget.value)} aria-label="{v.label} 색 고르기" />
					</label>
					<input class="hex" value={value(v.name)} placeholder={base[v.name]} spellcheck="false" aria-label="{v.label} 색 값" onchange={(e) => (e.currentTarget.value.trim() ? set(v.name, e.currentTarget.value.trim()) : reset(v.name))} />
					<IconButton label="{v.label} 기본값으로" size="sm" disabled={!value(v.name)} onclick={() => reset(v.name)}><RotateCcw /></IconButton>
				</div>
			{/each}
		{/each}
	</div>

	<section class="advanced">
		<button class="adv-toggle" aria-expanded={cssOpen} onclick={() => (cssOpen = !cssOpen)}>
			{#if cssOpen}<ChevronDown />{:else}<ChevronRight />{/if}
			직접 쓰는 CSS <span class="faint small">고급 · 색 편집으로 부족할 때만</span>
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
</style>
