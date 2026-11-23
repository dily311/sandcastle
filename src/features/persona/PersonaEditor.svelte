<script lang="ts">
	import { Camera, Copy, Crop, MoreHorizontal, Star, Trash2, Users, X } from "@lucide/svelte";
	import type { PersonaInput_Deserialize as PersonaInput, WorldInfoBook_Serialize as Book } from "$ipc/bindings";
	import { Avatar, AvatarFitDialog, Badge, Button, ColorPicker, ContextMenu, Field, IconButton, Input, TagInput, Textarea, type MenuItem } from "$shared/ui";
	import type { AvatarFit } from "$ipc/bindings";
	import { fileSrc } from "$shared/ipc/file";
	import { relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { library, type Persona } from "$features/character/library.svelte";

	interface Props {
		persona: Persona;
		isDefault: boolean;
		books: Book[];
		saving: boolean;
		/** 바뀐 내용이 있는지 (부모가 화면 이동 전에 물어봄) */
		dirty?: boolean;
		onsave: (input: PersonaInput) => void;
		onavatar: (pick: boolean) => void;
		/** 프로필 그림 확대·위치 저장 (5-E-17-3) */
		onavatarfit: (fit: AvatarFit | null) => void | Promise<void>;
		ondefault: () => void;
		onduplicate: () => void;
		ontocharacter: () => void;
		ondelete: () => void;
	}
	let { persona, isDefault, books, saving, dirty = $bindable(false), onsave, onavatar, onavatarfit, ondefault, onduplicate, ontocharacter, ondelete }: Props = $props();

	// 편집 사본 (persona가 바뀌면 부모가 {#key}로 새로 만듦)
	// svelte-ignore state_referenced_locally
	const p0 = persona;
	let name = $state(p0.name);
	let description = $state(p0.description ?? "");
	let tags = $state<string[]>([...p0.tags]);
	let tagInput: ReturnType<typeof TagInput> | undefined = $state();
	let color = $state(p0.color || "");
	let worldInfoIds = $state<string[]>([...p0.worldInfoIds]);

	$effect(() => {
		dirty =
			name !== p0.name ||
			description !== (p0.description ?? "") ||
			color !== (p0.color || "") ||
			tags.join("\u0000") !== p0.tags.join("\u0000") ||
			[...worldInfoIds].sort().join() !== [...p0.worldInfoIds].sort().join();
	});

	function toggleBook(id: string) {
		worldInfoIds = worldInfoIds.includes(id) ? worldInfoIds.filter((b) => b !== id) : [...worldInfoIds, id];
	}
	function reset() {
		name = p0.name;
		description = p0.description ?? "";
		tags = [...p0.tags];
		color = p0.color || "";
		worldInfoIds = [...p0.worldInfoIds];
	}
	function save() {
		tagInput?.commit();
		onsave({ name: name.trim(), avatar: persona.avatar, description, tags, color, worldInfoIds, isSystemDefault: persona.isSystemDefault });
	}

	// 어디서 쓰는지
	const usedBy = $derived(library.characters.filter((c) => c.defaultPersonaId === persona.id));
	const convs = $derived(library.conversations.filter((c) => c.personaId === persona.id).sort((a, b) => b.updatedAt - a.updatedAt));

	let fitOpen = $state(false);
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const menuItems: MenuItem[] = $derived([
		{ label: "복제", icon: Copy, onselect: onduplicate },
		{ label: "캐릭터로 복사", icon: Users, onselect: ontocharacter },
		...(persona.avatar
			? [
					{ label: "아바타 위치·확대", icon: Crop, onselect: () => (fitOpen = true) },
					{ label: "아바타 지우기", icon: X, onselect: () => onavatar(false) },
				]
			: []),
		{ label: isDefault ? "삭제 (기본은 못 지움)" : "삭제", icon: Trash2, danger: true, disabled: isDefault, separator: true, onselect: ondelete },
	]);
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
</script>

<AvatarFitDialog open={fitOpen} name={name || persona.name} src={fileSrc(persona.avatar)} color={color || null} fit={persona.avatarFit} onclose={() => (fitOpen = false)} onsave={onavatarfit} />

<div class="editor">
	<header class="head">
		<button class="avatar-btn" title="아바타 바꾸기" aria-label="아바타 바꾸기" onclick={() => onavatar(true)}>
			<Avatar name={name || persona.name} src={fileSrc(persona.avatar)} color={color || null} size={88} fit={persona.avatarFit} />
			<span class="cam"><Camera /></span>
		</button>
		<div class="ident">
			<h1>{name || "이름 없음"}</h1>
			<p class="faint">
				페르소나{#if tags.length} · {tags.slice(0, 4).join(" · ")}{/if}
			</p>
		</div>
		<div class="cta">
			{#if isDefault}
				<Badge tone="accent"><Star /> 기본 페르소나</Badge>
			{:else}
				<Button size="sm" onclick={ondefault}>{#snippet icon()}<Star />{/snippet}기본으로</Button>
			{/if}
			<IconButton label="더 보기" onclick={openMenu}><MoreHorizontal /></IconButton>
		</div>
	</header>

	<div class="bar" class:show={dirty}>
		<span>바뀐 내용 있음</span>
		<Button size="sm" variant="ghost" onclick={reset}>되돌리기</Button>
		<Button size="sm" variant="primary" loading={saving} disabled={!name.trim()} onclick={save}>저장</Button>
	</div>

	<div class="form">
		<Field label="이름" required hint={"대화에서 {{user}} 자리에 들어감"}>
			<Input bind:value={name} placeholder="이름" />
		</Field>
		<Field label="설명" hint="AI에게 보내는 내 캐릭터 소개. 외모·성격·말투·처지 등">
			{#snippet aside()}<span class="faint count">{description.length.toLocaleString()}자</span>{/snippet}
			<Textarea bind:value={description} rows={8} autosize maxRows={24} placeholder="예: 스무 살 대학생. 말수가 적지만 호기심이 많음" />
		</Field>
		<Field label="태그" hint="Enter나 쉼표로 추가. 목록 검색에 씀">
			<TagInput bind:this={tagInput} bind:value={tags} />
		</Field>
		<Field label="색" hint="아바타가 없을 때 바탕색">
			<ColorPicker bind:value={color} />
		</Field>
		<Field label="월드인포" hint="이 페르소나로 대화할 때 함께 쓰는 월드인포 책">
			{#if books.length}
				<div class="books">
					{#each books as b (b.id)}
						<button class="chip" class:on={worldInfoIds.includes(b.id)} aria-pressed={worldInfoIds.includes(b.id)} title={b.description ?? undefined} onclick={() => toggleBook(b.id)}>{b.name}</button>
					{/each}
				</div>
			{:else}
				<p class="faint small">월드인포 책 없음</p>
			{/if}
		</Field>
	</div>
</div>

<ContextMenu at={menu} items={menuItems} label="페르소나 메뉴" onclose={() => (menu = null)} />

<style>
	.editor {
		max-width: var(--page-narrow);
		margin: 0 auto;
		padding: var(--space-8) var(--space-6) var(--space-12, 48px);
	}
	.head {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-4);
		margin-bottom: var(--space-6);
	}
	.avatar-btn {
		position: relative;
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
	h1 {
		font-size: var(--text-xl);
		overflow-wrap: anywhere;
	}
	.cta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.cta :global(.badge svg) {
		width: 12px;
		height: 12px;
	}
	.bar {
		position: sticky;
		top: 0;
		z-index: var(--z-float);
		display: none;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
		padding: var(--space-2) var(--space-3);
		border: 1px solid var(--accent);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
	}
	.bar.show {
		display: flex;
	}
	.bar span {
		flex: 1;
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding: var(--space-5);
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
	}
	.count {
		font-size: var(--text-xs);
	}
	.books {
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
	.small {
		font-size: var(--text-sm);
	}

</style>
