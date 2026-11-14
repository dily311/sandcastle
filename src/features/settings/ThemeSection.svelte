<script lang="ts">
	import { onMount } from "svelte";
	import { Plus } from "@lucide/svelte";
	import type { CssThemeMeta_Serialize as ThemeMeta } from "$ipc/bindings";
	import { Badge, Button, Dialog, Field, Input, Segmented, Select, Switch, Textarea } from "$shared/ui";
	import { ACCENTS } from "$shared/ui/accents";
	import { theme } from "$app/theme.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "theme" };
	let list = $state<ThemeMeta[]>([]);
	async function loadList() {
		list = await call(commands.getThemeList(), { ...src, silent: true }).catch(() => []);
	}
	onMount(loadList);

	// ---- 테마 편집 ----
	let editing = $state<{ filename: string | null; name: string; author: string; version: string; css: string } | null>(null);
	let editOpen = $state(false);
	const TEMPLATE = `/* 밝은 화면 */
:root[data-theme="light"] {
	--bg: #f6f3ee;
	--surface: #fffdfa;
	--text: #2b2722;
}
/* 어두운 화면 */
:root[data-theme="dark"] {
	--bg: #171513;
	--surface: #1f1c19;
	--text: #ece6dd;
}`;

	function startNew() {
		editing = { filename: null, name: "", author: "", version: "1.0.0", css: TEMPLATE };
		editOpen = true;
	}
	async function startEdit(t: ThemeMeta) {
		const raw = await call(commands.loadThemeContent(t.filename ?? ""), src);
		editing = { filename: t.filename ?? null, name: t.name, author: t.author, version: t.version, css: raw };
		editOpen = true;
	}
	function slug(name: string) {
		const s = name.trim().toLowerCase().replace(/[^\p{L}\p{N}]+/gu, "-").replace(/^-|-$/g, "");
		return `${s || "theme"}-${Date.now().toString(36)}.json`;
	}
	async function saveEdit() {
		if (!editing || !editing.name.trim()) return;
		const filename = editing.filename ?? slug(editing.name);
		await call(
			commands.saveTheme(filename, { name: editing.name.trim(), author: editing.author.trim(), version: editing.version.trim() || "1.0.0", cssContent: editing.css }),
			src,
		);
		editOpen = false;
		await loadList();
		// 쓰는 중인 테마면 바로 다시 적용
		if (theme.setting.selectTheme === filename) theme.apply();
		toasts.success("테마 저장함");
	}
	async function remove(t: ThemeMeta) {
		if (!(await confirmAction({ title: "테마 삭제", message: `'${t.name}' 테마를 지움`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteTheme(t.filename ?? ""), src);
		if (theme.setting.selectTheme === t.filename) await theme.update({ selectTheme: "default" });
		await loadList();
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
		<Row label="강조색" desc="버튼·선택 표시·내 메시지 색">
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
		<Row label="글자 크기" desc="앱 전체 기본 글자 크기">
			<Select value={String(theme.setting.fontSize)} options={fontSizes} aria-label="글자 크기" onchange={(e) => theme.update({ fontSize: Number(e.currentTarget.value) })} />
		</Row>
		<Row label="프레임 없는 창" desc="운영체제 제목 표시줄 대신 앱 헤더 사용">
			<Switch checked={theme.setting.isFrameless} label="프레임 없는 창" hideLabel onchange={(v) => theme.update({ isFrameless: v })} />
		</Row>
	</Group>

	<Group title="사용자 테마">
		{#snippet aside()}<Button size="sm" onclick={startNew}>{#snippet icon()}<Plus />{/snippet}새 테마</Button>{/snippet}
		<Row label="기본" desc="앱 기본 색">
			{#if theme.setting.selectTheme === "default" || !theme.setting.selectTheme}
				<Badge tone="accent">사용 중</Badge>
			{:else}
				<Button size="sm" variant="soft" onclick={() => theme.update({ selectTheme: "default" })}>적용</Button>
			{/if}
		</Row>
		{#each list as t (t.filename ?? t.name)}
			<Row label={t.name} desc="v{t.version}{t.author ? ` · ${t.author}` : ''}">
				<Button size="sm" variant="ghost" onclick={() => remove(t)}>삭제</Button>
				<Button size="sm" onclick={() => startEdit(t)}>편집</Button>
				{#if theme.setting.selectTheme === t.filename}
					<Badge tone="accent">사용 중</Badge>
				{:else}
					<Button size="sm" variant="soft" onclick={() => theme.update({ selectTheme: t.filename ?? "default" })}>적용</Button>
				{/if}
			</Row>
		{/each}
	</Group>
</Section>

<Dialog bind:open={editOpen} title={editing?.filename ? "테마 편집" : "새 테마"} description="CSS 변수로 앱 색을 덮어씀. 강조색은 위에서 고른 값을 씀" size="lg">
	{#if editing}
		<div class="form">
			<div class="meta">
				<Field label="이름" required><Input bind:value={editing.name} placeholder="테마 이름" /></Field>
				<Field label="만든 사람"><Input bind:value={editing.author} /></Field>
				<Field label="버전"><Input bind:value={editing.version} /></Field>
			</div>
			<Field label="CSS" hint="주요 변수: --bg, --bg-sunken, --surface, --surface-2, --border, --text, --text-muted">
				<Textarea bind:value={editing.css} mono rows={16} spellcheck={false} />
			</Field>
		</div>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (editOpen = false)}>취소</Button>
		<Button variant="primary" disabled={!editing?.name.trim()} onclick={saveEdit}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.accents {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.accents button {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
	}
	.accents button.on {
		box-shadow: 0 0 0 2px var(--text);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.meta {
		display: grid;
		grid-template-columns: 2fr 1fr 1fr;
		gap: var(--space-3);
	}
</style>
