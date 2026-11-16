<script lang="ts">
	import { onMount } from "svelte";
	import { BookOpen, Check, ChevronsUpDown, Copy, Download, Ellipsis, Gamepad2, House, Pencil, Pin, PinOff, Plus, Settings, Smartphone, Trash2, UserCog, Users, Images } from "@lucide/svelte";
	import { Avatar, ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { call, commands } from "$shared/ipc/client";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { convActions } from "$features/character/actions";
	import { nav, type Page } from "./nav.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";

	// 화면이 늘어나면 여기에 추가 (페르소나는 아래 사용자 프로필에서)
	// 폰·게임은 코어 플러그인이 꺼져 있으면 숨김
	const allItems: { page: Page; label: string; icon: typeof Users; plugin?: PluginId }[] = [
		{ page: "home", label: "홈", icon: House },
		{ page: "character", label: "캐릭터", icon: Users },
		{ page: "phone", label: "폰", icon: Smartphone, plugin: "phone" },
		{ page: "game", label: "게임", icon: Gamepad2, plugin: "game" },
		{ page: "lorebook", label: "로어북", icon: BookOpen },
		{ page: "images", label: "이미지", icon: Images },
	];
	const items = $derived(allItems.filter((i) => !i.plugin || plugins.on(i.plugin)));

	$effect(() => {
		if (!library.loaded) void library.load();
	});

	const chats = (pinned: boolean) =>
		library.conversations
			.filter((c) => c.mode !== "phone" && c.pinned === pinned && library.character(c.characterId))
			.sort((a, b) => b.updatedAt - a.updatedAt)
			.slice(0, 10);
	// 고정된 대화 / 최근 대화 (폰 대화 제외)
	const pinned = $derived(chats(true));
	const recent = $derived(chats(false));
	const cur = $derived(nav.current);
	const openConv = $derived(cur.page === "character" ? cur.convId : undefined);

	// ---- 대화 동작 메뉴 (… 버튼 / 오른쪽 클릭) ----
	let menu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	function convMenu(c: Conversation, at: { x: number; y: number; align?: "start" | "end" }) {
		menu = {
			at,
			items: [
				{
					label: "이름 바꾸기",
					icon: Pencil,
					onselect: async () => {
						const t = await promptText({ title: "대화 이름", input: { value: c.title, placeholder: "대화 이름" }, confirmLabel: "저장" });
						if (t?.trim()) await library.rename(c.id, t);
					},
				},
				{ label: c.pinned ? "고정 풀기" : "고정", icon: c.pinned ? PinOff : Pin, onselect: () => library.togglePin(c.id) },
				{
					label: "복제",
					icon: Copy,
					onselect: async () => {
						const d = await library.duplicate(c.id);
						if (d) nav.go({ page: "character", charId: d.characterId ?? undefined, convId: d.id });
					},
				},
				{ label: "내보내기…", icon: Download, onselect: () => convActions.exportMenu(c) },
				{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => convActions.remove(c) },
			],
		};
	}
	function openMenu(e: MouseEvent, c: Conversation) {
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		convMenu(c, { x: r.right, y: r.bottom + 4, align: "end" });
	}
	function rightClick(e: MouseEvent, c: Conversation) {
		e.preventDefault();
		convMenu(c, { x: e.clientX, y: e.clientY });
	}

	// ---- 사용자 프로필: 대화를 보는 중이면 그 대화의 페르소나, 아니면 앱 기본 페르소나 ----
	let defaultPersonaId = $state<string | null>(null);
	onMount(async () => {
		const s = await call(commands.getConvSettings(), { source: "persona", silent: true }).catch(() => null);
		defaultPersonaId = s?.defaultPersonaId ?? null;
	});
	const openConvObj = $derived(library.conversation(openConv));
	const me = $derived(library.persona(openConvObj ? openConvObj.personaId : defaultPersonaId) ?? library.personas[0]);
	const scope = $derived(openConvObj ? "이 대화" : "기본");

	let profileOpen = $state(false);
	// 사이드바가 overflow:hidden이라 팝업은 화면 기준(fixed)으로 띄움
	let meBtn: HTMLButtonElement | undefined = $state();
	let popStyle = $state("");
	function toggleProfile() {
		profileOpen = !profileOpen;
		if (!profileOpen || !meBtn) return;
		const r = meBtn.getBoundingClientRect();
		popStyle = `left:${r.left}px;bottom:${window.innerHeight - r.top + 6}px;max-height:${r.top - 12}px`;
	}
	async function choose(id: string) {
		profileOpen = false;
		const c = openConvObj;
		if (c) {
			await library.setOptions(c.id, { personaId: id, promptPresetId: c.promptPresetId, apiProfileId: c.apiProfileId });
		} else {
			await call(commands.setSystemDefaultPersona(id), { source: "persona" });
			defaultPersonaId = id;
		}
		toasts.success(`${library.persona(id)?.name ?? "페르소나"}(으)로 바꿈`);
	}
	async function addPersona() {
		profileOpen = false;
		const name = await promptText({ title: "새 페르소나", message: "대화에서 내가 될 인물", input: { value: "", placeholder: "이름" }, confirmLabel: "만들기" });
		if (!name?.trim()) return;
		const p = await call(commands.createPersona({ name: name.trim(), avatar: null, description: "", worldInfoIds: [], tags: [], color: "", isSystemDefault: false }), { source: "persona" });
		library.personas = [...library.personas, p];
		nav.go({ page: "persona", personaId: p.id });
	}
</script>

<svelte:window
	onclick={(e) => {
		if (profileOpen && !(e.target as HTMLElement).closest(".profile")) profileOpen = false;
	}}
	onkeydown={(e) => e.key === "Escape" && (profileOpen = false)}
/>

{#snippet row(c: Conversation)}
	{@const ch = library.character(c.characterId)}
	<li class="row" class:on={openConv === c.id}>
		<button class="open" title="{ch?.name} · {c.title}" onclick={() => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: c.id })} oncontextmenu={(e) => rightClick(e, c)}>
			<Avatar name={ch?.name ?? "?"} src={fileSrc(ch?.avatar)} color={ch?.color} size={22} />
			<span class="t">{c.title}</span>
			<span class="when">{relative(c.updatedAt)}</span>
		</button>
		<button class="more" aria-label="{c.title} 메뉴" title="메뉴" onclick={(e) => openMenu(e, c)}><Ellipsis /></button>
	</li>
{/snippet}

