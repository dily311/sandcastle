<script lang="ts">
	import { _ } from "svelte-i18n";
	import { onMount } from "svelte";
	import { ArrowUp, Camera, Crop, Plus, Trash2, X } from "@lucide/svelte";
	import type { CharacterInput_Deserialize as CharacterInput, WorldInfoBook_Serialize as Book } from "$ipc/bindings";
	import { Avatar, AvatarFitDialog, Button, ColorPicker, Field, IconButton, Input, Select, TagInput, Textarea } from "$shared/ui";
	import type { AvatarFit } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
	import { dropPaths } from "$app/drop.svelte";
	import { library, type Character, type VarDef } from "./library.svelte";
	import VarDefsEditor from "./VarDefsEditor.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";

	let { character }: { character: Character } = $props();
	const src = { source: "character" };

	// ---- 편집 사본 ----
	// svelte-ignore state_referenced_locally
	const c0 = character;
	let name = $state(c0.name);
	let category = $state(c0.category ?? "");
	let tags = $state<string[]>([...c0.tags]);
	let color = $state(c0.color ?? "");
	let description = $state(c0.description);
	let creatorNotes = $state(c0.creatorNotes ?? "");
	let firsts = $state<string[]>([c0.firstMessage ?? "", ...c0.alternateFirstMessage]);
	let example = $state(c0.exampleDialogue ?? "");
	let systemPrompt = $state(c0.systemPromptOverride ?? "");
	let personaId = $state(c0.defaultPersonaId ?? "");
	let presetId = $state(c0.defaultPromptPresetId ?? "");
	let worldInfoIds = $state<string[]>([...c0.worldInfoIds]);
	let tagInput: ReturnType<typeof TagInput> | undefined = $state();

	let books = $state<Book[]>([]);
	let saving = $state(false);
	// 변수 정의 (5-F-4-3): 상태창·변수 플러그인이 켜졌을 때만 읽고 보인다. 저장 버튼으로 같이 저장
	const varsOn = plugins.on("status");
	let varDefs = $state<VarDef[]>([]);
	let varDefsOriginal = $state("[]");
	// 상태창 틀 (5-F-4-5): 변수로 채워 고정 자리에 그리는 HTML
	let statusTemplate = $state("");
	let statusTemplateOriginal = $state("");
	onMount(async () => {
		books = await call(commands.getAllWorldinfoBooks(), { ...src, silent: true }).catch(() => []);
		if (varsOn) {
			varDefs = (await library.loadVarDefs(c0.id)).map((d) => ({ ...d }));
			varDefsOriginal = JSON.stringify(varDefs);
			statusTemplate = statusTemplateOriginal = (await library.loadStatusTemplate(c0.id)) ?? "";
		}
	});

	const input = $derived<CharacterInput>({
		name: name.trim(),
		avatar: character.avatar,
		description,
		alternateDescriptions: c0.alternateDescriptions,
		firstMessage: firsts[0]?.trim() ? firsts[0] : null,
		alternateFirstMessage: firsts.slice(1).filter((f) => f.trim()),
		exampleDialogue: example.trim() ? example : null,
		systemPromptOverride: systemPrompt.trim() ? systemPrompt : null,
		creatorNotes: creatorNotes.trim() ? creatorNotes : null,
		worldInfoIds,
		tags,
		color: color || null,
		category: category.trim() || null,
		defaultPersonaId: personaId || null,
		defaultPromptPresetId: presetId || null,
		favorite: character.favorite,
	});
	const original = JSON.stringify({ ...stripAvatar(toInput(c0)) });
	function toInput(c: Character): CharacterInput {
		return {
			name: c.name.trim(),
			avatar: c.avatar,
			description: c.description,
			alternateDescriptions: c.alternateDescriptions,
			firstMessage: c.firstMessage?.trim() ? c.firstMessage : null,
			alternateFirstMessage: c.alternateFirstMessage.filter((f) => f.trim()),
			exampleDialogue: c.exampleDialogue?.trim() ? c.exampleDialogue : null,
			systemPromptOverride: c.systemPromptOverride?.trim() ? c.systemPromptOverride : null,
			creatorNotes: c.creatorNotes?.trim() ? c.creatorNotes : null,
			worldInfoIds: c.worldInfoIds,
			tags: c.tags,
			color: c.color || null,
			category: c.category?.trim() || null,
			defaultPersonaId: c.defaultPersonaId || null,
			defaultPromptPresetId: c.defaultPromptPresetId || null,
			favorite: c.favorite,
		};
	}
	function stripAvatar(i: CharacterInput) {
		const { avatar: _a, favorite: _f, ...rest } = i;
		return { ...rest, worldInfoIds: [...rest.worldInfoIds].sort() };
	}
	const varDefsDirty = $derived(varsOn && (JSON.stringify(varDefs) !== varDefsOriginal || statusTemplate !== statusTemplateOriginal));
	const dirty = $derived(JSON.stringify(stripAvatar(input)) !== original || varDefsDirty);

	// 저장하지 않고 나가려 하면 물어봄
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: $_("charEditor.unsavedTitle"), message: $_("charEditor.unsavedMsg"), confirmLabel: $_("charEditor.discardBtn") }));
	});

	function close() {
		void nav.replace({ page: "character", charId: character.id });
	}
	async function save() {
		tagInput?.commit();
		if (!input.name) return toasts.error($_("charEditor.nameRequired"));
		saving = true;
		try {
			// 변수 정의를 먼저: 확인에 걸리면(이름 겹침 등) 알림을 보이고 편집에 남는다
			if (varDefsDirty) {
				varDefs = await library.setVarDefs(character.id, varDefs);
				varDefsOriginal = JSON.stringify(varDefs);
				await library.setStatusTemplate(character.id, statusTemplate);
				statusTemplateOriginal = statusTemplate;
			}
			const saved = await call(commands.updateCharacter(character.id, input), src);
			const i = library.characters.findIndex((c) => c.id === saved.id);
			if (i >= 0) library.characters[i] = saved;
			toasts.success($_("charEditor.savedToast"));
			// 저장했으니 물어보지 않고 상세로
			nav.clearGuard();
			close();
		} finally {
			saving = false;
		}
	}
	async function avatar(pick: boolean) {
		let path: string | null = null;
		if (pick) {
			[path] = await pickFiles([IMAGE_FILES], false);
			if (!path) return;
		}
		await setAvatar(path);
	}
	async function setAvatar(path: string | null) {
		const saved = await call(commands.setCharacterAvatar(character.id, path), src);
		const c = library.character(character.id);
		if (c) {
			c.avatar = saved.avatar;
			c.avatarFit = saved.avatarFit;
		}
	}
	// 프로필 그림 확대·위치 (5-E-17-3): 바로 저장 (커버 위치와 같음)
	let fitOpen = $state(false);
	const avatarFit = $derived(library.character(character.id)?.avatarFit ?? character.avatarFit ?? null);
	async function saveFit(fit: AvatarFit | null) {
		await call(commands.setAvatarFit("character", character.id, fit), src);
		const c = library.character(character.id);
		if (c) c.avatarFit = fit;
	}
	// 편집 중에 끌어다 놓은 그림은 아바타로 (캐릭터 화면의 "카드 가져오기"보다 우선)
	$effect(() => dropPaths({ priority: 5, label: $_("charEditor.dropAvatarLabel"), hint: $_("charEditor.dropAvatarHint"), multiple: false, onpaths: ([path]) => setAvatar(path) }));

	// ---- 첫 메시지 ----
	function addFirst() {
		firsts = [...firsts, ""];
	}
	function removeFirst(i: number) {
		firsts = firsts.length > 1 ? firsts.filter((_, k) => k !== i) : [""];
	}
	function makePrimary(i: number) {
		firsts = [firsts[i], ...firsts.filter((_, k) => k !== i)];
	}
	function toggleBook(id: string) {
		worldInfoIds = worldInfoIds.includes(id) ? worldInfoIds.filter((b) => b !== id) : [...worldInfoIds, id];
	}

	const SECTIONS = $derived([
		{ id: "basic", label: $_("charEditor.secBasic") },
		{ id: "notes", label: $_("charEditor.secNotes") },
		{ id: "prompt", label: $_("charEditor.secPrompt") },
		{ id: "first", label: $_("charEditor.secFirst") },
		{ id: "links", label: $_("charEditor.secLinks") },
		...(varsOn ? [{ id: "vars", label: $_("charEditor.secVars") }] : []),
	]);
	let body: HTMLDivElement | undefined = $state();
	function jump(id: string) {
		body?.querySelector(`#ce-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
	}
	const count = (s: string) => $_("charEditor.charCount", { values: { count: s.length.toLocaleString() } });
</script>

<div class="editor">
	<div class="layout">
		<nav class="toc" aria-label={$_("charEditor.tocAria")}>
			{#each SECTIONS as s (s.id)}
				<button class="link" onclick={() => jump(s.id)}>{s.label}</button>
			{/each}
			<div class="actions">
				{#if dirty}<span class="dirty">{$_("charEditor.dirtyLabel")}</span>{/if}
				<div class="btns">
					<Button variant="ghost" onclick={close}>{$_("common.cancel")}</Button>
					<Button variant="primary" loading={saving} disabled={!dirty || !name.trim()} onclick={save}>{$_("common.save")}</Button>
				</div>
			</div>
		</nav>

		<div class="body" bind:this={body}>
			<section id="ce-basic">
				<h2>{$_("charEditor.secBasic")}</h2>
				<div class="card">
					<div class="who">
						<button class="avatar-btn" title={$_("charEditor.changeAvatarAria")} aria-label={$_("charEditor.changeAvatarAria")} onclick={() => avatar(true)}>
							<Avatar name={name || character.name} src={fileSrc(character.avatar)} color={color || null} size={88} fit={avatarFit} />
							<span class="cam"><Camera /></span>
						</button>
						<div class="who-fields">
							<Field label={$_("charEditor.nameField")} required hint={$_("charEditor.nameHint")}><Input bind:value={name} /></Field>
							{#if character.avatar}
								<div class="avatar-acts">
									<Button size="sm" variant="ghost" onclick={() => (fitOpen = true)}>{#snippet icon()}<Crop />{/snippet}{$_("charEditor.avatarFitBtn")}</Button>
									<Button size="sm" variant="ghost" onclick={() => avatar(false)}>{#snippet icon()}<X />{/snippet}{$_("charEditor.avatarRemoveBtn")}</Button>
								</div>
								<AvatarFitDialog open={fitOpen} name={name || character.name} src={fileSrc(character.avatar)} color={color || null} fit={avatarFit} onclose={() => (fitOpen = false)} onsave={saveFit} />
							{/if}
						</div>
					</div>
					<div class="two">
						<Field label={$_("charEditor.catField")} hint={$_("charEditor.catHint")}><Input bind:value={category} placeholder={$_("charEditor.catPlaceholder")} /></Field>
						<Field label={$_("charEditor.tagsField")} hint={$_("charEditor.tagsHint")}><TagInput bind:this={tagInput} bind:value={tags} /></Field>
					</div>
					<Field label={$_("charEditor.colorField")} hint={$_("charEditor.colorHint")}><ColorPicker bind:value={color} /></Field>
				</div>
			</section>

			<section id="ce-notes">
				<h2>{$_("charEditor.secNotes")}</h2>
				<div class="card">
					<Field label={$_("charEditor.creatorNotesField")} hint={$_("charEditor.creatorNotesHint")}>
						{#snippet aside()}<span class="faint small">{count(creatorNotes)}</span>{/snippet}
						<Textarea bind:value={creatorNotes} rows={4} autosize maxRows={16} placeholder={$_("charEditor.creatorNotesPlaceholder")} />
					</Field>
				</div>
			</section>

			<section id="ce-prompt">
				<h2>{$_("charEditor.secPrompt")}</h2>
				<div class="card">
					<Field label={$_("charEditor.descField")} hint={$_("charEditor.descHint")}>
						{#snippet aside()}<span class="faint small">{count(description)}</span>{/snippet}
						<Textarea bind:value={description} rows={10} autosize maxRows={30} />
					</Field>
					<Field label={$_("charEditor.exampleField")} hint={$_("charEditor.exampleHint")}>
						<Textarea bind:value={example} rows={5} autosize maxRows={20} mono placeholder={$_("charEditor.examplePlaceholder")} />
					</Field>
					<Field label={$_("charEditor.sysOverrideField")} hint={$_("charEditor.sysOverrideHint")}>
						<Textarea bind:value={systemPrompt} rows={3} autosize maxRows={16} mono />
					</Field>
				</div>
			</section>

			<section id="ce-first">
				<h2>{$_("charEditor.secFirst")}</h2>
				<p class="lead">{$_("charEditor.firstLead")}</p>
				<ol class="firsts">
					{#each firsts as _f, i (i)}
						<li class="card">
							<div class="first-head">
								<b>{i === 0 ? $_("charEditor.firstDefault") : $_("charEditor.firstAlt", { values: { num: i } })}</b>
								<span class="sp"></span>
								{#if i > 0}<Button size="sm" variant="ghost" onclick={() => makePrimary(i)}>{#snippet icon()}<ArrowUp />{/snippet}{$_("charEditor.makeDefaultBtn")}</Button>{/if}
								<IconButton label={$_("charEditor.deleteFirstAria")} size="sm" onclick={() => removeFirst(i)}><Trash2 /></IconButton>
							</div>
							<Textarea bind:value={firsts[i]} rows={4} autosize maxRows={20} aria-label={i === 0 ? $_("charEditor.defaultFirstAria") : $_("charEditor.firstAlt", { values: { num: i } })} />
						</li>
					{/each}
				</ol>
				<Button size="sm" onclick={addFirst}>{#snippet icon()}<Plus />{/snippet}{$_("charEditor.addAltFirstBtn")}</Button>
			</section>

			<section id="ce-links">
				<h2>{$_("charEditor.secLinks")}</h2>
				<div class="card">
					<div class="two">
						<Field label={$_("charEditor.defaultPersonaField")} hint={$_("charEditor.defaultPersonaHint")}>
							<Select bind:value={personaId} options={[{ value: "", label: $_("charEditor.appDefault") }, ...library.personas.map((p) => ({ value: p.id, label: p.name }))]} />
						</Field>
						<Field label={$_("charEditor.defaultPresetField")} hint={$_("charEditor.defaultPersonaHint")}>
							<Select bind:value={presetId} options={[{ value: "", label: $_("charEditor.appDefault") }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]} />
						</Field>
					</div>
					<Field label={$_("charEditor.wiField")} hint={$_("charEditor.wiHint")}>
						{#if books.length}
							<div class="chips">
								{#each books as b (b.id)}
									<button class="chip" class:on={worldInfoIds.includes(b.id)} aria-pressed={worldInfoIds.includes(b.id)} title={b.description ?? undefined} onclick={() => toggleBook(b.id)}>{b.name}</button>
								{/each}
							</div>
						{:else}
							<p class="faint small">{$_("charEditor.noWiBooks")}</p>
						{/if}
					</Field>
				</div>
			</section>
			{#if varsOn}
				<section id="ce-vars">
					<h2>{$_("charEditor.secVars")}</h2>
					<div class="card">
						<VarDefsEditor bind:defs={varDefs} />
						<Field label={$_("charEditor.statusTplField")} hint={$_("charEditor.statusTplHint")}>
							<Textarea bind:value={statusTemplate} rows={6} placeholder={'<div>HP {{getvar::hp}} / 100</div>'} />
						</Field>
					</div>
				</section>
			{/if}
		</div>
	</div>
</div>

<style>
	.editor {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	.actions {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin-top: auto;
		padding-top: var(--space-3);
	}
	.btns {
		display: flex;
		gap: var(--space-2);
	}
	.btns > :global(*) {
		flex: 1;
	}
	.dirty {
		font-size: var(--text-sm);
		color: var(--accent-text, var(--accent));
	}
	.layout {
		flex: 1;
		min-height: 0;
		display: grid;
		grid-template-columns: 200px minmax(0, 1fr);
	}
	.toc {
		display: flex;
		flex-direction: column;
		gap: 2px;
		padding: var(--space-4) var(--space-3);
		border-right: 1px solid var(--border);
		background: var(--bg);
	}
	.toc .link {
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: none;
		color: var(--text-muted);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.toc .link:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.body {
		overflow: auto;
		padding: var(--space-6) var(--space-6) 30vh;
		background: var(--surface-2);
	}
	section {
		max-width: var(--page-narrow);
		margin: 0 auto var(--space-8);
		scroll-margin-top: var(--space-4);
	}
	h2 {
		margin-bottom: var(--space-3);
		font-size: var(--text-lg);
	}
	.lead {
		margin: calc(var(--space-2) * -1) 0 var(--space-3);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}
	.two {
		display: grid;
		grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
		gap: var(--space-3);
	}
	.who {
		display: flex;
		align-items: flex-start;
		gap: var(--space-4);
	}
	.who-fields {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: var(--space-2);
	}
	.who-fields > :global(:first-child) {
		align-self: stretch;
	}
	/* 세로로 긴 그림이어도 둥근 모양 그대로 (flex에 늘어나지 않게) */
	.avatar-acts {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.avatar-btn {
		position: relative;
		flex: none;
		align-self: flex-start;
		width: 88px;
		height: 88px;
		overflow: hidden;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: none;
		cursor: pointer;
	}
	.cam {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		border-radius: 50%;
		background: rgb(0 0 0 / 0.45);
		color: #fff;
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.avatar-btn:hover .cam,
	.avatar-btn:focus-visible .cam {
		opacity: 1;
	}
	.cam :global(svg) {
		width: 22px;
		height: 22px;
	}
	.firsts {
		list-style: none;
		margin: 0 0 var(--space-3);
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.firsts .card {
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.first-head {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		font-size: var(--text-sm);
	}
	.sp {
		flex: 1;
	}
	.small {
		font-size: var(--text-xs);
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.chip {
		height: 28px;
		padding: 0 var(--space-3);
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.chip.on {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--text);
	}
	@media (max-width: 860px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}
		/* 좁으면 목차는 숨기고 저장 줄만 아래에 */
		.layout {
			grid-template-rows: minmax(0, 1fr) auto;
		}
		.toc {
			order: 2;
			padding: var(--space-2) var(--space-3);
			border-right: none;
			border-top: 1px solid var(--border);
		}
		.toc .link {
			display: none;
		}
		.actions {
			margin-top: 0;
			flex-direction: row;
			align-items: center;
			justify-content: flex-end;
		}
		.two {
			grid-template-columns: minmax(0, 1fr);
		}
	}
</style>
