<script lang="ts">
	import {
		Bookmark,
		ChevronLeft,
		ChevronRight,
		Copy,
		Download,
		EyeOff,
		FileSearch,
		GitBranch,
		Eye,
		ImagePlus,
		Search,
		PanelLeft,
		Pencil,
		Plus,
		RefreshCw,
		Send,
		Trash2,
	} from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, IconButton, Input, Select, Switch, Textarea, type MenuItem } from "$shared/ui";
	import { ArrowDownUp, ChevronDown as Chev, FileArchive, Folder, FolderOpen, FolderPlus, Images, ListTree, SlidersHorizontal, Upload } from "@lucide/svelte";

	const panelTabs = [
		{ id: "settings", label: "대화 설정", icon: SlidersHorizontal },
		{ id: "toc", label: "목차", icon: ListTree },
		{ id: "assets", label: "에셋", icon: Images },
	] as const;
	import ConvCard from "./ConvCard.svelte";
	import { characters, convActions, folderActions, relative, store, ui, type MockConv } from "./mock.svelte";

	let { convId, charId, onopen }: { convId: string; charId: string; onopen: (id: string) => void } = $props();
	const character = $derived(characters.find((c) => c.id === charId)!);

	let showList = $state(true);
	// 오른쪽 패널 버튼은 타이틀바에 있다
	$effect(() => {
		ui.panelAvailable = true;
		ui.panelTabs = [...panelTabs];
		ui.panelTab = "settings";
		return () => {
			ui.panelAvailable = false;
			ui.panelTabs = [];
		};
	});
	const panelTab = $derived(ui.panelTab);
	const showPanel = $derived(ui.panelOpen);
	let draft = $state("");
	let persona = $state("min");
	let personaOpen = $state(false);
	let personaBtn: HTMLButtonElement | undefined = $state();
	/** 패널 스크롤에 잘리지 않게 버튼 기준 고정 위치 */
	let popStyle = $state("");
	function togglePersona() {
		personaOpen = !personaOpen;
		if (!personaOpen || !personaBtn) return;
		const r = personaBtn.getBoundingClientRect();
		popStyle = `left:${r.left}px;width:${r.width}px;bottom:${window.innerHeight - r.top + 6}px;max-height:${r.top - 12}px`;
	}
	let preset = $state("st-default");
	let api = $state("gemini");
	let imagesOn = $state(true);
	let regexOn = $state([true, false, true]);

	// 같은 캐릭터의 대화만
	let query = $state("");
	type Sort = "newest" | "oldest" | "name" | "name-desc" | "most" | "least";
	let sort: Sort = $state("newest");
	const sorts: { id: Sort; label: string }[] = [
		{ id: "newest", label: "최신순" },
		{ id: "oldest", label: "오래된순" },
		{ id: "name", label: "이름순 (가→하)" },
		{ id: "name-desc", label: "이름 역순 (하→가)" },
		{ id: "most", label: "메시지 많은 순" },
		{ id: "least", label: "메시지 적은 순" },
	];
	const compare: Record<Sort, (a: MockConv, b: MockConv) => number> = {
		newest: (a, b) => a.ago - b.ago,
		oldest: (a, b) => b.ago - a.ago,
		name: (a, b) => a.title.localeCompare(b.title, "ko"),
		"name-desc": (a, b) => b.title.localeCompare(a.title, "ko"),
		most: (a, b) => b.count - a.count,
		least: (a, b) => a.count - b.count,
	};
	const convs = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return store.convs
			.filter((c) => c.charId === charId && (!q || c.title.toLowerCase().includes(q)))
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || compare[sort](a, b));
	});
	let sortMenu = $state<{ x: number; y: number } | null>(null);
	function openSortMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		sortMenu = { x: r.left, y: r.bottom + 4 };
	}
	const folders = $derived(store.folders.filter((f) => f.charId === charId));
	let collapsed: Record<string, boolean> = $state({});

	// 폴더 만들기 / 이름 바꾸기 (같은 입력칸)
	let editingFolder: string | null = $state(null); // "new" 또는 폴더 id
	let folderName = $state("");
	function startFolderEdit(id: string, name = "") {
		editingFolder = id;
		folderName = name;
	}
	function commitFolder() {
		if (editingFolder === "new") folderActions.create(charId, folderName);
		else if (editingFolder) folderActions.rename(editingFolder, folderName);
		editingFolder = null;
	}
	let folderMenuAt = $state<{ x: number; y: number } | null>(null);
	let folderMenuItems: MenuItem[] = $state([]);
	function folderMenu(e: MouseEvent, id: string, name: string) {
		e.preventDefault();
		folderMenuItems = [
			{ label: "이름 바꾸기", icon: Pencil, onselect: () => startFolderEdit(id, name) },
			{ label: "폴더 삭제 (대화는 남김)", icon: Trash2, danger: true, separator: true, onselect: () => folderActions.remove(id) },
		];
		folderMenuAt = { x: e.clientX, y: e.clientY };
	}
	// 카드를 폴더(또는 폴더 밖)로 끌어다 놓기
	let dropTarget: string | null = $state(null);
	const dnd = (target: string | null, key: string) => ({
		ondragover: (e: DragEvent) => {
			e.preventDefault();
			dropTarget = key;
		},
		ondragleave: () => (dropTarget = null),
		ondrop: (e: DragEvent) => {
			e.preventDefault();
			const id = e.dataTransfer?.getData("text/conv-id");
			if (id) convActions.move(id, target);
			dropTarget = null;
		},
	});

	const personas = [
		{ id: "min", name: "Min", desc: "숲에서 길 잃은 여행자" },
		{ id: "haru", name: "하루", desc: "대학생" },
		{ id: "default", name: "기본", desc: "" },
		{ id: "knight", name: "기사 레온", desc: "왕국 기사" },
	];
	const current = $derived(personas.find((p) => p.id === persona)!);

	let assets = $state([
		{ key: "joy", on: true },
		{ key: "anger", on: true },
		{ key: "amusement", on: true },
		{ key: "sadness", on: true },
		{ key: "neutral", on: true },
		{ key: "fear", on: false },
	]);
	// 에셋 오른쪽 클릭 메뉴
	let menu = $state<{ x: number; y: number } | null>(null);
	let menuItems: MenuItem[] = $state([]);
	function openAssetMenu(e: MouseEvent, i: number) {
		e.preventDefault();
		const a = assets[i];
		menuItems = [
			{ label: a.on ? "AI에게 보내지 않기" : "AI에게 보내기", icon: a.on ? EyeOff : Eye, onselect: () => (assets[i].on = !assets[i].on) },
			{ label: "키 이름 바꾸기", icon: Pencil, onselect: () => {} },
			{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => assets.splice(i, 1) },
		];
		menu = { x: e.clientX, y: e.clientY };
	}

	const toc = [
		{ kind: "chapter", label: "1장 · 숲에서 깨어남", n: 1 },
		{ kind: "msg", label: "눈을 떴군요. 다행이에요…", n: 1 },
		{ kind: "msg", label: "여기가 어디죠?", n: 2 },
		{ kind: "msg", label: "제 오두막이에요…", n: 3 },
		{ kind: "chapter", label: "2장 · 그림자 송곳니", n: 4 },
		{ kind: "msg", label: "(숨긴 메시지)", n: 4 },
	];
	let tocQuery = $state("");
	const tocShown = $derived.by(() => {
		const q = tocQuery.trim().toLowerCase();
		if (!q) return toc;
		// 메시지 번호(#3, 3)나 내용으로 찾기
		const num = Number(q.replace("#", ""));
		return toc.filter((t) => t.label.toLowerCase().includes(q) || (t.kind === "msg" && t.n === num));
	});
