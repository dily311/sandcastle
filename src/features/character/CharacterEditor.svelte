<script lang="ts">
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
	import { library, type Character } from "./library.svelte";

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
	onMount(async () => {
		books = await call(commands.getAllWorldinfoBooks(), { ...src, silent: true }).catch(() => []);
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
	const dirty = $derived(JSON.stringify(stripAvatar(input)) !== original);

	// 저장하지 않고 나가려 하면 물어봄
	$effect(() => {
		if (!dirty) return;
		return nav.setGuard(() => confirmAction({ title: "저장하지 않음", message: "바뀐 내용을 버리고 나감", confirmLabel: "버리기" }));
	});

	function close() {
		void nav.replace({ page: "character", charId: character.id });
	}
	async function save() {
		tagInput?.commit();
		if (!input.name) return toasts.error("이름을 입력해야 함");
		saving = true;
		try {
			const saved = await call(commands.updateCharacter(character.id, input), src);
			const i = library.characters.findIndex((c) => c.id === saved.id);
			if (i >= 0) library.characters[i] = saved;
			toasts.success("저장함");
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
	$effect(() => dropPaths({ priority: 5, label: "아바타로 쓰기", hint: "PNG·JPG·WEBP·GIF 한 장", multiple: false, onpaths: ([path]) => setAvatar(path) }));

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

	const SECTIONS = [
		{ id: "basic", label: "기본 정보" },
		{ id: "notes", label: "소개" },
		{ id: "prompt", label: "프롬프트" },
		{ id: "first", label: "첫 메시지" },
		{ id: "links", label: "연결" },
	];
	let body: HTMLDivElement | undefined = $state();
	function jump(id: string) {
		body?.querySelector(`#ce-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
	}
	const count = (s: string) => `${s.length.toLocaleString()}자`;
</script>

<div class="editor">
	<div class="layout">
		<nav class="toc" aria-label="편집 항목">
			{#each SECTIONS as s (s.id)}
				<button class="link" onclick={() => jump(s.id)}>{s.label}</button>
			{/each}
			<div class="actions">
				{#if dirty}<span class="dirty">바뀐 내용 있음</span>{/if}
				<div class="btns">
					<Button variant="ghost" onclick={close}>취소</Button>
					<Button variant="primary" loading={saving} disabled={!dirty || !name.trim()} onclick={save}>저장</Button>
				</div>
			</div>
		</nav>

		<div class="body" bind:this={body}>
			<section id="ce-basic">
				<h2>기본 정보</h2>
				<div class="card">
					<div class="who">
						<button class="avatar-btn" title="아바타 바꾸기" aria-label="아바타 바꾸기" onclick={() => avatar(true)}>
							<Avatar name={name || character.name} src={fileSrc(character.avatar)} color={color || null} size={88} fit={avatarFit} />
							<span class="cam"><Camera /></span>
						</button>
						<div class="who-fields">
							<Field label="이름" required hint={"대화에서 {{char}} 자리에 들어감"}><Input bind:value={name} /></Field>
							{#if character.avatar}
								<div class="avatar-acts">
									<Button size="sm" variant="ghost" onclick={() => (fitOpen = true)}>{#snippet icon()}<Crop />{/snippet}위치·확대</Button>
									<Button size="sm" variant="ghost" onclick={() => avatar(false)}>{#snippet icon()}<X />{/snippet}아바타 지우기</Button>
								</div>
								<AvatarFitDialog open={fitOpen} name={name || character.name} src={fileSrc(character.avatar)} color={color || null} fit={avatarFit} onclose={() => (fitOpen = false)} onsave={saveFit} />
							{/if}
						</div>
					</div>
					<div class="two">
						<Field label="분류" hint="목록에 이름 아래 보임"><Input bind:value={category} placeholder="예: 판타지" /></Field>
						<Field label="태그" hint="Enter나 쉼표로 추가"><TagInput bind:this={tagInput} bind:value={tags} /></Field>
					</div>
					<Field label="색" hint="아바타가 없을 때 바탕색"><ColorPicker bind:value={color} /></Field>
				</div>
			</section>

			<section id="ce-notes">
				<h2>소개</h2>
				<div class="card">
					<Field label="제작자 메모" hint="캐릭터 화면 개요에 보임. AI에게는 보내지 않음">
						{#snippet aside()}<span class="faint small">{count(creatorNotes)}</span>{/snippet}
						<Textarea bind:value={creatorNotes} rows={4} autosize maxRows={16} placeholder="캐릭터 소개, 버전 기록, 쓰는 법 등" />
					</Field>
				</div>
			</section>

			<section id="ce-prompt">
				<h2>프롬프트</h2>
				<div class="card">
					<Field label="설명" hint="AI에게 보내는 캐릭터 설정. 외모·성격·말투·배경">
						{#snippet aside()}<span class="faint small">{count(description)}</span>{/snippet}
						<Textarea bind:value={description} rows={10} autosize maxRows={30} />
					</Field>
					<Field label="대화 예시" hint={"말투를 보여주는 예시. <START>로 구분, {{char}}·{{user}} 사용 가능"}>
						<Textarea bind:value={example} rows={5} autosize maxRows={20} mono placeholder={"<START>\n{{user}}: 안녕\n{{char}}: ..."} />
					</Field>
					<Field label="시스템 프롬프트 덮어쓰기" hint="채우면 프리셋의 기본 지시(main) 블록 대신 씀. 비우면 프리셋 그대로">
						<Textarea bind:value={systemPrompt} rows={3} autosize maxRows={16} mono />
					</Field>
				</div>
			</section>

			<section id="ce-first">
				<h2>첫 메시지</h2>
				<p class="lead">새 대화를 열면 AI가 먼저 보내는 메시지. 여러 개면 다른 것은 스와이프로 고를 수 있음</p>
				<ol class="firsts">
					{#each firsts as _, i (i)}
						<li class="card">
							<div class="first-head">
								<b>{i === 0 ? "기본" : `다른 첫 메시지 ${i}`}</b>
								<span class="sp"></span>
								{#if i > 0}<Button size="sm" variant="ghost" onclick={() => makePrimary(i)}>{#snippet icon()}<ArrowUp />{/snippet}기본으로</Button>{/if}
								<IconButton label="이 첫 메시지 지우기" size="sm" onclick={() => removeFirst(i)}><Trash2 /></IconButton>
							</div>
							<Textarea bind:value={firsts[i]} rows={4} autosize maxRows={20} aria-label={i === 0 ? "기본 첫 메시지" : `다른 첫 메시지 ${i}`} />
						</li>
					{/each}
				</ol>
				<Button size="sm" onclick={addFirst}>{#snippet icon()}<Plus />{/snippet}다른 첫 메시지 추가</Button>
			</section>

			<section id="ce-links">
				<h2>연결</h2>
				<div class="card">
					<div class="two">
						<Field label="기본 페르소나" hint="이 캐릭터와 새 대화를 열 때">
							<Select bind:value={personaId} options={[{ value: "", label: "앱 기본값" }, ...library.personas.map((p) => ({ value: p.id, label: p.name }))]} />
						</Field>
						<Field label="기본 프리셋" hint="대화에서 따로 고르지 않으면">
							<Select bind:value={presetId} options={[{ value: "", label: "앱 기본값" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]} />
						</Field>
					</div>
					<Field label="월드인포" hint="이 캐릭터와 대화할 때 함께 쓰는 책">
						{#if books.length}
							<div class="chips">
								{#each books as b (b.id)}
									<button class="chip" class:on={worldInfoIds.includes(b.id)} aria-pressed={worldInfoIds.includes(b.id)} title={b.description ?? undefined} onclick={() => toggleBook(b.id)}>{b.name}</button>
								{/each}
							</div>
						{:else}
							<p class="faint small">월드인포 책 없음</p>
						{/if}
					</Field>
				</div>
			</section>
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
