<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { ChevronRight, Copy, Pencil, Plus, Trash2, Upload } from "@lucide/svelte";
	import { pickFiles } from "$shared/ipc/dialog";
	import { drops, isImageAvif, storeFile } from "$app/drop.svelte";
	import BackgroundPicker from "$shared/ui/BackgroundPicker.svelte";
	import type { AvatarFrameShape, CssThemeMeta_Serialize as ThemeMeta } from "$ipc/bindings";
	import { Avatar, Button, ContextMenu, Input, Segmented, Select, Slider, Switch, Textarea, type MenuItem } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { ACCENTS, accentValue, isHexColor } from "$shared/ui/accents";
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
	import ScrimColorPicker from "./ScrimColorPicker.svelte";
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
		const paths = await pickFiles([{ name: $_("settings.more.imgFilter"), extensions: ["png", "jpg", "jpeg", "webp", "gif", "avif"] }], true);
		await theme.addBackgrounds(paths, true);
	}
	// ---- 글꼴 요약 (상세 추가·지정은 별도 글꼴 관리 페이지에서) ----
	const fonts = $derived(theme.setting.fonts ?? []);
	const fontName = (path: string | null | undefined, fallback: string) => (path ? fonts.find((f) => f.path === path)?.name ?? fallback : fallback);
	const fontSummary = $derived(
		$_("settings.more.fontSummary", {
			values: {
				ui: fontName(theme.setting.appFont, $_("settings.more.default")),
				text: fontName(theme.setting.textFont, $_("settings.more.followUi")),
				mono: fontName(theme.setting.monoFont, $_("settings.more.default")),
			},
		}),
	);
	// ---- 아바타 프레임 (캐릭터 / 유저 페르소나 별도 설정) ----
	const FRAME_OPTIONS = $derived<{ value: AvatarFrameShape; label: string }[]>([
		{ value: "none", label: $_("settings.themeSec.frameNone") },
		{ value: "rect", label: $_("settings.themeSec.frameRect") },
		{ value: "square", label: $_("settings.themeSec.frameSquare") },
		{ value: "circle", label: $_("settings.themeSec.frameCircle") },
		{ value: "custom", label: $_("settings.themeSec.frameCustom") },
	]);
	async function uploadFrame(target: "char" | "user") {
		const [path] = await pickFiles([{ name: $_("settings.more.frameImgFilter"), extensions: ["png", "webp", "gif", "avif", "svg", "jpg", "jpeg"] }], false);
		if (!path) return;
		await theme.uploadAvatarFrame(target, path);
		toasts.success(target === "char" ? $_("settings.more.charFrameApplied") : $_("settings.more.userFrameApplied"));
	}
	// 이 화면에 놓은 그림은 배경으로
	$effect(() =>
		drops.register({
			label: $_("settings.more.dropBgLabel"),
			hint: $_("settings.more.dropBgHint"),
			accept: isImageAvif,
			ondrop: async (files) => {
				await theme.addBackgrounds(await Promise.all(files.map((f) => storeFile(f))), true);
			},
		}),
	);
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
	const selectKey = (key: string) => theme.update({ selectTheme: key });

	// ---- 사용자 테마 메뉴 ----
	let menu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function themeMenu(e: MouseEvent, t: ThemeMeta) {
		e.preventDefault();
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{ label: $_("common.edit"), icon: Pencil, onselect: () => edit(t.filename ?? "") },
				{ label: $_("common.duplicate"), icon: Copy, onselect: () => duplicate(t) },
				{ label: $_("common.delete"), icon: Trash2, danger: true, separator: true, onselect: () => remove(t) },
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
		await call(
			commands.saveTheme(`theme-${Date.now().toString(36)}.json`, {
				name: $_("settings.more.copySuffix", { values: { name: full.name } }),
				author: full.author,
				version: full.version,
				cssContent: full.cssContent,
				colors: full.colors,
				accentColor: full.accentColor,
				grayTone: full.grayTone,
				chat: full.chat,
			}),
			src,
		);
		await loadList();
	}
	async function remove(t: ThemeMeta) {
		if (!(await confirmAction({ title: $_("settings.more.deleteThemeTitle"), message: $_("settings.more.deleteThemeMsg", { values: { name: t.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.deleteTheme(t.filename ?? ""), src);
		if (selected === t.filename) await theme.update({ selectTheme: "default" });
		await loadList();
	}

	// ---- 스니펫 ----
	let openSnippet = $state<string | null>(null);
	async function addSnippet() {
		const s: CssSnippet = { id: `s-${Date.now().toString(36)}`, name: $_("settings.more.newSnippetName"), css: "/* e.g. slightly larger chat text */\n.block p {\n\tfont-size: 1.05em;\n}", enabled: false };
		await theme.saveSnippets([...theme.snippets, s]);
		openSnippet = s.id;
	}
	async function patchSnippet(id: string, p: Partial<CssSnippet>) {
		await theme.saveSnippets(theme.snippets.map((s) => (s.id === id ? { ...s, ...p } : s)));
	}
	async function removeSnippet(s: CssSnippet) {
		if (!(await confirmAction({ title: $_("settings.more.deleteSnippetTitle"), message: $_("settings.more.deleteSnippetMsg", { values: { name: s.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await theme.saveSnippets(theme.snippets.filter((x) => x.id !== s.id));
		toasts.success($_("settings.more.deletedToast"));
	}

	const fontSizes = ["13", "14", "15", "16", "17", "18", "20"].map((v) => ({ value: v, label: `${v}px` }));
</script>

<Section title={$_("settings.themeSec.title")}>
	<Group>
		<Row label={$_("settings.themeSec.mode")} desc={$_("settings.themeSec.modeDesc")}>
			<Segmented
				label={$_("settings.themeSec.mode")}
				value={theme.setting.mode}
				options={[
					{ value: "system", label: $_("settings.themeSec.modeSystem") },
					{ value: "light", label: $_("settings.themeSec.modeLight") },
					{ value: "dark", label: $_("settings.themeSec.modeDark") },
				]}
				onchange={(mode) => theme.update({ mode })}
			/>
		</Row>
		<Row label={$_("settings.themeSec.accent")} desc={$_("settings.themeSec.accentDesc")}>
			<div class="accents" role="radiogroup" aria-label={$_("settings.themeSec.accent")}>
				{#each ACCENTS as a (a.id)}
					{@const alabel = $_(`uiPolish.accents.${a.id}`, { default: a.label })}
					<button
						role="radio"
						aria-checked={theme.accentId === a.id}
						aria-label={alabel}
						title={alabel}
						class:on={theme.accentId === a.id}
						style:background={theme.dark ? a.dark : a.light}
						onclick={() => theme.update({ accentColor: a.id })}
					></button>
				{/each}
				<!-- 팔레트 밖의 색 (5-E-14) -->
				<input
					type="color"
					class="custom"
					class:on={isHexColor(theme.setting.accentColor)}
					value={accentValue(theme.accentId, theme.dark)}
					aria-label={$_("settings.more.accentCustomAria")}
					title={$_("settings.more.accentCustomTitle")}
					onchange={(e) => theme.update({ accentColor: e.currentTarget.value })}
				/>
			</div>
		</Row>
		<Row label={$_("settings.themeSec.grayTone")} desc={$_("settings.themeSec.grayToneDesc")}>
			<div class="grays" role="radiogroup" aria-label={$_("settings.themeSec.grayTone")}>
				{#each GRAYS as g (g.id)}
					{@const s = theme.dark ? g.dark : g.light}
					{@const glabel = $_(`uiPolish.grays.${g.id}`, { default: g.label })}
					<button
						role="radio"
						aria-checked={theme.setting.grayTone === g.id}
						aria-label={glabel}
						title={glabel}
						class:on={theme.setting.grayTone === g.id}
						style:background="linear-gradient(135deg, {s[2]} 0 50%, {s[10]} 50% 100%)"
						onclick={() => theme.update({ grayTone: g.id })}
					></button>
				{/each}
			</div>
		</Row>
		<Row label={$_("settings.themeSec.fontSize")} desc={$_("settings.themeSec.fontSizeDesc")}>
			<Select value={String(theme.setting.fontSize)} options={fontSizes} aria-label={$_("settings.themeSec.fontSize")} onchange={(e) => theme.update({ fontSize: Number(e.currentTarget.value) })} />
		</Row>
		<Row label={$_("settings.themeSec.font")} desc={fontSummary}>
			<Button size="sm" variant="secondary" onclick={() => nav.go({ page: "settings", tab: "theme", edit: "fonts" })}>{$_("settings.themeSec.manageFonts", { values: { count: fonts.length } })}{#snippet icon()}<ChevronRight />{/snippet}</Button>
		</Row>
		<Row label={$_("settings.themeSec.frameless")} desc={$_("settings.themeSec.framelessDesc")}>
			<Switch checked={theme.setting.isFrameless} label={$_("settings.themeSec.frameless")} hideLabel onchange={(v) => theme.update({ isFrameless: v })} />
		</Row>
	</Group>

	<Group title={$_("settings.themeSec.chatDisplayGroup")}>
		<Row label={$_("settings.themeSec.showTimestamps")} desc={$_("settings.themeSec.showTimestampsDesc")} nowrap>
			<Switch checked={theme.setting.chat?.showTimestamps ?? true} label={$_("settings.themeSec.showTimestamps")} hideLabel onchange={(v) => theme.updateChat({ showTimestamps: v })} />
		</Row>
		<Row label={$_("settings.themeSec.showAvatars")} desc={$_("settings.themeSec.showAvatarsDesc")} nowrap>
			<Switch checked={theme.setting.chat?.showAvatars ?? true} label={$_("settings.themeSec.showAvatars")} hideLabel onchange={(v) => theme.updateChat({ showAvatars: v })} />
		</Row>
		<Row label={$_("settings.themeSec.charFrame")} desc={$_("settings.themeSec.charFrameDesc")}>
			<div class="frame-ctrl">
				{#if (theme.setting.chat?.charAvatarFrame ?? "circle") !== "none"}
					<Avatar
						name="C"
						size={28}
						shape={(theme.setting.chat?.charAvatarFrame ?? "circle") as Exclude<AvatarFrameShape, "none">}
						frameSrc={theme.setting.chat?.charAvatarFrame === "custom" ? fileSrc(theme.setting.chat?.charAvatarFrameImage) : null}
					/>
				{/if}
				<Select
					value={theme.setting.chat?.charAvatarFrame ?? "circle"}
					options={FRAME_OPTIONS}
					aria-label={$_("settings.themeSec.charFrame")}
					onchange={(e) => theme.updateChat({ charAvatarFrame: e.currentTarget.value as AvatarFrameShape })}
				/>
				{#if theme.setting.chat?.charAvatarFrame === "custom"}
					<Button size="sm" variant="secondary" onclick={() => uploadFrame("char")}>{#snippet icon()}<Upload />{/snippet}{theme.setting.chat?.charAvatarFrameImage ? $_("settings.themeSec.changeFrame") : $_("settings.themeSec.uploadFrame")}</Button>
				{/if}
			</div>
		</Row>
		<Row label={$_("settings.themeSec.userFrame")} desc={$_("settings.themeSec.userFrameDesc")}>
			<div class="frame-ctrl">
				{#if (theme.setting.chat?.userAvatarFrame ?? "circle") !== "none"}
					<Avatar
						name="U"
						size={28}
						shape={(theme.setting.chat?.userAvatarFrame ?? "circle") as Exclude<AvatarFrameShape, "none">}
						frameSrc={theme.setting.chat?.userAvatarFrame === "custom" ? fileSrc(theme.setting.chat?.userAvatarFrameImage) : null}
					/>
				{/if}
				<Select
					value={theme.setting.chat?.userAvatarFrame ?? "circle"}
					options={FRAME_OPTIONS}
					aria-label={$_("settings.themeSec.userFrame")}
					onchange={(e) => theme.updateChat({ userAvatarFrame: e.currentTarget.value as AvatarFrameShape })}
				/>
				{#if theme.setting.chat?.userAvatarFrame === "custom"}
					<Button size="sm" variant="secondary" onclick={() => uploadFrame("user")}>{#snippet icon()}<Upload />{/snippet}{theme.setting.chat?.userAvatarFrameImage ? $_("settings.themeSec.changeFrame") : $_("settings.themeSec.uploadFrame")}</Button>
				{/if}
			</div>
		</Row>
		<Row label={$_("settings.themeSec.multiCharAvatars")} desc={$_("settings.themeSec.multiCharAvatarsDesc")} nowrap>
			<Switch checked={theme.setting.chat?.showMultiCharAvatars ?? false} label={$_("settings.themeSec.multiCharAvatars")} hideLabel onchange={(v) => theme.updateChat({ showMultiCharAvatars: v })} />
		</Row>
	</Group>

	<Group title={$_("settings.themeSec.bgGroup")}>
		<div class="bg-row">
			<p class="bg-desc">{$_("settings.themeSec.bgDesc")}</p>
			<BackgroundPicker
				label={$_("settings.themeSec.bgGroup")}
				value={theme.setting.backgroundImage ?? null}
				images={theme.setting.backgrounds ?? []}
				onpick={(v) => theme.setBackground(v)}
				onupload={uploadBackgrounds}
				onremove={(p) => theme.removeBackground(p)}
			/>
		</div>
		{#if hasBg}
			<div class="bg-scrim-row">
				<Slider
					label={$_("settings.themeSec.bgScrim")}
					hint={$_("settings.themeSec.bgScrimHint")}
					min={0}
					max={85}
					step={5}
					fallback={35}
					unsetLabel="35%"
					bind:value={bgScrimVal, setBgScrimDraft}
					oncommit={commitBgScrim}
				/>
			</div>
			<Row label={$_("settings.themeSec.bgScrimColor")} desc={$_("settings.themeSec.bgScrimColorDesc")}>
				<ScrimColorPicker />
			</Row>
		{/if}
	</Group>

	<Group title={$_("settings.themeSec.builtinGroup")}>
		<div class="gallery" role="radiogroup" aria-label={$_("settings.themeSec.builtinGroup")}>
			{#each BUILTIN_THEMES as t (t.id)}
				{@const key = t.id === "default" ? "default" : `builtin:${t.id}`}
				<ThemeCard name={$_(`uiPolish.builtins.${t.id}`, { default: t.name })} colors={cardColors(t.colors)} selected={selected === key} onselect={() => selectKey(key)}>
					{#snippet actions()}
						<Button size="sm" variant="secondary" onclick={() => edit(`builtin:${t.id}`)}>{#snippet icon()}<Copy />{/snippet}{$_("settings.themeSec.cloneEdit")}</Button>
					{/snippet}
				</ThemeCard>
			{/each}
		</div>
	</Group>

	<Group title={$_("settings.themeSec.myThemesGroup")}>
		{#snippet aside()}<Button size="sm" onclick={() => edit("new")}>{#snippet icon()}<Plus />{/snippet}{$_("settings.themeSec.newTheme")}</Button>{/snippet}
		<div class="gallery">
			{#each list as t (t.filename ?? t.name)}
				<ThemeCard name={t.name} colors={cardColors(t.colors)} selected={selected === t.filename} onselect={() => selectKey(t.filename ?? "default")} oncontextmenu={(e) => themeMenu(e, t)}>
					{#snippet actions()}
						<Button size="sm" variant="secondary" onclick={() => edit(t.filename ?? "")}>{#snippet icon()}<Pencil />{/snippet}{$_("common.edit")}</Button>
						<Button size="sm" variant="secondary" aria-label="{t.name} {$_('common.more')}" onclick={(e) => openMenu(e, t)}>⋯</Button>
					{/snippet}
				</ThemeCard>
			{/each}
			<button class="new-card" onclick={() => edit("new")}><Plus />{$_("settings.themeSec.createTheme")}</button>
		</div>
	</Group>

	<Group title={$_("settings.themeSec.snippetsGroup")} flush>
		{#snippet aside()}<Button size="sm" onclick={addSnippet}>{#snippet icon()}<Plus />{/snippet}{$_("settings.themeSec.newSnippet")}</Button>{/snippet}
		{#each theme.snippets as s (s.id)}
			<div class="snip" class:open={openSnippet === s.id}>
				<div class="snip-head">
					<button class="snip-name" aria-expanded={openSnippet === s.id} onclick={() => (openSnippet = openSnippet === s.id ? null : s.id)}>{s.name}</button>
					<Switch checked={s.enabled} label={s.name} hideLabel onchange={(v) => patchSnippet(s.id, { enabled: v })} />
				</div>
				{#if openSnippet === s.id}
					<div class="snip-body">
						<Input value={s.name} aria-label="Snippet name" onchange={(e) => patchSnippet(s.id, { name: e.currentTarget.value.trim() || "Untitled" })} />
						<Textarea value={s.css} mono rows={8} spellcheck={false} aria-label="Snippet CSS" onchange={(e) => patchSnippet(s.id, { css: e.currentTarget.value })} />
						<div class="snip-foot">
							<span class="faint small">{$_("settings.themeSec.snippetHint")}</span>
							<Button size="sm" variant="ghost" onclick={() => removeSnippet(s)}>{#snippet icon()}<Trash2 />{/snippet}{$_("common.delete")}</Button>
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<p class="faint small pad">{$_("settings.themeSec.noSnippets")}</p>
		{/each}
	</Group>
</Section>

{#if menu}<ContextMenu at={menu.at} items={menu.items} label={$_("settings.more.themeMenuAria")} onclose={() => (menu = null)} />{/if}

<style>
	.frame-ctrl {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.bg-row {
		padding: var(--space-4) 0;
	}
	.bg-desc {
		margin-bottom: var(--space-3);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.bg-scrim-row {
		padding: var(--space-3) 0;
		border-top: 1px solid var(--border);
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
	.accents input.custom {
		width: 24px;
		height: 24px;
		padding: 0;
		border: 2px solid var(--bg-sunken);
		border-radius: 50%;
		box-shadow: 0 0 0 1px var(--border-strong);
		background: conic-gradient(red, yellow, lime, cyan, blue, magenta, red);
		cursor: pointer;
	}
	.accents input.custom.on {
		box-shadow: 0 0 0 2px var(--text);
		background: none;
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