</script>

{#snippet folderInput()}
	<!-- svelte-ignore a11y_autofocus -->
	<input
		class="folder-input"
		bind:value={folderName}
		placeholder="폴더 이름"
		aria-label="폴더 이름"
		autofocus
		onkeydown={(e) => {
			if (e.key === "Enter") commitFolder();
			if (e.key === "Escape") editingFolder = null;
		}}
		onblur={commitFolder}
	/>
{/snippet}

<svelte:window
	onmousedown={(e) => {
		const t = e.target as Element;
		if (personaOpen && !t.closest(".ppop") && !t.closest(".pcur")) personaOpen = false;
	}}
	onkeydown={(e) => e.key === "Escape" && (personaOpen = false)}
/>

{#snippet stamp(minutesAgo: number, model?: string)}
	{@const at = new Date(Date.now() - minutesAgo * 60_000)}
	<span class="stamp">
		<time datetime={at.toISOString()} title={at.toLocaleString("ko-KR", { dateStyle: "full", timeStyle: "medium" })}>{relative(minutesAgo)}</time>
		{#if model}<span class="model">· {model}</span>{/if}
	</span>
{/snippet}

<div class="chat" class:no-list={!showList} class:no-panel={!showPanel}>
	{#if showList}
		<aside class="convs" aria-label="대화 목록">
			<div class="convs-head">
				<span class="spacer"></span>
				<IconButton label="새 대화" size="sm" variant="soft"><Plus /></IconButton>
				<IconButton label="대화 가져오기 (SillyTavern .jsonl 등)" size="sm"><Upload /></IconButton>
				<IconButton label="새 폴더" size="sm" onclick={() => startFolderEdit("new")}><FolderPlus /></IconButton>
			</div>
			<div class="convs-tools">
				<Input bind:value={query} placeholder="대화 검색" aria-label="대화 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<IconButton label="정렬: {sorts.find((s) => s.id === sort)?.label}" size="md" active={!!sortMenu} aria-haspopup="menu" onclick={openSortMenu}><ArrowDownUp /></IconButton>
			</div>
			<ContextMenu
				at={sortMenu}
				label="정렬"
				items={sorts.map((s, i) => ({ label: s.label, checked: sort === s.id, separator: i === 2 || i === 4, onselect: () => (sort = s.id) }))}
				onclose={() => (sortMenu = null)}
			/>
			<div class="convs-list">
				{#if editingFolder === "new"}{@render folderInput()}{/if}
				{#each folders as f (f.id)}
					{@const items = convs.filter((c) => c.folderId === f.id)}
					{#if editingFolder === f.id}
						{@render folderInput()}
					{:else}
						<button
							class="folder"
							class:drop={dropTarget === f.id}
							aria-expanded={!collapsed[f.id]}
							onclick={() => (collapsed[f.id] = !collapsed[f.id])}
							oncontextmenu={(e) => folderMenu(e, f.id, f.name)}
							{...dnd(f.id, f.id)}
						>
							<Chev class={collapsed[f.id] ? "closed" : ""} />
							{#if collapsed[f.id]}<Folder />{:else}<FolderOpen />{/if}
							<span class="fname">{f.name}</span>
							<span class="fcount">{items.length}</span>
						</button>
					{/if}
					{#if !collapsed[f.id]}
						<div class="in-folder">
							{#each items as c (c.id)}
								<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
							{:else}
								<p class="empty-folder">비어 있음</p>
							{/each}
						</div>
					{/if}
				{/each}
				<div class="loose" class:drop={dropTarget === "loose"} {...dnd(null, "loose")}>
					{#each convs.filter((c) => !c.folderId) as c (c.id)}
						<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
					{/each}
				</div>
				{#if !convs.length}<p class="none">검색 결과 없음</p>{/if}
			</div>
			<ContextMenu at={folderMenuAt} items={folderMenuItems} label="폴더 메뉴" onclose={() => (folderMenuAt = null)} />
		</aside>
	{/if}

	<section class="reader">
		<header class="bar">
			<IconButton label="대화 목록" size="sm" active={showList} onclick={() => (showList = !showList)}><PanelLeft /></IconButton>
		</header>

		<div class="scroll">
			<article class="text">
				<h2 class="chapter"><span>1장</span>숲에서 깨어남</h2>

				<div class="block ai">
					<div class="who">
						<span class="no">#1</span><b>Seraphina</b>
						{@render stamp(42, "gemini-3.7-flash")}
						<div class="actions" role="toolbar" aria-label="메시지 1 동작">
							<IconButton label="분기 나누기" size="sm"><GitBranch /></IconButton>
							<IconButton label="복사" size="sm"><Copy /></IconButton>
							<IconButton label="수정" size="sm"><Pencil /></IconButton>
							<IconButton label="여기서 챕터 시작" size="sm"><Bookmark /></IconButton>
							<IconButton label="AI에게 숨기기" size="sm"><EyeOff /></IconButton>
							<IconButton label="처음부터 다시 생성 (스와이프 버림)" size="sm"><RefreshCw /></IconButton>
							<IconButton label="보낸 컨텍스트 보기" size="sm"><FileSearch /></IconButton>
							<IconButton label="삭제" size="sm"><Trash2 /></IconButton>
						</div>
					</div>
					<p>*당신이 눈을 뜨자, 부드러운 빛 속에서 한 여인이 고개를 숙인다.*</p>
					<p>"눈을 떴군요. 다행이에요… 숲의 짐승들에게 쫓기다 쓰러졌었어요."</p>
					<figure class="cg"><div class="img"></div><figcaption>amusement</figcaption></figure>
				</div>

				<div class="block user">
					<div class="who"><span class="no">#2</span><b>Min</b>{@render stamp(41)}</div>
					<p>여기가 어디죠? 숲인가요?</p>
				</div>

				<div class="block ai">
					<div class="who"><span class="no">#3</span><b>Seraphina</b>{@render stamp(40, "gemini-3.7-flash")}</div>
					<p>"제 오두막이에요. 이 숲, 엘도리아는 제가 지키는 곳이에요. 여기선 아무도 당신을 해치지 못해요."</p>
					<p>*그녀가 따뜻한 차를 건넨다.*</p>
				</div>

				<h2 class="chapter"><span>2장</span>그림자 송곳니</h2>

				<div class="block ai hidden">
					<div class="who"><span class="no">#4</span><b>Seraphina</b><Badge><EyeOff /> AI에게 숨김</Badge></div>
					<p>(작가 메모: 다음 장면에서 늑대 무리 등장)</p>
				</div>

				<div class="block ai last">
					<div class="who"><span class="no">#5</span><b>Seraphina</b>{@render stamp(2, "gemini-3.7-flash")}</div>
					<p>*멀리서 늑대 울음소리가 들린다. 세라피나의 표정이 굳는다.*</p>
					<div class="swipe">
						<IconButton label="이전 응답" size="sm"><ChevronLeft /></IconButton>
						<span>2 / 3</span>
						<IconButton label="다음 응답 (마지막이면 새로 생성)" size="sm"><ChevronRight /></IconButton>
					</div>
				</div>
			</article>
		</div>

		<form class="composer" onsubmit={(e) => e.preventDefault()}>
			<IconButton label="첨부·추가 기능"><Plus /></IconButton>
			<Textarea bind:value={draft} rows={1} autosize maxRows={8} placeholder="다음 행동이나 대사 입력" aria-label="메시지" />
			<IconButton label="보내기" variant="solid" disabled={!draft.trim()}><Send /></IconButton>
		</form>
	</section>

	{#if showPanel}
		<aside class="panel" aria-label="대화 설정">
			{#if panelTab === "settings"}
				<div class="panel-body">
					<section>
						<h4>이 대화</h4>
						<label class="row"><span>API</span><Select bind:value={api} options={[{ value: "gemini", label: "Gemini · 3.7 flash" }, { value: "claude", label: "Claude" }]} /></label>
						<label class="row"><span>프리셋</span><Select bind:value={preset} options={[{ value: "st-default", label: "Default (ST)" }, { value: "none", label: "기본 배치" }]} /></label>
						<div class="persona">
							<span>페르소나</span>
							<div class="pwrap">
								{#if personaOpen}
									<div class="ppop" role="listbox" aria-label="페르소나 바꾸기" style={popStyle}>
										{#each personas as p (p.id)}
											<button
												role="option"
												aria-selected={persona === p.id}
												class:on={persona === p.id}
												onclick={() => {
													persona = p.id;
													personaOpen = false;
												}}
											>
												<Avatar name={p.name} size={32} />
												<span class="pt"><b>{p.name}</b>{#if p.desc}<small>{p.desc}</small>{/if}</span>
											</button>
										{/each}
									</div>
								{/if}
								<button bind:this={personaBtn} class="pcur" aria-haspopup="listbox" aria-expanded={personaOpen} onclick={togglePersona}>
									<Avatar name={current.name} size={40} />
									<span class="pt"><b>{current.name}</b>{#if current.desc}<small>{current.desc}</small>{/if}</span>
									<span class="pchange">바꾸기</span>
								</button>
							</div>
						</div>
						<Switch bind:checked={imagesOn} label="캐릭터 이미지" description="키 목록 전송, <cg>로 표시" />
					</section>
					<section>
						<h4>정규식 <span class="faint">적용 중 2개</span></h4>
						{#each ["별표 지우기 · 전역", "생각 숨기기 · 프리셋", "말투 교정 · 캐릭터"] as r, i (r)}
							<Switch bind:checked={regexOn[i]} label={r} />
						{/each}
					</section>
					<section>
						<h4>내보내기</h4>
						<Button block>
							{#snippet icon()}<Download />{/snippet}
							이 대화 내보내기
						</Button>
					</section>
				</div>
			{:else if panelTab === "assets"}
				<div class="panel-body">
					<div class="atools">
						<span class="faint">{assets.length}개</span>
						<IconButton label="이미지 가져오기 (여러 장 선택)" size="sm"><ImagePlus /></IconButton>
						<IconButton label="ZIP 가져오기 (표정 폴더 묶음)" size="sm"><FileArchive /></IconButton>
					</div>
					<div class="agrid">
						{#each assets as a, i (a.key)}
							<button class="tile" class:off={!a.on} title={a.on ? a.key : `${a.key} (AI에게 안 보냄)`} oncontextmenu={(e) => openAssetMenu(e, i)}>
								<span class="thumb">{#if !a.on}<EyeOff />{/if}</span>
								<span class="key">{a.key}</span>
							</button>
						{/each}
					</div>
					<ContextMenu at={menu} items={menuItems} label="에셋 메뉴" onclose={() => (menu = null)} />
				</div>
			{:else}
				<div class="toc-search">
					<Input bind:value={tocQuery} placeholder="챕터·메시지·번호 검색" aria-label="목차 검색">
						{#snippet leading()}<Search />{/snippet}
					</Input>
				</div>
				<ol class="toc">
					{#each tocShown as t (t.kind + t.n + t.label)}
						<li class={t.kind}><button><span class="no">{t.kind === "msg" ? `#${t.n}` : ""}</span>{t.label}</button></li>
					{:else}
						<li class="toc-none">찾는 항목 없음</li>
					{/each}
				</ol>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.chat {
		display: grid;
		grid-template-columns: 280px minmax(0, 1fr) var(--panel-width);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.chat.no-list {
		grid-template-columns: minmax(0, 1fr) var(--panel-width);
	}
	.chat.no-panel {
		grid-template-columns: 280px minmax(0, 1fr);
	}
	.chat.no-list.no-panel {
		grid-template-columns: minmax(0, 1fr);
	}

	.convs {
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--border);
		background: var(--bg);
	}
	.convs-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-3) var(--space-2);
	}
	.spacer {
		flex: 1;
	}
	.convs-tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-1);
		padding: 0 var(--space-3) var(--space-2);
	}
	.atools {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		margin-bottom: calc(var(--space-2) * -1);
	}
	.atools span {
		margin-right: auto;
		font-size: var(--text-xs);
	}
	.toc-search {
		padding: var(--space-3) var(--space-3) var(--space-2);
	}
	.toc-none {
		padding: var(--space-4);
		text-align: center;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.convs-list {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		overflow: auto;
		padding: 0 var(--space-3) var(--space-3);
	}
	.folder {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: 1px dashed transparent;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font-size: var(--text-sm);
		font-weight: 600;
		text-align: left;
		cursor: pointer;
	}
	.folder:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.folder :global(svg) {
		width: 15px;
		height: 15px;
		flex: none;
		transition: transform var(--duration-fast) var(--ease);
	}
	.folder :global(.closed) {
		transform: rotate(-90deg);
	}
	.fname {
		flex: 1;
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.fcount {
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--text-faint);
	}
	.folder.drop,
	.loose.drop {
		border-color: var(--accent);
		background: var(--accent-soft);
	}
	.in-folder {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding-left: var(--space-3);
		margin-left: 9px;
		border-left: 1px solid var(--border);
	}
	.empty-folder {
		padding: 2px var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.loose {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-height: 24px;
		border: 1px dashed transparent;
		border-radius: var(--radius-md);
	}
	.folder-input {
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		background: var(--surface);
		outline: none;
		flex: none;
	}
	.none {
		padding: var(--space-6);
		text-align: center;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.persona {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.pwrap {
		position: relative;
	}
	.pcur,
	.ppop button {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.pcur {
		border: 1px solid var(--border);
		background: var(--surface);
	}
	.pcur:hover,
	.ppop button:hover {
		background: var(--surface-hover);
	}
	.pt {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		line-height: 1.3;
	}
	.pt small {
		color: var(--text-muted);
		font-size: var(--text-xs);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.pchange {
		font-size: var(--text-xs);
		color: var(--accent-text);
		font-weight: 600;
	}
	/* 현재 페르소나 위로 뜨는 선택 목록 */
	.ppop {
		position: fixed;
		z-index: 100;
		overflow: auto;
		display: flex;
		flex-direction: column;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		animation: up var(--duration-fast) var(--ease);
	}
	@keyframes up {
		from {
			opacity: 0;
			transform: translateY(4px);
		}
	}
	.ppop button.on {
		background: var(--accent-soft);
	}
	.agrid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: var(--space-2);
	}
	.tile {
		display: flex;
		flex-direction: column;
		padding: 0;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
		cursor: context-menu;
	}
	.tile:hover {
		border-color: var(--border-strong);
	}
	.thumb {
		display: grid;
		place-items: center;
		aspect-ratio: 1;
		background: linear-gradient(135deg, var(--accent-soft-hover), var(--bg-sunken));
		color: var(--text-muted);
	}
	.tile.off .thumb {
		opacity: 0.4;
		filter: grayscale(1);
	}
	.key {
		padding: 3px 6px;
		font-size: 11px;
		font-family: var(--font-mono);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		text-align: left;
	}
	.tile.off .key {
		color: var(--text-faint);
	}

	.reader {
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--surface);
	}
	.bar {
		display: flex;
		justify-content: space-between;
		padding: var(--space-2) var(--space-3) 0;
		background: transparent;
		position: fixed;
	}
	/* 대화 목록 버튼은 마우스를 올리거나 키보드로 갔을 때만 보임 */
	.bar :global(.icon-btn) {
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.bar:hover :global(.icon-btn),
	.bar :global(.icon-btn:focus-visible) {
		opacity: 1;
	}
	.stamp {
		display: inline-flex;
		gap: 4px;
		color: var(--text-faint);
	}
	.stamp time {
		cursor: help;
	}
	.scroll {
		flex: 1;
		overflow: auto;
	}
	.text {
		max-width: 720px;
		margin: 0 auto;
		padding: var(--space-8) var(--space-6) var(--space-10);
		font-size: 15.5px;
		line-height: 1.85;
	}
	.chapter {
		display: flex;
		flex-direction: column;
		align-items: center;
		margin: var(--space-8) 0 var(--space-6);
		font-size: var(--text-xl);
		text-align: center;
	}
	.chapter:first-child {
		margin-top: 0;
	}
	.chapter span {
		font-size: var(--text-xs);
		color: var(--accent-text);
		letter-spacing: 0.2em;
		font-weight: 700;
	}
	.block {
		position: relative;
		margin: 0 calc(var(--space-3) * -1);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
	}
	.block + .block {
		margin-top: var(--space-2);
	}
	.block:hover {
		background: var(--surface-2);
	}
	.block p + p {
		margin-top: 0.6em;
	}
	.who {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-height: 28px;
		font-size: var(--text-xs);
		color: var(--text-muted);
		line-height: 1;
	}
	.no {
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.actions {
		margin-left: auto;
		display: flex;
		gap: 0;
		padding: 2px;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
		opacity: 0;
		pointer-events: none;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.block:hover .actions,
	.block:focus-within .actions {
		opacity: 1;
		pointer-events: auto;
	}
	.user {
		border-left: 3px solid var(--accent);
		border-radius: 0 var(--radius-md) var(--radius-md) 0;
		background: var(--accent-soft);
		color: var(--text);
	}
	.user:hover {
		background: var(--accent-soft-hover);
	}
	.hidden {
		opacity: 0.6;
		outline: 1px dashed var(--border-strong);
	}
	.cg {
		margin: var(--space-4) auto 0;
		width: min(320px, 100%);
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid var(--border);
	}
	.cg .img {
		height: 200px;
		background: linear-gradient(135deg, var(--accent-soft-hover), var(--bg-sunken));
	}
	.cg figcaption {
		padding: 2px var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.swipe {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-top: var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.composer {
		display: flex;
		align-items: flex-end;
		gap: var(--space-2);
		max-width: 760px;
		width: 100%;
		margin: 0 auto;
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.composer :global(textarea) {
		resize: none;
		border-radius: var(--radius-lg);
	}

	.panel {
		display: flex;
		flex-direction: column;
		border-left: 1px solid var(--border);
		background: var(--bg);
		min-width: 0;
	}
	.panel-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding: var(--space-2) var(--space-4) var(--space-4);
		overflow: auto;
	}
	.panel section {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	h4 {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
		letter-spacing: 0.04em;
		display: flex;
		justify-content: space-between;
	}
	.row {
		display: grid;
		grid-template-columns: 64px 1fr;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.toc {
		list-style: none;
		margin: 0;
		padding: 0 var(--space-2) var(--space-4);
		overflow: auto;
	}
	.toc button {
		display: flex;
		gap: var(--space-2);
		width: 100%;
		padding: 6px var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		text-align: left;
		font-size: var(--text-sm);
		color: var(--text-muted);
		cursor: pointer;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.toc button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.toc .chapter-item,
	.toc .chapter button {
		font-weight: 700;
		color: var(--text);
		margin-top: var(--space-2);
	}
	.toc .no {
		width: 26px;
		flex: none;
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	@media (max-width: 900px) {
		.chat,
		.chat.no-list,
		.chat.no-panel {
			grid-template-columns: minmax(0, 1fr);
		}
		.convs,
		.panel {
			display: none;
		}
	}
</style>
