<script lang="ts">
	import { _ } from "svelte-i18n";
	import { Camera, Copy, Crop, MoreHorizontal, Star, Trash2, Users, X } from "@lucide/svelte";
	import type { PersonaInput_Deserialize as PersonaInput, WorldInfoBook_Serialize as Book } from "$ipc/bindings";
	import { Avatar, AvatarFitDialog, Badge, Button, ColorPicker, ContextMenu, Field, IconButton, Input, TagInput, Textarea, type MenuItem } from "$shared/ui";
	import type { AvatarFit } from "$ipc/bindings";
	import { fileSrc } from "$shared/ipc/file";
	import { relative } from "$shared/time";
	import { nav } from "$app/nav.svelte";
	import { library, type Persona } from "$features/character/library.svelte";
	import BookPicker from "../lorebook/BookPicker.svelte";

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
	let alias = $state(p0.alias ?? "");
	let description = $state(p0.description ?? "");
	let tags = $state<string[]>([...p0.tags]);
	let tagInput: ReturnType<typeof TagInput> | undefined = $state();
	let color = $state(p0.color || "");
	let worldInfoIds = $state<string[]>([...p0.worldInfoIds]);

	$effect(() => {
		dirty =
			name !== p0.name ||
			alias !== (p0.alias ?? "") ||
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
		alias = p0.alias ?? "";
		description = p0.description ?? "";
		tags = [...p0.tags];
		color = p0.color || "";
		worldInfoIds = [...p0.worldInfoIds];
	}
	function save() {
		tagInput?.commit();
		onsave({ name: name.trim(), alias: alias.trim() || null, avatar: persona.avatar, description, tags, color, worldInfoIds, isSystemDefault: persona.isSystemDefault });
	}

	// 어디서 쓰는지
	const usedBy = $derived(library.characters.filter((c) => c.defaultPersonaId === persona.id));
	const convs = $derived(library.conversations.filter((c) => c.personaId === persona.id).sort((a, b) => b.updatedAt - a.updatedAt));

	let fitOpen = $state(false);
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const menuItems: MenuItem[] = $derived([
		{ label: $_("persona.duplicateMenu"), icon: Copy, onselect: onduplicate },
		{ label: $_("persona.copyToCharMenu"), icon: Users, onselect: ontocharacter },
		...(persona.avatar
			? [
					{ label: $_("persona.avatarFitMenu"), icon: Crop, onselect: () => (fitOpen = true) },
					{ label: $_("persona.avatarClearMenu"), icon: X, onselect: () => onavatar(false) },
				]
			: []),
		{ label: isDefault ? $_("persona.deleteDefaultDisabled") : $_("common.delete"), icon: Trash2, danger: true, disabled: isDefault, separator: true, onselect: ondelete },
	]);
	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
</script>

<AvatarFitDialog open={fitOpen} name={name || persona.name} src={fileSrc(persona.avatar)} color={color || null} fit={persona.avatarFit} onclose={() => (fitOpen = false)} onsave={onavatarfit} />

<div class="editor">
	<header class="head">
		<button class="avatar-btn" title={$_("persona.changeAvatarAria")} aria-label={$_("persona.changeAvatarAria")} onclick={() => onavatar(true)}>
			<Avatar name={name || persona.name} src={fileSrc(persona.avatar)} color={color || null} size={88} fit={persona.avatarFit} />
			<span class="cam"><Camera /></span>
		</button>
		<div class="ident">
			<h1>{alias.trim() || name || $_("persona.unnamed")}</h1>
			<p class="faint">
				{$_("persona.roleLabel")}{#if alias.trim() && name} · {name}{/if}{#if tags.length} · {tags.slice(0, 4).join(" · ")}{/if}
			</p>
		</div>
		<div class="cta">
			{#if isDefault}
				<Badge tone="accent"><Star /> {$_("persona.defaultBadge")}</Badge>
			{:else}
				<Button size="sm" onclick={ondefault}>{#snippet icon()}<Star />{/snippet}{$_("persona.makeDefaultBtn")}</Button>
			{/if}
			<IconButton label={$_("persona.moreAria")} onclick={openMenu}><MoreHorizontal /></IconButton>
		</div>
	</header>

	<div class="bar" class:show={dirty}>
		<span>{$_("persona.dirtyBanner")}</span>
		<Button size="sm" variant="ghost" onclick={reset}>{$_("persona.revertBtn")}</Button>
		<Button size="sm" variant="primary" loading={saving} disabled={!name.trim()} onclick={save}>{$_("common.save")}</Button>
	</div>

	<div class="form">
		<Field label={$_("persona.nameField")} required hint={$_("persona.nameHint")}>
			<Input bind:value={name} placeholder={$_("persona.namePlaceholder")} />
		</Field>
		<Field label={$_("persona.aliasField")} hint={$_("persona.aliasHint")}>
			<Input bind:value={alias} placeholder={$_("persona.aliasPlaceholder")} />
		</Field>
		<Field label={$_("persona.descField")} hint={$_("persona.descHint")}>
			{#snippet aside()}<span class="faint count">{$_("persona.charCount", { values: { count: description.length.toLocaleString() } })}</span>{/snippet}
			<Textarea bind:value={description} rows={8} autosize maxRows={24} placeholder={$_("persona.descPlaceholder")} />
		</Field>
		<Field label={$_("persona.tagsField")} hint={$_("persona.tagsHint")}>
			<TagInput bind:this={tagInput} bind:value={tags} />
		</Field>
		<Field label={$_("persona.colorField")} hint={$_("persona.colorHint")}>
			<ColorPicker bind:value={color} />
		</Field>
		<Field label={$_("persona.wiField")} hint={$_("persona.wiHint")}>
			{#if books.length}
				<BookPicker label={$_("persona.wiPickerAria")} value={worldInfoIds ?? []} onchange={(ids) => (worldInfoIds = ids)} />
			{:else}
				<p class="faint small">{$_("persona.noWiBooks")}</p>
			{/if}
		</Field>
	</div>

	{#if usedBy.length || convs.length}
		<section class="usage">
			<h2>{$_("persona.usageTitle")}</h2>
			{#if usedBy.length}
				<div class="usage-group">
					<p class="usage-label">{$_("persona.usedByChars", { values: { count: usedBy.length } })}</p>
					<div class="usage-chips">
						{#each usedBy as c (c.id)}
							<button type="button" class="usage-chip" onclick={() => nav.go({ page: "character", charId: c.id })}>
								<Avatar name={c.name} src={fileSrc(c.avatar)} fit={c.avatarFit} color={c.color} size={20} />
								<span>{c.name}</span>
							</button>
						{/each}
					</div>
				</div>
			{/if}
			{#if convs.length}
				<div class="usage-group">
					<p class="usage-label">{$_("persona.pinnedConvs", { values: { count: convs.length } })}</p>
					<ul class="usage-list">
						{#each convs.slice(0, 8) as c (c.id)}
							{@const ch = library.character(c.characterId)}
							<li>
								<button type="button" onclick={() => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: c.id })}>
									<b>{c.title}</b>
									<small>{ch?.name ?? $_("persona.charFallback")} · {relative(c.updatedAt)}</small>
								</button>
							</li>
						{/each}
					</ul>
				</div>
			{/if}
		</section>
	{/if}
</div>

<ContextMenu at={menu} items={menuItems} label={$_("persona.menuAria")} onclose={() => (menu = null)} />

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
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
	}
	.usage {
		margin-top: var(--space-5);
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
	}
	.usage h2 {
		font-size: var(--text-md);
		font-weight: 700;
	}
	.usage-group {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.usage-label {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-muted);
	}
	.usage-chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.usage-chip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 4px 10px 4px 6px;
		border: 1px solid var(--border);
		border-radius: var(--radius-full);
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.usage-chip:hover {
		background: var(--surface-hover);
		border-color: var(--border-strong);
	}
	.usage-list {
		list-style: none;
		margin: 0;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
	}
	.usage-list li + li {
		border-top: 1px solid var(--border);
	}
	.usage-list button {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-2) var(--space-3);
		border: none;
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.usage-list button:hover {
		background: var(--surface-hover);
	}
	.usage-list b {
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.usage-list small {
		flex: none;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.count {
		font-size: var(--text-xs);
	}
	.small {
		font-size: var(--text-sm);
	}

</style>
