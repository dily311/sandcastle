<script lang="ts">
	import { onMount } from "svelte";
	import { Copy, Pencil, Plus, Trash2, Upload } from "@lucide/svelte";
	import { pickFiles } from "$shared/ipc/dialog";
	import { drops, isFont, isImageAvif, storeFile } from "$app/drop.svelte";
	import { declarePreviewFonts, fontFamilyOf } from "$shared/ui/userFont";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import type { CssThemeMeta_Serialize as ThemeMeta } from "$ipc/bindings";
	import { Button, ContextMenu, Input, Segmented, Select, Switch, Textarea, type MenuItem } from "$shared/ui";
	import { ACCENTS } from "$shared/ui/accents";
	import { GRAYS } from "$shared/ui/grays";
	import { BUILTIN_THEMES, defaultColors, withLegacy, type ColorMap } from "$shared/ui/themes";
	import { theme, type CssSnippet } from "$app/theme.svelte";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import ThemeCard from "./ThemeCard.svelte";

	const src = { source: "theme" };
	let list = $state<ThemeMeta[]>([]);
	async function loadList() {
		list = await call(commands.getThemeList(), { ...src, silent: true }).catch(() => []);
		// 색을 메타데이터에 갖고 있지 않은 예전 테마는 CSS에서 색을 뽑아 카드 미리보기에 쓴다 (파일은 고치지 않음)
		for (const t of list) {
			const empty = !Object.keys(t.colors?.light ?? {}).length && !Object.keys(t.colors?.dark ?? {}).length;
			if (!empty || !t.filename) continue;
			const full = await call(commands.loadTheme(t.filename), { ...src, silent: true }).catch(() => null);
			if (full?.cssContent) t.colors = withLegacy({ light: {}, dark: {} }, full.cssContent);
		}
		list = [...list];
	}
	onMount(loadList);

	const mode = $derived(theme.dark ? "dark" : "light");
	const base = $derived(defaultColors(mode, theme.setting.grayTone ?? "slate", theme.accentId));
	const cardColors = (c: { light?: ColorMap; dark?: ColorMap } | undefined): ColorMap => {
		const over = Object.fromEntries(Object.entries(c?.[mode] ?? {}).filter(([, v]) => v?.trim()));
		return { ...base, ...over };
	};
	const selected = $derived(theme.setting.selectTheme || "default");

	// ---- 배경 이미지 (GIF 가능) ----
	const hasBg = $derived(!!theme.setting.backgroundImage);
	async function uploadBackgrounds() {
		const paths = await pickFiles([{ name: "이미지", extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true);
		await theme.addBackgrounds(paths, true);
	}
	// ---- 글꼴 (올린 글꼴 중에서 앱 글꼴을 고름) ----
	const fonts = $derived(theme.setting.fonts ?? []);
	const fontOptions = $derived([{ value: "", label: "기본 (Pretendard)" }, ...fonts.map((f) => ({ value: f.path, label: f.name }))]);
	// 글꼴 목록의 미리보기용 @font-face: 이 화면이 열려 있는 동안만 올린 글꼴을 모두 불러옴
	$effect(() => declarePreviewFonts(fonts));
	async function uploadFonts() {
		await addFonts(await pickFiles([{ name: "글꼴", extensions: ["ttf", "otf", "woff", "woff2"] }], true));
	}
	async function addFonts(paths: string[]) {
		if (!paths.length) return;
		const before = new Set(fonts.map((f) => f.path));
		await theme.addFonts(paths, false);
		const added = (theme.setting.fonts ?? []).filter((f) => !before.has(f.path));
		// 올리자마자 바꾸지는 않고, 한 개를 올렸으면 바로 쓸 수 있게 알림에 버튼을 둠
		if (added.length === 1) toasts.show({ tone: "success", message: `글꼴 '${added[0].name}'을(를) 올림`, action: { label: "앱 글꼴로 쓰기", run: () => void theme.update({ appFont: added[0].path }) } });
		else toasts.success(`글꼴 ${added.length}개를 올림`);
	}
	async function removeFont(path: string, name: string) {
		if (!(await confirmAction({ title: "글꼴 삭제", message: `'${name}' 글꼴을 목록에서 지우고 복사해 둔 파일도 지움 (올릴 때 고른 원본 파일은 그대로)`, confirmLabel: "삭제", danger: true }))) return;
		await theme.removeFont(path);
	}
	// 이 화면에 놓은 파일: 그림은 배경으로, 글꼴은 글꼴로 (같은 화면에 드롭 대상을 둘 등록하면 나중 것이 앞의 것을 가려서 하나로)
	$effect(() =>
		drops.register({
			label: "배경 이미지·글꼴 올리기",
			hint: "그림(PNG·JPG·WEBP·GIF·AVIF)은 배경으로, 글꼴(TTF·OTF·WOFF·WOFF2)은 글꼴 목록으로",
			accept: (f) => isImageAvif(f) || isFont(f),
			ondrop: async (files) => {
				const images = files.filter(isImageAvif);
				const fontFiles = files.filter(isFont);
				if (images.length) await theme.addBackgrounds(await Promise.all(images.map((f) => storeFile(f))), true);
				if (fontFiles.length) await addFonts(await Promise.all(fontFiles.map((f) => storeFile(f))));
			},
		}),
	);
	const opacities = [0.5, 0.65, 0.75, 0.82, 0.9, 0.95].map((v) => ({ value: String(v), label: `${Math.round(v * 100)}%` }));
	const selectKey = (key: string) => theme.update({ selectTheme: key });

	// ---- 사용자 테마 메뉴 ----
	let menu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function themeMenu(e: MouseEvent, t: ThemeMeta) {
		e.preventDefault();
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{ label: "편집", icon: Pencil, onselect: () => edit(t.filename ?? "") },
				{ label: "복제", icon: Copy, onselect: () => duplicate(t) },
				{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => remove(t) },
			],
		};
	}
	function openMenu(e: MouseEvent, t: ThemeMeta) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		themeMenu(new MouseEvent("contextmenu", { clientX: r.right - 160, clientY: r.bottom + 4 }), t);
	}
	const edit = (key: string) => nav.go({ page: "settings", tab: "theme", edit: key });
	async function duplicate(t: ThemeMeta) {
		const full = await call(commands.loadTheme(t.filename ?? ""), src);
		await call(commands.saveTheme(`theme-${Date.now().toString(36)}.json`, { name: `${full.name} 사본`, author: full.author, version: full.version, cssContent: full.cssContent, colors: full.colors }), src);
		await loadList();
	}
	async function remove(t: ThemeMeta) {
		if (!(await confirmAction({ title: "테마 삭제", message: `'${t.name}' 테마를 지움`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteTheme(t.filename ?? ""), src);
		if (selected === t.filename) await theme.update({ selectTheme: "default" });
		await loadList();
	}

	// ---- 스니펫 ----
	let openSnippet = $state<string | null>(null);
	async function addSnippet() {
		const s: CssSnippet = { id: `s-${Date.now().toString(36)}`, name: "새 스니펫", css: "/* 예: 대화 글자를 조금 크게 */\n.block p {\n\tfont-size: 1.05em;\n}", enabled: false };
		await theme.saveSnippets([...theme.snippets, s]);
		openSnippet = s.id;
	}
	async function patchSnippet(id: string, p: Partial<CssSnippet>) {
		await theme.saveSnippets(theme.snippets.map((s) => (s.id === id ? { ...s, ...p } : s)));
	}
	async function removeSnippet(s: CssSnippet) {
		if (!(await confirmAction({ title: "스니펫 삭제", message: `'${s.name}' 스니펫을 지움`, confirmLabel: "삭제", danger: true }))) return;
		await theme.saveSnippets(theme.snippets.filter((x) => x.id !== s.id));
		toasts.success("지움");
	}

	const fontSizes = ["13", "14", "15", "16", "17", "18", "20"].map((v) => ({ value: v, label: `${v}px` }));
</script>

<Section title="테마">
	<Group>
		<Row label="화면 모드" desc="시스템 설정을 따르거나 직접 고름">
			<Segmented
				label="화면 모드"
				value={theme.setting.mode}
				options={[
					{ value: "system", label: "시스템" },
					{ value: "light", label: "밝게" },
					{ value: "dark", label: "어둡게" },
				]}
				onchange={(mode) => theme.update({ mode })}
			/>
		</Row>
		<Row label="강조색" desc="버튼·선택 표시·내 메시지 색. 테마가 정하면 테마 색">
			<div class="accents" role="radiogroup" aria-label="강조색">
				{#each ACCENTS as a (a.id)}
					<button
						role="radio"
						aria-checked={theme.accentId === a.id}
						aria-label={a.label}
						title={a.label}
						class:on={theme.accentId === a.id}
						style:background={theme.dark ? a.dark : a.light}
						onclick={() => theme.update({ accentColor: a.id })}
					></button>
				{/each}
			</div>
		</Row>
		<Row label="회색 톤" desc="바탕·테두리·글자의 기본 색감">
			<div class="grays" role="radiogroup" aria-label="회색 톤">
				{#each GRAYS as g (g.id)}
					{@const s = theme.dark ? g.dark : g.light}
					<button
						role="radio"
						aria-checked={theme.setting.grayTone === g.id}
						aria-label={g.label}
						title={g.label}
						class:on={theme.setting.grayTone === g.id}
						style:background="linear-gradient(135deg, {s[2]} 0 50%, {s[10]} 50% 100%)"
						onclick={() => theme.update({ grayTone: g.id })}
					></button>
				{/each}
			</div>
		</Row>
		<Row label="글자 크기" desc="앱 전체 기본 글자 크기">
			<Select value={String(theme.setting.fontSize)} options={fontSizes} aria-label="글자 크기" onchange={(e) => theme.update({ fontSize: Number(e.currentTarget.value) })} />
		</Row>
		<Row label="글꼴" desc="아래 '글꼴 파일'에서 올린 글꼴 중에서 고름">
			<Select value={theme.setting.appFont ?? ""} options={fontOptions} aria-label="글꼴" onchange={(e) => theme.update({ appFont: e.currentTarget.value || null })} />
		</Row>
		<Row label="프레임 없는 창" desc="운영체제 제목 표시줄 대신 앱 헤더 사용">
			<Switch checked={theme.setting.isFrameless} label="프레임 없는 창" hideLabel onchange={(v) => theme.update({ isFrameless: v })} />
		</Row>
	</Group>

	<Group title="글꼴 파일">
		{#snippet aside()}<Button size="sm" onclick={uploadFonts}>{#snippet icon()}<Upload />{/snippet}글꼴 올리기</Button>{/snippet}
		<div class="bg-row">
			{#if fonts.length}
				<ul class="fonts">
					{#each fonts as f (f.path)}
						<li class:on={theme.setting.appFont === f.path}>
							<div class="font-txt">
								<b>{f.name}</b>
								<span class="sample" style:font-family={`"${fontFamilyOf(f.path)}", var(--font-sans)`}>가나다라마바사 ABC abc 123</span>
							</div>
							{#if theme.setting.appFont === f.path}
								<Button size="sm" variant="ghost" onclick={() => theme.update({ appFont: null })}>해제</Button>
							{:else}
								<Button size="sm" variant="secondary" onclick={() => theme.update({ appFont: f.path })}>사용</Button>
							{/if}
							<Button size="sm" variant="ghost" aria-label="{f.name} 삭제" onclick={() => removeFont(f.path, f.name)}>{#snippet icon()}<Trash2 />{/snippet}</Button>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="font-empty">올린 글꼴이 없음. 지금은 기본 글꼴 사용</p>
			{/if}
		</div>
	</Group>

	<Group title="배경 이미지">
		<div class="bg-row">
			<p class="bg-desc">앱 뒤에 깔 그림. 카메라로 여러 장 올려 두고 골라 씀 (움짤 GIF 가능, 앱 데이터에 복사). 더보기에서 지울 수 있음</p>
			<BackgroundPicker
				label="앱 배경"
				value={theme.setting.backgroundImage ?? null}
				images={theme.setting.backgrounds ?? []}
				onpick={(v) => theme.setBackground(v)}
				onupload={uploadBackgrounds}
				onremove={(p) => theme.removeBackground(p)}
			/>
		</div>
		{#if hasBg}
			<Row label="화면 불투명도" desc="낮을수록 배경이 잘 보이고 글은 덜 또렷함">
				<Select value={String(theme.setting.backgroundOpacity ?? 0.82)} options={opacities} aria-label="화면 불투명도" onchange={(e) => theme.update({ backgroundOpacity: Number(e.currentTarget.value) })} />
			</Row>
		{/if}
	</Group>

	<Group title="기본 테마">
		<div class="gallery" role="radiogroup" aria-label="기본 테마">
			{#each BUILTIN_THEMES as t (t.id)}
				{@const key = t.id === "default" ? "default" : `builtin:${t.id}`}
				<ThemeCard name={t.name} colors={cardColors(t.colors)} selected={selected === key} onselect={() => selectKey(key)}>
					{#snippet actions()}
						<Button size="sm" variant="secondary" onclick={() => edit(`builtin:${t.id}`)}>{#snippet icon()}<Copy />{/snippet}복제해서 편집</Button>
					{/snippet}
				</ThemeCard>
			{/each}
		</div>
	</Group>

	<Group title="내 테마">
		{#snippet aside()}<Button size="sm" onclick={() => edit("new")}>{#snippet icon()}<Plus />{/snippet}새 테마</Button>{/snippet}
		<div class="gallery">
			{#each list as t (t.filename ?? t.name)}
				<ThemeCard name={t.name} colors={cardColors(t.colors)} selected={selected === t.filename} onselect={() => selectKey(t.filename ?? "default")} oncontextmenu={(e) => themeMenu(e, t)}>
					{#snippet actions()}
						<Button size="sm" variant="secondary" onclick={() => edit(t.filename ?? "")}>{#snippet icon()}<Pencil />{/snippet}편집</Button>
						<Button size="sm" variant="secondary" aria-label="{t.name} 더 보기" onclick={(e) => openMenu(e, t)}>⋯</Button>
					{/snippet}
				</ThemeCard>
			{/each}
			<button class="new-card" onclick={() => edit("new")}><Plus />새 테마 만들기</button>
		</div>
	</Group>

	<Group title="CSS 스니펫" flush>
		{#snippet aside()}<Button size="sm" onclick={addSnippet}>{#snippet icon()}<Plus />{/snippet}새 스니펫</Button>{/snippet}
		{#each theme.snippets as s (s.id)}
			<div class="snip" class:open={openSnippet === s.id}>
				<div class="snip-head">
					<button class="snip-name" aria-expanded={openSnippet === s.id} onclick={() => (openSnippet = openSnippet === s.id ? null : s.id)}>{s.name}</button>
					<Switch checked={s.enabled} label="{s.name} 사용" hideLabel onchange={(v) => patchSnippet(s.id, { enabled: v })} />
				</div>
				{#if openSnippet === s.id}
					<div class="snip-body">
						<Input value={s.name} aria-label="스니펫 이름" onchange={(e) => patchSnippet(s.id, { name: e.currentTarget.value.trim() || "이름 없음" })} />
						<Textarea value={s.css} mono rows={8} spellcheck={false} aria-label="스니펫 CSS" onchange={(e) => patchSnippet(s.id, { css: e.currentTarget.value })} />
						<div class="snip-foot">
							<span class="faint small">입력칸에서 나가면 저장·적용됨</span>
							<Button size="sm" variant="ghost" onclick={() => removeSnippet(s)}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<p class="faint small pad">스니펫 없음</p>
		{/each}
	</Group>
</Section>

{#if menu}<ContextMenu at={menu.at} items={menu.items} label="테마 메뉴" onclose={() => (menu = null)} />{/if}

<style>
	.bg-row {
		padding: var(--space-4) 0;
	}
	.bg-desc {
		margin-bottom: var(--space-3);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.fonts {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.fonts li {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
	}
	.fonts li.on {
		border-color: var(--accent);
	}
	.font-txt {
		display: flex;
		flex: 1;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
	}
	.font-txt b {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.sample {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: var(--text-lg);
	}
	.font-empty {
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.accents,
	.grays {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.accents button,
	.grays button {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
	}
	.accents button.on,
	.grays button.on {
		box-shadow: 0 0 0 2px var(--text);
	}
	.gallery {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(190px, 45%), 1fr));
		gap: var(--space-3);
		padding: var(--space-4) 0;
	}
	.new-card {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: var(--space-1);
		min-height: 110px;
		border: 1px dashed var(--border-strong);
		border-radius: var(--radius-md);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}
	.new-card:hover {
		border-color: var(--accent);
		color: var(--text);
	}
	.new-card :global(svg) {
		width: 18px;
		height: 18px;
	}
	.small {
		font-size: var(--text-xs);
	}
	.pad {
		padding: var(--space-2) var(--space-3);
	}
	.snip {
		border-radius: var(--radius-sm);
	}
	.snip + .snip {
		border-top: 1px solid var(--border);
	}
	.snip-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) var(--space-3);
	}
	.snip-name {
		flex: 1;
		min-height: 36px;
		padding: 0;
		border: none;
		background: none;
		color: var(--text);
		font: inherit;
		font-weight: 600;
		text-align: left;
		cursor: pointer;
	}
	.snip-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0 var(--space-3) var(--space-3);
	}
	.snip-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
</style>
