<script lang="ts">
	import { lastAt } from "$features/character/library.svelte";
	import { BookOpen, Check, ChevronsUpDown, Copy, Download, ArrowUpDown, ChevronDown, Ellipsis, Gamepad2, GitBranch, House, Pencil, Pin, PinOff, Plus, Settings, Smartphone, Trash2, UserCog, Users, Images, ListFilter } from "@lucide/svelte";
	import { Avatar, ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { call, commands } from "$shared/ipc/client";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { relative } from "$shared/time";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { convActions } from "$features/character/actions";
	import { branchOf } from "$features/character/branches";
	import { nav, type Page } from "./nav.svelte";
	import { layout, SIDEBAR_MAX, SIDEBAR_MIN } from "./layout.svelte";
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
// ---- 대화 목록: 고정됨(접기) + 그룹화·하위 그룹화·필터·정렬·표시 메뉴 + 갈래 트리 ----
	type SortBy = "recent" | "created" | "name";
	type GroupBy = "none" | "character";
	type GroupSortBy = "recent" | "name";
	const GROUP_SORT_LABEL: Record<GroupSortBy, string> = {
		recent: "최근 대화순",
		name: "이름순",
	};
	type SubGroupBy = "none" | "forked";
	const SORT_LABEL: Record<SortBy, string> = { recent: "마지막 업데이트", created: "생성 날짜", name: "이름" };
	const SUBGROUP_LABEL: Record<SubGroupBy, string> = { none: "없음", forked: "갈래 (forked)" };
	interface Pref {
		pinnedOpen: boolean;
		recentOpen: boolean;
		sortBy: SortBy;
		desc: boolean;
		group: GroupBy;
		subGroup: SubGroupBy;
		groupSortBy: GroupSortBy;
		groupSortDesc: boolean;
		/** 이 캐릭터의 대화만 (null = 전부) */
		char: string | null;
		showAvatar: boolean;
		showForks: boolean;
	}
	const PREF = "sc.sidebar.list";
	const DEFAULT_PREF: Pref = {
		pinnedOpen: true,
		recentOpen: true,
		sortBy: "recent",
		desc: true,
		group: "none",
		subGroup: "none",
		groupSortBy: "name",
		groupSortDesc: true,
		char: null,
		showAvatar: true,
		showForks: true,
	};
	function readPref(): Pref {
		try {
			const p = JSON.parse(localStorage.getItem(PREF) ?? "{}");
			return {
				pinnedOpen: p.pinnedOpen !== false,
				recentOpen: p.recentOpen !== false,
				sortBy: p.sortBy in SORT_LABEL ? p.sortBy : "recent",
				desc: p.desc !== false,
				group: p.group === "character" ? "character" : "none",
				subGroup: p.subGroup === "forked" ? "forked" : "none",
				groupSortBy: p.groupSortBy,
				groupSortDesc: p.groupSortDesc,
				char: typeof p.char === "string" ? p.char : null,
				showAvatar: p.showAvatar !== false,
				showForks: p.showForks !== false,
			};
		} catch {
			return { ...DEFAULT_PREF };
		}
	}
	const pref0 = readPref();
	let pinnedOpen = $state(pref0.pinnedOpen);
	let recentOpen = $state(pref0.recentOpen);
	let sortBy = $state<SortBy>(pref0.sortBy);
	let desc = $state(pref0.desc);
	let group = $state<GroupBy>(pref0.group);
	let subGroup = $state<SubGroupBy>(pref0.subGroup);
	let groupSortBy = $state<GroupSortBy>(pref0.groupSortBy);
	let groupSortDesc = $state(pref0.groupSortDesc);
	let charFilter = $state<string | null>(pref0.char);
	let showAvatar = $state(pref0.showAvatar);
	let showForks = $state(pref0.showForks);
	$effect(() => {
		try {
			localStorage.setItem(
				PREF,
				JSON.stringify({
					pinnedOpen,
					recentOpen,
					sortBy,
					desc,
					group,
					subGroup,
					groupSortBy,
					groupSortDesc,
					char: charFilter,
					showAvatar,
					showForks,
				} satisfies Pref),
			);
		} catch {
			/* 기억하지 못해도 동작은 같음 */
		}
	});
	// 지워진 캐릭터를 거르고 있으면 풀기
	$effect(() => {
		if (library.loaded && charFilter && !library.character(charFilter)) charFilter = null;
	});
	/** 기본 보기와 다른가 (정렬 단추 강조, 재설정 가능 여부) */
	const customized = $derived(
		sortBy !== "recent" || !desc || group !== "none" || subGroup !== "none" || charFilter !== null || !showAvatar || !showForks,
	);
	function resetView() {
		({ sortBy, desc, group, subGroup, showAvatar, showForks } = { ...DEFAULT_PREF });
		charFilter = null;
		shown = PAGE;
		groupShown = {};
	}

	// 폰 대화는 폰 화면에서. 20개씩 더 보기
	const PAGE = 20;
	let shown = $state(PAGE);
	const base = $derived(library.conversations.filter((c) => c.mode !== "phone" && library.character(c.characterId) && (!charFilter || c.characterId === charFilter)));
	const cmp = (a: Conversation, b: Conversation) => {
		const v = sortBy === "name" ? a.title.localeCompare(b.title, "ko") : sortBy === "created" ? a.createdAt - b.createdAt : lastAt(a) - lastAt(b);
		// 이름은 오름차순이 "작은 것부터"라 날짜와 방향이 같게(내림차순 = 큰 쪽부터) 뒤집는다
		return desc ? -v : v;
	};
	const pinned = $derived(base.filter((c) => c.pinned).sort(cmp));
	const unpinned = $derived(base.filter((c) => !c.pinned));
	/** 이 대화에서 갈라져 나온 대화 수 (고정된 것 포함) */
	const forkCount = $derived.by(() => {
		const m = new Map<string, number>();
		for (const c of base) {
			const b = branchOf(c);
			if (b) m.set(b.parentId, (m.get(b.parentId) ?? 0) + 1);
		}
		return m;
	});
	/** 하위 그룹화가 'forked'일 때만 부모 밑에 들어가는 갈래 (부모가 같은 목록에 있을 때만, 갈라진 메시지 순) */
	const kids = $derived.by(() => {
		const m = new Map<string, Conversation[]>();
		if (subGroup !== "forked") return m;
		const ids = new Set(unpinned.map((c) => c.id));
		for (const c of unpinned) {
			const b = branchOf(c);
			if (b && ids.has(b.parentId)) m.set(b.parentId, [...(m.get(b.parentId) ?? []), c]);
		}
		for (const list of m.values()) list.sort((a, b) => (branchOf(a)?.seq ?? 0) - (branchOf(b)?.seq ?? 0) || a.createdAt - b.createdAt);
		return m;
	});
	const roots = $derived.by(() => {
		if (subGroup !== "forked") return [...unpinned].sort(cmp);
		const inTree = new Set([...kids.values()].flat().map((c) => c.id));
		return unpinned.filter((c) => !inTree.has(c.id)).sort(cmp);
	});
	const rootsShown = $derived(roots.slice(0, shown));
	/** 그룹화: 묶음마다 따로 몇 개를 보이는지 (없으면 shown) */
	let groupShown = $state<Record<string, number>>({});
	const limitOf = (id: string) => groupShown[id] ?? shown;
	/** 그룹화(캐릭터): 보이는 대화를 캐릭터별 묶음으로. 묶음은 가장 최신 메시지 순 */
	const sections = $derived.by(() => {
		const m = new Map<string, Conversation[]>();
		for (const c of roots) m.set(c.characterId, [...(m.get(c.characterId) ?? []), c]);
		return [...m.entries()]
			.map(([id, list]) => ({
				id,
				name: library.character(id)?.name ?? "캐릭터",
				list,
				latestAt: Math.max(...list.map((c) => lastAt(c))),
			}))
			.sort((a, b) => groupSortBy === "name" ? a.name.localeCompare(b.name, "ko") : b.latestAt - a.latestAt || a.name.localeCompare(b.name, "ko"));
	});
	let folded = $state<Set<string>>(new Set());
	function toggleFold(id: string) {
		const next = new Set(folded);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		folded = next;
	}

	/** 오른쪽 위 단추의 메뉴: 그룹화 기준 · 하위 그룹화 · 필터 · 정렬 기준 · 표시 · 재설정 (한 단계 하위 메뉴) */
	/** 정렬·보기 단추가 눌린 상태 (메뉴가 열려 있는 동안만 강조) */
	let viewOpen = $state(false);
	function viewMenu(e: MouseEvent) {
		viewOpen = true;
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const pick = (fn: () => void) => () => {
			fn();
			shown = PAGE;
			groupShown = {};
		};
		menu = {
			at: { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{
					label: "그룹화 기준",
					value: group === "character" ? "캐릭터" : "없음",
					children: [
						{ label: "없음", checked: group === "none", onselect: pick(() => (group = "none")) },
						{ label: "캐릭터", checked: group === "character", onselect: pick(() => (group = "character")) },
					],
				},
				{
					label: "하위 그룹화",
					value: SUBGROUP_LABEL[subGroup],
					visible: group !== "none",
					children: [
						{ id: "none", label: "없음", checked: subGroup === "none", onselect: pick(() => (subGroup = "none")) },
						{ id: "forked", label: "갈래 (forked)", checked: subGroup === "forked", onselect: pick(() => (subGroup = "forked")) },
					],
				},
				{
					label: "그룹화 정렬",
					value: GROUP_SORT_LABEL[groupSortBy],
					visible: group !== "none",
					children: [
						...(Object.keys(GROUP_SORT_LABEL) as GroupSortBy[]).map((k) => ({ id: k, label: GROUP_SORT_LABEL[k], checked: sortBy === k, onselect: () => (sortBy = k) })),
						{ id: "desc", label: "내림차순", separator: true, checked: groupSortDesc, onselect: () => (groupSortDesc = true) },
						{ id: "asc", label: "오름차순", checked: !groupSortDesc, onselect: () => (groupSortDesc = false) },
					]
				},
				{
					label: "필터",
					value: charFilter ? (library.character(charFilter)?.name ?? "캐릭터") : "전체 캐릭터",
					children: [
						{ id: "all", label: "전체 캐릭터", checked: !charFilter, onselect: pick(() => (charFilter = null)) },
						// 이름이 같은 캐릭터가 있어도 구분되도록 id로 키를 잡는다
						...[...library.characters].sort((a, b) => a.name.localeCompare(b.name, "ko")).map((c) => ({ id: c.id, label: c.name, checked: charFilter === c.id, onselect: pick(() => (charFilter = c.id)) })),
					],
				},
				{
					label: "정렬 기준",
					value: SORT_LABEL[sortBy],
					children: [
						...(Object.keys(SORT_LABEL) as SortBy[]).map((k) => ({ id: k, label: SORT_LABEL[k], checked: sortBy === k, onselect: () => (sortBy = k) })),
						{ id: "desc", label: "내림차순", separator: true, checked: desc, onselect: () => (desc = true) },
						{ id: "asc", label: "오름차순", checked: !desc, onselect: () => (desc = false) },
					],
				},
				{
					label: "표시",
					children: [
						{ id: "avatar", label: "캐릭터 이미지", checked: showAvatar, onselect: () => (showAvatar = !showAvatar) },
						{ id: "forks", label: "갈래 수", checked: showForks, onselect: () => (showForks = !showForks) },
					],
				},
				{ label: "기본값으로 재설정", separator: true, disabled: !customized, onselect: resetView },
				{ label: "모두 보기", disabled: !charFilter && shown >= roots.length, onselect: () => ((charFilter = null), (groupShown = {}), (shown = Number.MAX_SAFE_INTEGER)) },
			],
		};
	}
	const cur = $derived(nav.current);
	const openConv = $derived(cur.page === "character" ? cur.convId : undefined);
	const charName = $derived(charFilter ? (library.character(charFilter)?.name ?? "캐릭터") : "전체 캐릭터");

	// ---- 너비 조절: 오른쪽 가장자리를 끌기 (←/→ 키로도) ----
	let dragging = $state(false);
	function startResize(e: PointerEvent) {
		e.preventDefault();
		const startX = e.clientX;
		const startW = layout.sidebarWidth;
		dragging = true;
		const move = (ev: PointerEvent) => layout.setSidebarWidth(startW + (ev.clientX - startX));
		const up = () => {
			layout.setSidebarWidth(layout.sidebarWidth, true);
			dragging = false;
			window.removeEventListener("pointermove", move);
			window.removeEventListener("pointerup", up);
			document.body.style.cursor = "";
		};
		document.body.style.cursor = "col-resize";
		window.addEventListener("pointermove", move);
		window.addEventListener("pointerup", up);
	}
	function resizeKey(e: KeyboardEvent) {
		const d = e.key === "ArrowLeft" ? -16 : e.key === "ArrowRight" ? 16 : 0;
		if (!d) return;
		e.preventDefault();
		layout.setSidebarWidth(layout.sidebarWidth + d, true);
	}

	/** 새 대화: 거르는 캐릭터(없으면 지금 연 대화의 캐릭터)로. 정할 수 없으면 캐릭터 화면으로 */
	async function newConversation() {
		const charId = charFilter ?? (library.conversation(openConv)?.characterId || null);
		if (!charId) return nav.go({ page: "character" });
		const c = await library.createConversation(charId);
		nav.go({ page: "character", charId, convId: c.id });
	}

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

	// ---- 사용자 프로필: 언제나 전역 기본 페르소나 (ADR-028). 자물쇠를 건 대화는 그 대화의 것을 쓴다 ----
	const openConvObj = $derived(library.conversation(openConv));
	const me = $derived(library.persona(library.defaultPersonaId) ?? library.personas[0]);

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
		await library.setDefaultPersona(id);
		// 열린 대화가 다른 프로필로 잠겨 있으면 그 대화는 그대로임을 알린다
		const c = openConvObj;
		const locked = c && c.mode !== "phone" && c.personaId && c.personaId !== id ? library.persona(c.personaId)?.name : null;
		toasts.success(`${library.persona(id)?.name ?? "페르소나"}(으)로 바꿈${locked ? ` · 이 대화는 '${locked}'(으)로 고정됨` : ""}`);
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

{#snippet row(c: Conversation, depth: number)}
	{@const ch = library.character(c.characterId)}
	{@const sub = kids.get(c.id) ?? []}
	{@const forks = forkCount.get(c.id) ?? 0}
	<li class="row" class:on={openConv === c.id} style:--depth={depth}>
		<button class="open" title="{ch?.name} · {c.title} · {relative(lastAt(c))}" onclick={() => nav.go({ page: "character", charId: c.characterId ?? undefined, convId: c.id })} oncontextmenu={(e) => rightClick(e, c)}>
			{#if showAvatar}<Avatar name={ch?.name ?? "?"} src={fileSrc(ch?.avatar)} fit={ch?.avatarFit} color={ch?.color} size={18} />{/if}
			<span class="t">{c.title}</span>
		</button>
		{#if sub.length}
			<button class="fold" aria-label={folded.has(c.id) ? "갈래 펼치기" : "갈래 접기"} aria-expanded={!folded.has(c.id)} onclick={() => toggleFold(c.id)}><ChevronDown class={folded.has(c.id) ? "rot" : ""} /></button>
		{/if}
		{#if showForks && (forks || branchOf(c))}
			<span class="fork" title={forks ? `갈래 ${forks}개` : "다른 대화에서 갈라져 나온 대화"}><GitBranch />{#if forks}{forks}{/if}</span>
		{/if}
		<button class="more" aria-label="{c.title} 메뉴" title="메뉴" onclick={(e) => openMenu(e, c)}><Ellipsis /></button>
	</li>
	{#if sub.length && !folded.has(c.id)}
		{#each sub as k (k.id)}{@render row(k, depth + 1)}{/each}
	{/if}
{/snippet}

<aside class="sidebar sc-sidebar" aria-label="앱 메뉴" style:--sidebar-width="{layout.sidebarWidth}px">
	<nav aria-label="화면">
		{#each items as it (it.page)}
			<button class="item" class:on={cur.page === it.page} aria-current={cur.page === it.page ? "page" : undefined} onclick={() => nav.go({ page: it.page } as never)}>
				<it.icon />{it.label}
			</button>
		{/each}
	</nav>

	<div class="lists">
		{#if pinned.length}
			<div class="pinnedList">
				<button class="group head" aria-expanded={pinnedOpen} onclick={() => (pinnedOpen = !pinnedOpen)}>고정됨<ChevronDown class={pinnedOpen ? "" : "rot"} /></button>
				{#if pinnedOpen}
					<ul class="pinned">
						{#each pinned as c (c.id)}{@render row(c, 0)}{/each}
					</ul>
				{/if}
			</div>
		{/if}
		<!-- 필터·새 대화 단추: 묶음 없이 보면 "최근 대화" 머리글 옆, 그룹화하면 첫 묶음 머리글 옆 -->
		{#snippet tools()}
			<IconButton label="그룹화·필터·정렬·표시" size="sm" active={viewOpen} onclick={viewMenu}><ListFilter /></IconButton>
			<IconButton label="새 대화" size="sm" onclick={newConversation}><Plus /></IconButton>
		{/snippet}
		{#if group !== "character" || !sections.length}
			<div class="listbar">
				<button class="group head" aria-expanded={recentOpen} title={charFilter ? `${charName}의 대화만 보는 중` : ""} onclick={() => (recentOpen = !recentOpen)}>
					<span class="gn">{charFilter ? charName : "최근 대화"}</span><ChevronDown class={recentOpen ? "" : "rot"} />
				</button>
				<span class="sp"></span>
				{@render tools()}
			</div>
		{/if}
		{#if group === "character" || recentOpen}
			<ul class="tree">
				{#if group === "character"}
					<!-- 묶음 하나가 li 하나: 머리글은 그 묶음 안에서만 위에 붙고, 단추는 묶음에 마우스를 올렸을 때 보인다 (첫 묶음은 늘) -->
					{#each sections as sec, i (sec.id)}
						{@const n = limitOf(sec.id)}
						<li class="grp" class:first={i === 0}>
							<div class="gh">
								<button class="group head" aria-expanded={!folded.has(`g:${sec.id}`)} title="{sec.name} · {sec.list.length}개" onclick={() => toggleFold(`g:${sec.id}`)}>
									<span class="gn">{sec.name}</span><ChevronDown class={folded.has(`g:${sec.id}`) ? "rot" : ""} />
								</button>
								<span class="sp"></span>
								<span class="tools">{@render tools()}</span>
							</div>
							{#if !folded.has(`g:${sec.id}`)}
								<ul class="gl">
									{#each sec.list.slice(0, n) as c (c.id)}{@render row(c, 0)}{/each}
									{#if sec.list.length > n}
										<li><button class="show-more" onclick={() => (groupShown[sec.id] = (groupShown[sec.id] ?? shown) + PAGE)}>더 보기 <small>({sec.list.length - n}개 남음)</small></button></li>
									{/if}
								</ul>
							{/if}
						</li>
					{/each}
					{#if !sections.length}<li class="empty">대화가 없음</li>{/if}
				{:else}
					{#each rootsShown as c (c.id)}{@render row(c, 0)}{/each}
					{#if !rootsShown.length}
						<li class="empty">대화가 없음</li>
					{:else if roots.length > rootsShown.length}
						<li><button class="show-more" onclick={() => (shown += PAGE)}>더 보기 <small>({roots.length - rootsShown.length}개 남음)</small></button></li>
					{/if}
				{/if}
			</ul>
		{/if}
	</div>

	<div class="bottom">
		<div class="profile">
			<button bind:this={meBtn} class="me" class:on={cur.page === "persona"} aria-haspopup="listbox" aria-expanded={profileOpen} title="사용자 프로필 바꾸기" onclick={toggleProfile}>
				{#if me}<Avatar name={me.name} src={fileSrc(me.avatar)} fit={me.avatarFit} color={me.color || null} size={26} />{/if}
				<span class="who"><b>{me?.alias?.trim() || me?.name || "페르소나 없음"}</b><small>기본 프로필</small></span>
				<ChevronsUpDown />
			</button>
			{#if profileOpen}
				<div class="pop" role="listbox" aria-label="사용자 프로필" style={popStyle}>
					<p class="pop-head">대화의 기본 프로필 (자물쇠를 건 대화는 그대로)</p>
					<ul>
						{#each library.personas as p (p.id)}
							<li>
								<button role="option" aria-selected={p.id === me?.id} onclick={() => choose(p.id)}>
									<Avatar name={p.name} src={fileSrc(p.avatar)} fit={p.avatarFit} color={p.color || null} size={24} />
									<span class="t">{p.alias?.trim() || p.name}</span>
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
								// 좁은 화면은 목록과 편집을 한 번에 하나만 보여서, 특정 프로필을 주면 목록을 건너뛰고 편집만 뜸
								nav.go({ page: "persona", personaId: layout.narrow ? undefined : me?.id });
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
	{#if !layout.narrow}
		<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (포커스 가능한 separator는 조절 가능한 위젯) -->
		<div
			class="grip"
			class:drag={dragging}
			role="separator"
			aria-orientation="vertical"
			aria-label="사이드바 너비 조절 (←/→)"
			aria-valuemin={SIDEBAR_MIN}
			aria-valuemax={SIDEBAR_MAX}
			aria-valuenow={layout.sidebarWidth}
			tabindex="0"
			onpointerdown={startResize}
			onkeydown={resizeKey}
		></div>
	{/if}
</aside>

{#if menu}<ContextMenu at={menu.at} items={menu.items} label="대화 메뉴" onclose={() => ((menu = null), (viewOpen = false))} />{/if}

<style>
	.sidebar {
		display: flex;
		flex-direction: column;
		position: relative;
		width: var(--sidebar-width, 220px);
		flex: none;
		height: 100%;
		padding: 0;
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: hidden;
	}
	/* 좌우 여백 (사이드바 바닥 줄·메뉴·머리글) */
	nav,
	.bottom,
	.listbar {
		display: flex;
		align-items: center;
		gap: 2px;
		margin: var(--space-3) 0 var(--space-1);
	}
	/* 캐릭터별 묶음(li.grp) 머리글 줄: "고정됨"과 같은 머리글 + 오른쪽에 단추. 묶음 안에서는 스크롤해도 위에 붙어 있음 */
	.grp {
		margin-top: var(--space-2);
	}
	.grp.first {
		margin-top: var(--space-3);
	}
	.gh {
		position: sticky;
		top: 0;
		z-index: 1;
		display: flex;
		align-items: center;
		gap: 2px;
		background: var(--bg);
	}
	/* 묶음 안의 대화 목록: 스크롤은 바깥 목록(ul.tree)이 하므로 여기서는 늘 펼쳐 둠 */
	.lists ul.gl {
		padding: 0;
		overflow: visible;
	}
	/* 단추는 그 묶음에 마우스를 올리거나 키보드로 들어왔을 때만 (첫 묶음과 터치 화면은 늘) */
	.tools {
		display: inline-flex;
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.grp:hover .tools,
	.grp:focus-within .tools {
		opacity: 1;
	}
	@media (hover: none) {
		.tools {
			opacity: 1;
		}
	}
	.listbar .group.head,
	.gh .group.head {
		flex: 0 1 auto;
		min-width: 0;
		margin: 0;
		padding: 0 0 0 var(--space-1);
		height: 28px;
	}
	.sp {
		flex: 1;
	}
	.gn {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.group.head :global(svg) {
		flex: none;
	}
	/* 오른쪽 가장자리를 끌어 너비를 바꿈 */
	.grip {
		position: absolute;
		top: 0;
		right: -3px;
		bottom: 0;
		z-index: 2;
		width: 7px;
		padding: 0;
		border: none;
		background: transparent;
		cursor: col-resize;
		touch-action: none;
	}
	.grip:hover,
	.grip:focus-visible,
	.grip.drag {
		background: color-mix(in oklab, var(--accent) 35%, transparent);
		outline: none;
	}
	nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding: 0 var(--space-2);
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
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		overflow: hidden;
	}
	/* 스크롤은 목록(ul)이, 좌우 여백도 ul이 가진다 */
	.lists ul {
		padding: 0 var(--space-2);
		overflow-y: auto;
	}
	.lists ul.tree {
		flex: 1;
		min-height: 0;
		padding-bottom: var(--space-2);
	}
	.pinnedList {
		display: flex;
		flex-direction: column;
		padding: 0 var(--space-2);
		margin-top: var(--space-2);
	}
	.pinnedList .group.head {
		padding: 0 0 0 var(--space-1);
	}
	.pinnedList .open {
		padding: 0 var(--space-2) 0 0;
	}
	.show-more {
		display: block;
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.show-more:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.show-more small {
		color: var(--text-faint);
	}
	.group {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	/* 접을 수 있는 머리글 (고정됨) */
	.group.head {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 0;
		border: none;
		background: none;
		font-family: inherit;
		cursor: pointer;
	}
	.group.head:hover {
		color: var(--text-muted);
	}
	.group.head :global(svg),
	.fold :global(svg) {
		width: 13px;
		height: 13px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.group.head :global(.rot),
	.fold :global(.rot) {
		transform: rotate(-90deg);
	}
	/* 거르기·정렬·새 대화 줄 */
	.listbar {
		display: flex;
		align-items: center;
		gap: 2px;
		margin: var(--space-3) 2px var(--space-1);
		padding: 2px 2px 2px var(--space-1);
		border-radius: var(--radius-sm);
	}
	.listbar:hover {
		background: var(--surface-hover);
	}
	.empty {
		margin: var(--space-3) var(--space-2);
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	/* 갈래(분기) 표시: 갈래 수 */
	.fork {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 2px;
		padding-right: var(--space-2);
		font-size: var(--text-xs);
		color: var(--accent-text, var(--accent));
		font-variant-numeric: tabular-nums;
	}
	.fork :global(svg) {
		width: 13px;
		height: 13px;
	}
	.fold {
		flex: none;
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		padding: 0;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-faint);
		cursor: pointer;
	}
	.fold:hover {
		background: var(--surface-active);
		color: var(--text);
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	/* 대화 한 줄: 평소엔 시각, 마우스를 올리면 그 자리에 … 버튼 */
	.row {
		position: relative;
		display: flex;
		align-items: center;
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
		flex: 1;
		min-width: 0;
		height: 30px;
		padding: 0 var(--space-2) 0 calc(var(--space-2) + var(--depth, 0) * 16px);
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
	.row:hover .fork,
	.row:focus-within .fork {
		visibility: hidden;
	}
	.bottom {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-2) var(--space-2) 0;
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