<aside class="sidebar" aria-label="앱 메뉴">
	<nav aria-label="화면">
		{#each items as it (it.page)}
			<button class="item" class:on={cur.page === it.page} aria-current={cur.page === it.page ? "page" : undefined} onclick={() => nav.go({ page: it.page } as never)}>
				<it.icon />{it.label}
			</button>
		{/each}
	</nav>

	<div class="lists">
		{#if pinned.length}
			<p class="group">고정됨</p>
			<ul>
				{#each pinned as c (c.id)}{@render row(c)}{/each}
			</ul>
		{/if}
		{#if recent.length}
			<p class="group">최근 대화</p>
			<ul>
				{#each recent as c (c.id)}{@render row(c)}{/each}
			</ul>
		{/if}
	</div>

	<div class="bottom">
		<div class="profile">
			<button bind:this={meBtn} class="me" class:on={cur.page === "persona"} aria-haspopup="listbox" aria-expanded={profileOpen} title="사용자 프로필 바꾸기" onclick={toggleProfile}>
				{#if me}<Avatar name={me.name} src={fileSrc(me.avatar)} color={me.color || null} size={26} />{/if}
				<span class="who"><b>{me?.name ?? "페르소나 없음"}</b><small>{scope} 프로필</small></span>
				<ChevronsUpDown />
			</button>
			{#if profileOpen}
				<div class="pop" role="listbox" aria-label="사용자 프로필" style={popStyle}>
					<p class="pop-head">{scope === "이 대화" ? "이 대화에서 쓸 프로필" : "새 대화에 쓸 기본 프로필"}</p>
					<ul>
						{#each library.personas as p (p.id)}
							<li>
								<button role="option" aria-selected={p.id === me?.id} onclick={() => choose(p.id)}>
									<Avatar name={p.name} src={fileSrc(p.avatar)} color={p.color || null} size={24} />
									<span class="t">{p.name}</span>
									{#if p.id === me?.id}<Check />{/if}
								</button>
							</li>
						{/each}
					</ul>
					<div class="pop-foot">
						<button onclick={addPersona}><Plus />추가</button>
						<button
							onclick={() => {
								profileOpen = false;
								nav.go({ page: "persona", personaId: me?.id });
							}}><UserCog />관리</button
						>
					</div>
				</div>
			{/if}
		</div>
		<IconButton label="설정" active={cur.page === "settings"} aria-current={cur.page === "settings" ? "page" : undefined} onclick={() => nav.go({ page: "settings" })}>
			<Settings />
		</IconButton>
	</div>
</aside>

{#if menu}<ContextMenu at={menu.at} items={menu.items} label="대화 메뉴" onclose={() => (menu = null)} />{/if}

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		width: 220px;
		height: 100%;
		padding: var(--space-2);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: hidden;
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.item {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 32px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
	}
	.item :global(svg) {
		width: 16px;
		height: 16px;
	}
	.item:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.item.on {
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
	}
	.item.on :global(svg) {
		color: var(--accent);
	}
	.lists {
		flex: 1;
		min-height: 0;
		overflow: auto;
	}
	.group {
		margin: var(--space-4) var(--space-2) var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	/* 대화 한 줄: 평소엔 시각, 마우스를 올리면 그 자리에 … 버튼 */
	.row {
		position: relative;
		border-radius: var(--radius-sm);
	}
	.row:hover,
	.row:focus-within {
		background: var(--surface-hover);
	}
	.row.on {
		background: var(--surface-active);
	}
	.open {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.row:hover .open,
	.row.on .open {
		color: var(--text);
	}
	.t {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.when {
		flex: none;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.more {
		position: absolute;
		top: 3px;
		right: 4px;
		display: none;
		place-items: center;
		width: 24px;
		height: 24px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.more :global(svg) {
		width: 15px;
		height: 15px;
	}
	.more:hover {
		background: var(--surface-active);
		color: var(--text);
	}
	.row:hover .more,
	.row:focus-within .more {
		display: grid;
	}
	.row:hover .when,
	.row:focus-within .when {
		visibility: hidden;
	}
	.bottom {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding-top: var(--space-2);
		border-top: 1px solid var(--border);
	}
	.profile {
		position: relative;
		flex: 1;
		min-width: 0;
	}
	.me {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 40px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.me:hover,
	.me[aria-expanded="true"],
	.me.on {
		background: var(--surface-hover);
	}
	.me > :global(svg) {
		width: 14px;
		height: 14px;
		color: var(--text-faint);
		flex-shrink: 0;
	}
	.who {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		line-height: 1.2;
	}
	.who b,
	.who small {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.who b {
		font-size: var(--text-sm);
	}
	.who small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.pop {
		position: fixed;
		z-index: var(--z-popover);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		width: 240px;
		padding: var(--space-1);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.pop-head {
		padding: var(--space-2) var(--space-2) var(--space-1);
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.pop ul {
		max-height: 260px;
		overflow: auto;
	}
	.pop li button,
	.pop-foot button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 34px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.pop li button:hover,
	.pop-foot button:hover {
		background: var(--surface-hover);
	}
	.pop li button :global(svg) {
		width: 15px;
		height: 15px;
		color: var(--accent);
	}
	.pop-foot {
		display: flex;
		gap: 2px;
		margin-top: var(--space-1);
		padding-top: var(--space-1);
		border-top: 1px solid var(--border);
	}
	.pop-foot button {
		justify-content: center;
		color: var(--text-muted);
	}
	.pop-foot :global(svg) {
		width: 15px;
		height: 15px;
	}
</style>
