<script lang="ts">
	import { trashedToast } from "$shared/trash";
	import { layout } from "$app/layout.svelte";
	import { onMount } from "svelte";
	import { Plus, Search, Star, UserCircle } from "@lucide/svelte";
	import type { PersonaInput_Deserialize as PersonaInput, WorldInfoBook_Serialize as Book } from "$ipc/bindings";
	import { Avatar, Button, EmptyState, IconButton, Input, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { fileSrc } from "$shared/ipc/file";
	import { pickFiles, IMAGE_FILES } from "$shared/ipc/dialog";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { nav } from "$app/nav.svelte";
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
	const list = $derived(q ? sorted.filter((p) => `${p.name} ${p.tags.join(" ")} ${p.description ?? ""}`.toLowerCase().includes(q)) : sorted);
	const current = $derived(library.persona(personaId) ?? (personaId ? undefined : sorted[0]));

	// 첫 진입이면 첫 페르소나를 주소에 고정 (뒤로 가기 기록은 남기지 않음)
	$effect(() => {
		if (ready && !personaId && current && !layout.narrow) nav.replace({ page: "persona", personaId: current.id });
	});

	// 고친 내용이 있으면 다른 페르소나·다른 화면으로 가기 전에 물어봄
	let releaseGuard: (() => void) | null = null;
	$effect(() => {
		if (!dirty) return;
		const release = nav.setGuard(() => confirmAction({ title: "저장하지 않음", message: "바뀐 내용을 버리고 나감", confirmLabel: "버리기" }));
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
		if (dirty && !(await confirmAction({ title: "저장하지 않음", message: "바뀐 내용을 버리고 새 페르소나를 만듦", confirmLabel: "버리기" }))) return;
		discard();
		const name = await promptText({ title: "새 페르소나", message: "대화에서 내가 될 인물", input: { value: "", placeholder: "이름" }, confirmLabel: "만들기" });
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
			toasts.success("저장함");
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
	async function makeDefault() {
		if (!current) return;
		await call(commands.setSystemDefaultPersona(current.id), src);
		defaultId = current.id;
		toasts.success(`'${current.name}'을(를) 기본으로 정함`);
	}
	async function duplicate() {
		if (!current) return;
		const p = await call(commands.createPersona({ ...current, name: `${current.name} 사본`, isSystemDefault: false }), src);
		put(p);
		discard();
		nav.replace({ page: "persona", personaId: p.id });
	}
	async function toCharacter() {
		if (!current) return;
		const c = await call(commands.copyPersonaToCharacter(current.id), src);
		library.characters = [...library.characters, c];
		toasts.show({ tone: "success", message: `캐릭터 '${c.name}' 만듦`, action: { label: "열기", run: () => nav.go({ page: "character", charId: c.id }) } });
	}
	async function remove() {
		if (!current) return;
		const target = current;
		const usage = await call(commands.getPersonaUsage(target.id), { ...src, silent: true }).catch(() => null);
		const convCount = library.conversations.filter((c) => c.personaId === target.id).length;
		const notes = [
			usage?.characterCount ? `캐릭터 ${usage.characterCount}개의 기본 페르소나가 비워짐` : "",
			convCount ? `대화 ${convCount}개는 페르소나 없이 남음` : "",
		].filter(Boolean);
		if (!(await confirmAction({ title: "페르소나 삭제", message: [`'${target.name}'을(를) 휴지통으로 옮김`, ...notes, "되살리면 다시 이어짐"].join(". "), confirmLabel: "삭제", danger: true }))) return;
		const item = await call(commands.deletePersona(target.id), src);
		trashedToast(item, () => library.load());
		library.personas = library.personas.filter((p) => p.id !== target.id);
		discard();
		nav.replace({ page: "persona" });
		// 기본 페르소나·대화의 페르소나가 비워졌을 수 있음
		void library.load();
	}
</script>

{#if !library.loaded || !ready}
	<div class="center"><Spinner /></div>
{:else}
	<div class="page" class:desktop={!layout.mobile} class:mobile={layout.mobile} class:narrow={layout.narrow} class:listing={mobileList}>
		<aside class="side" aria-label="페르소나 목록">
			<div class="top">
				<Input bind:value={query} placeholder="이름·태그 검색" aria-label="페르소나 검색" type="search">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<IconButton label="페르소나 추가" size="md" onclick={create}><Plus /></IconButton>
			</div>
			<ul>
				{#each list as p (p.id)}
					<li>
						<button class:on={current?.id === p.id} aria-current={current?.id === p.id ? "page" : undefined} onclick={() => select(p.id)}>
							<Avatar name={p.name} src={fileSrc(p.avatar)} color={p.color || null} size={36} />
							<span class="t">
								<b>{p.name}{#if p.id === defaultId}<Star class="star" aria-label="기본" />{/if}</b>
								<span class="faint">{(p.description ?? "").split("\n")[0] || "설명 없음"}</span>
							</span>
						</button>
					</li>
				{:else}
					<li class="none faint">{q ? "검색 결과 없음" : "페르소나 없음"}</li>
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
						ondefault={makeDefault}
						onduplicate={duplicate}
						ontocharacter={toCharacter}
						ondelete={remove}
					/>
				{/key}
			{:else if personaId}
				<EmptyState title="페르소나를 찾을 수 없음" description="지워졌을 수 있음">
					{#snippet action()}<Button onclick={() => nav.replace({ page: "persona" })}>목록으로</Button>{/snippet}
				</EmptyState>
			{:else}
				<EmptyState title="페르소나 없음" description="대화에서 내가 될 인물을 만들어 둠">
					{#snippet icon()}<UserCircle />{/snippet}
					{#snippet action()}<Button variant="primary" onclick={create}>새 페르소나</Button>{/snippet}
				</EmptyState>
			{/if}
		</main>
	</div>
{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.page {
		display: grid;
		grid-template-columns: 280px minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
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
		background: var(--surface-active);
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
