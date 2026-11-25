<script lang="ts">
	import { _ } from "svelte-i18n";
	import { jobs } from "$shared/stores/jobs.svelte";
	import { trashedToast } from "$shared/trash";
	import { layout } from "$app/layout.svelte";
	import { onMount } from "svelte";
	import { Copy, Plus, Search, Star, Trash2, UserCircle, Users } from "@lucide/svelte";
	import type { AvatarFit, PersonaInput_Deserialize as PersonaInput, WorldInfoBook_Serialize as Book } from "$ipc/bindings";
	import { Avatar, Button, ContextMenu, EmptyState, IconButton, Input, Spinner, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { pickFiles, IMAGE_FILES } from "$shared/ipc/dialog";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
	import { dropPaths } from "$app/drop.svelte";
	import { library, type Persona } from "$features/character/library.svelte";
	import PersonaEditor from "./PersonaEditor.svelte";

	let { personaId }: { personaId?: string } = $props();
	// 휴대폰: 페르소나를 고르기 전에는 목록만, 고르면 편집만
	const mobileList = $derived(layout.narrow && !personaId);

	const src = { source: "persona" };
	let defaultId = $state<string | null>(null);
	let books = $state<Book[]>([]);
	let saving = $state(false);
	let dirty = $state(false);
	let query = $state("");
	let ready = $state(false);
	/** 저장하면 올려서 편집기를 새 값으로 다시 시작 (아바타만 바꿀 때는 그대로 둬서 고치던 내용 유지) */
	let version = $state(0);

	onMount(async () => {
		const [conv, b] = await Promise.all([
			call(commands.getConvSettings(), { ...src, silent: true }).catch(() => null),
			call(commands.getAllWorldinfoBooks(), { ...src, silent: true }).catch(() => []),
			library.loaded ? null : library.load(),
		]);
		defaultId = conv?.defaultPersonaId ?? null;
		books = b;
		ready = true;
	});

	// 기본 → 이름 순
	const sorted = $derived(
		[...library.personas].sort((a, b) => Number(b.id === defaultId) - Number(a.id === defaultId) || a.name.localeCompare(b.name, "ko")),
	);
	const q = $derived(query.trim().toLowerCase());
	const list = $derived(q ? sorted.filter((p) => `${p.alias ?? ""} ${p.name} ${p.tags.join(" ")} ${p.description ?? ""}`.toLowerCase().includes(q)) : sorted);
	const current = $derived(library.persona(personaId) ?? (personaId ? undefined : sorted[0]));

	// 첫 진입이면 첫 페르소나를 주소에 고정 (뒤로 가기 기록은 남기지 않음)
	$effect(() => {
		if (ready && !personaId && current && !layout.narrow) nav.replace({ page: "persona", personaId: current.id });
	});

	// 고친 내용이 있으면 다른 페르소나·다른 화면으로 가기 전에 물어봄
	let releaseGuard: (() => void) | null = null;
	$effect(() => {
		if (!dirty) return;
		const release = nav.setGuard(() => confirmAction({ title: $_("persona.unsavedTitle"), message: $_("persona.unsavedLeaveMsg"), confirmLabel: $_("persona.discardBtn") }));
		releaseGuard = release;
		return () => {
			release();
			releaseGuard = null;
		};
	});
	/** 이미 물어봤거나 버려도 되는 경우 */
	function discard() {
		releaseGuard?.();
		dirty = false;
	}

	function select(id: string) {
		if (layout.narrow) return void nav.go({ page: "persona", personaId: id });
		if (id !== current?.id) void nav.replace({ page: "persona", personaId: id });
	}

	function put(p: Persona) {
		const i = library.personas.findIndex((x) => x.id === p.id);
		if (i >= 0) library.personas[i] = p;
		else library.personas = [...library.personas, p];
	}
	const blank = (name: string): PersonaInput => ({ name, avatar: null, description: "", worldInfoIds: [], tags: [], color: "", isSystemDefault: false });

	async function create() {
		if (dirty && !(await confirmAction({ title: $_("persona.unsavedTitle"), message: $_("persona.unsavedNewMsg"), confirmLabel: $_("persona.discardBtn") }))) return;
		discard();
		const name = await promptText({ title: $_("persona.newTitle"), message: $_("persona.newMsg"), input: { value: "", placeholder: $_("persona.namePlaceholder") }, confirmLabel: $_("persona.createBtn") });
		if (!name?.trim()) return;
		const p = await call(commands.createPersona(blank(name.trim())), src);
		put(p);
		discard();
		nav.replace({ page: "persona", personaId: p.id });
	}
	async function save(input: PersonaInput) {
		if (!current) return;
		saving = true;
		try {
			await call(commands.updatePersona(current.id, input), src);
			put({ ...current, ...input, avatar: current.avatar, description: input.description, updatedAt: Date.now() });
			dirty = false;
			version++;
			toasts.success($_("persona.savedToast"));
		} finally {
			saving = false;
		}
	}
	async function avatar(pick: boolean) {
		if (!current) return;
		let path: string | null = null;
		if (pick) {
			[path] = await pickFiles([IMAGE_FILES], false);
			if (!path) return;
		}
		put(await call(commands.setPersonaAvatar(current.id, path), src));
	}
	async function avatarFit(fit: AvatarFit | null) {
		if (!current) return;
		await call(commands.setAvatarFit("persona", current.id, fit), src);
		put({ ...current, avatarFit: fit });
	}
	// 끌어다 놓은 그림은 지금 보고 있는 페르소나의 아바타로 (아바타 버튼과 같은 처리)
	$effect(() => {
		const p = current;
		if (!p) return;
		return dropPaths({
			label: $_("persona.dropAvatarLabel", { values: { name: p.name } }),
			hint: $_("persona.dropAvatarHint"),
			multiple: false,
			onpaths: async ([path]) => put(await call(commands.setPersonaAvatar(p.id, path), src)),
		});
	});
	async function makeDefault() {
		if (!current) return;
		await call(commands.setSystemDefaultPersona(current.id), src);
		defaultId = current.id;
		toasts.success($_("persona.setDefaultToast", { values: { name: current.name } }));
	}
	async function duplicate() {
		if (!current) return;
		const p = await call(commands.createPersona({ ...current, name: $_("persona.copySuffix", { values: { name: current.name } }), isSystemDefault: false }), src);
		put(p);
		discard();
		nav.replace({ page: "persona", personaId: p.id });
	}
	async function toCharacter() {
		if (!current) return;
		const c = await call(commands.copyPersonaToCharacter(current.id), src);
		library.characters = [...library.characters, c];
		toasts.show({ tone: "success", message: $_("persona.copiedToCharToast", { values: { name: c.name } }), action: { label: $_("persona.openBtn"), run: () => nav.go({ page: "character", charId: c.id }) } });
	}
	async function remove() {
		if (!current) return;
		const target = current;
		const usage = await call(commands.getPersonaUsage(target.id), { ...src, silent: true }).catch(() => null);
		const convCount = library.conversations.filter((c) => c.personaId === target.id).length;
		const notes = [
			usage?.characterCount ? $_("persona.deleteCharUsage", { values: { count: usage.characterCount } }) : "",
			convCount ? $_("persona.deleteConvUsage", { values: { count: convCount } }) : "",
		].filter(Boolean);
		if (!(await confirmAction({ title: $_("persona.deleteTitle"), message: [$_("persona.deleteTrashMsg", { values: { name: target.name } }), ...notes, $_("persona.deleteRestoreNote")].join(". "), confirmLabel: $_("common.delete"), danger: true }))) return;
		const item = await jobs.run(target.name, () => call(commands.deletePersona(target.id), src), { group: $_("persona.deleteJobGroup") });
		if (!item) return;
		trashedToast(item, () => library.load());
		library.personas = library.personas.filter((p) => p.id !== target.id);
		discard();
		nav.replace({ page: "persona" });
		// 기본 페르소나·대화의 페르소나가 비워졌을 수 있음
		void library.load();
	}
	let rowMenu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function openRowMenu(e: MouseEvent, p: Persona) {
		e.preventDefault();
		const isDef = p.id === defaultId;
		rowMenu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				...(!isDef ? [{ label: $_("persona.setDefaultMenu"), icon: Star, onselect: async () => { await call(commands.setSystemDefaultPersona(p.id), src); defaultId = p.id; toasts.success($_("persona.setDefaultToast", { values: { name: p.name } })); } }] : []),
				{
					label: $_("persona.duplicateMenu"),
					icon: Copy,
					onselect: async () => {
						const copy = await call(commands.createPersona({ ...p, name: $_("persona.copySuffix", { values: { name: p.name } }), isSystemDefault: false }), src);
						put(copy);
						nav.replace({ page: "persona", personaId: copy.id });
					},
				},
				{
					label: $_("persona.copyToCharMenu"),
					icon: Users,
					onselect: async () => {
						const c = await call(commands.copyPersonaToCharacter(p.id), src);
						library.characters = [...library.characters, c];
						toasts.show({ tone: "success", message: $_("persona.copiedToCharToast", { values: { name: c.name } }), action: { label: $_("persona.openBtn"), run: () => nav.go({ page: "character", charId: c.id }) } });
					},
				},
				{
					label: isDef ? $_("persona.deleteDefaultDisabled") : $_("common.delete"),
					icon: Trash2,
					danger: true,
					disabled: isDef,
					separator: true,
					onselect: async () => {
						if (current?.id === p.id) return void remove();
						if (!(await confirmAction({ title: $_("persona.deleteTitle"), message: $_("persona.deleteTrashMsg", { values: { name: p.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
						const item = await jobs.run(p.name, () => call(commands.deletePersona(p.id), src), { group: $_("persona.deleteJobGroup") });
						if (!item) return;
						trashedToast(item, () => library.load());
						library.personas = library.personas.filter((x) => x.id !== p.id);
						void library.load();
					},
				},
			],
		};
	}
</script>

{#if !library.loaded || !ready}
	<div class="center"><Spinner /></div>
{:else}
	<div class="page" class:desktop={!layout.mobile} class:mobile={layout.mobile} class:narrow={layout.narrow} class:listing={mobileList}>
		<aside class="side" aria-label={$_("persona.listAria")}>
			<div class="top">
				<Input bind:value={query} placeholder={$_("persona.searchPlaceholder")} aria-label={$_("persona.searchAria")} type="search">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<IconButton label={$_("persona.addAria")} size="md" onclick={create}><Plus /></IconButton>
			</div>
			<p class="side-count">{$_("persona.count", { values: { count: library.personas.length } })}</p>
			<ul>
				{#each list as p (p.id)}
					<li>
						<button class:on={current?.id === p.id} aria-current={current?.id === p.id ? "page" : undefined} onclick={() => select(p.id)} oncontextmenu={(e) => openRowMenu(e, p)}>
							<Avatar name={p.name} src={fileSrc(p.avatar)} fit={p.avatarFit} color={p.color || null} size={36} />
							<span class="t">
								<b>{p.alias?.trim() || p.name}{#if p.id === defaultId}<Star class="star" aria-label={$_("persona.defaultAria")} />{/if}</b>
								<span class="faint">{p.alias?.trim() ? `${p.name} · ` : ""}{(p.description ?? "").split("\n")[0] || $_("persona.noDesc")}</span>
							</span>
						</button>
					</li>
				{:else}
					<li class="none faint">{q ? $_("persona.noResults") : $_("persona.emptyList")}</li>
				{/each}
			</ul>
		</aside>

		<main class="main">
			{#if current}
				{#key `${current.id}:${version}`}
					<PersonaEditor
						persona={current}
						isDefault={current.id === defaultId}
						{books}
						{saving}
						bind:dirty
						onsave={save}
						onavatar={avatar}
						onavatarfit={avatarFit}
						ondefault={makeDefault}
						onduplicate={duplicate}
						ontocharacter={toCharacter}
						ondelete={remove}
					/>
				{/key}
			{:else if personaId}
				<EmptyState title={$_("persona.notFoundTitle")} description={$_("persona.notFoundDesc")}>
					{#snippet action()}<Button onclick={() => nav.replace({ page: "persona" })}>{$_("persona.toList")}</Button>{/snippet}
				</EmptyState>
			{:else}
				<EmptyState title={$_("persona.emptyList")} description={$_("persona.emptyDesc")}>
					{#snippet icon()}<UserCircle />{/snippet}
					{#snippet action()}<Button variant="primary" onclick={create}>{$_("persona.newTitle")}</Button>{/snippet}
				</EmptyState>
			{/if}
		</main>
	</div>
{/if}
{#if rowMenu}<ContextMenu at={rowMenu.at} items={rowMenu.items} label={$_("persona.menuAria")} onclose={() => (rowMenu = null)} />{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.page {
		display: grid;
		grid-template-columns: 260px minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.side-count {
		margin: 0;
		padding: 0 var(--space-2);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.side {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: auto;
	}
	.top {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-1);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	li button {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	li button:hover {
		background: var(--surface-hover);
	}
	li button.on {
		background: var(--accent-soft);
	}
	.t {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.t b {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.t b,
	.t span {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.t span {
		font-size: var(--text-xs);
	}
	.t :global(.star) {
		width: 12px;
		height: 12px;
		flex-shrink: 0;
		color: var(--accent);
		fill: var(--accent);
	}
	.none {
		padding: var(--space-4) var(--space-2);
		font-size: var(--text-sm);
	}
	.main {
		overflow: auto;
		background: var(--surface-2);
	}
	/* 좁은 화면: 목록과 편집을 한 번에 하나만 (mobileList와 같은 기준 layout.narrow) */
	.page.narrow {
		grid-template-columns: minmax(0, 1fr);
	}
	.page.narrow:not(.listing) .side,
	.page.listing .main {
		display: none;
	}
	.page.listing .side {
		border-right: none;
	}
</style>
