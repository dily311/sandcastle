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
		ArrowLeft,
		Eye,
		ImagePlus,
		Search,
		PanelLeft,
		PanelRight,
		Pencil,
		Plus,
		RefreshCw,
		Send,
		Trash2,
	} from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, IconButton, Input, Segmented, Select, Switch, Textarea, type MenuItem } from "$shared/ui";
	import ConvCard from "./ConvCard.svelte";
	import { characters, relative, store } from "./mock.svelte";

	let { convId, charId, onback, onopen }: { convId: string; charId: string; onback: () => void; onopen: (id: string) => void } = $props();
	const character = $derived(characters.find((c) => c.id === charId)!);

	let showList = $state(true);
	let showPanel = $state(true);
	let panelTab: "settings" | "toc" | "assets" = $state("settings");
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
	let sort: "recent" | "name" | "count" = $state("recent");
	const convs = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return store.convs
			.filter((c) => c.charId === charId && (!q || c.title.toLowerCase().includes(q)))
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || (sort === "recent" ? a.ago - b.ago : sort === "name" ? a.title.localeCompare(b.title) : b.count - a.count));
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
</script>

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
				<IconButton label="대화 목록으로" size="sm" onclick={onback}><ArrowLeft /></IconButton>
				<Avatar name={character.name} size={24} />
				<strong>{character.name}</strong>
				<IconButton label="새 대화" size="sm" variant="soft"><Plus /></IconButton>
			</div>
			<div class="convs-tools">
				<Input bind:value={query} placeholder="대화 검색" aria-label="대화 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<Select aria-label="정렬" bind:value={sort} options={[{ value: "recent", label: "최근 순" }, { value: "name", label: "이름 순" }, { value: "count", label: "메시지 많은 순" }]} />
			</div>
			<div class="convs-list">
				{#each convs as c (c.id)}
					<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
				{:else}
					<p class="none">검색 결과 없음</p>
				{/each}
			</div>
		</aside>
	{/if}

	<section class="reader">
		<header class="bar">
			<IconButton label="대화 목록" size="sm" active={showList} onclick={() => (showList = !showList)}><PanelLeft /></IconButton>
			<IconButton label="대화 패널" size="sm" active={showPanel} onclick={() => (showPanel = !showPanel)}><PanelRight /></IconButton>
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
			<div class="panel-head">
				<Segmented label="패널" bind:value={panelTab} options={[{ value: "settings", label: "설정" }, { value: "toc", label: "목차" }, { value: "assets", label: "에셋" }]} />
			</div>
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
					<div class="agrid">
						{#each assets as a, i (a.key)}
							<button class="tile" class:off={!a.on} title={a.on ? a.key : `${a.key} (AI에게 안 보냄)`} oncontextmenu={(e) => openAssetMenu(e, i)}>
								<span class="thumb">{#if !a.on}<EyeOff />{/if}</span>
								<span class="key">{a.key}</span>
							</button>
						{/each}
						<button class="tile add"><ImagePlus /><span>추가</span></button>
					</div>
					<ContextMenu at={menu} items={menuItems} label="에셋 메뉴" onclose={() => (menu = null)} />
				</div>
			{:else}
				<ol class="toc">
					{#each toc as t (t.kind + t.n + t.label)}
						<li class={t.kind}><button><span class="no">{t.kind === "msg" ? `#${t.n}` : ""}</span>{t.label}</button></li>
					{/each}
				</ol>
			{/if}
		</aside>
	{/if}
</div>

<style>
	.chat {
		display: grid;
		grid-template-columns: 280px minmax(0, 1fr) 300px;
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.chat.no-list {
		grid-template-columns: minmax(0, 1fr) 300px;
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
	.convs-head strong {
		flex: 1;
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.convs-tools {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: 0 var(--space-3) var(--space-2);
	}
	.convs-tools :global(select) {
		height: var(--control-sm);
		font-size: var(--text-xs);
	}
	.convs-list {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		overflow: auto;
		padding: 0 var(--space-3) var(--space-3);
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
	.tile.add {
		align-items: center;
		justify-content: center;
		gap: 4px;
		min-height: 80px;
		border-style: dashed;
		background: transparent;
		color: var(--text-muted);
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.tile.add:hover {
		border-color: var(--accent);
		color: var(--accent-text);
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
	.swipe .faint {
		margin-left: var(--space-2);
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
	.panel-head {
		padding: var(--space-3);
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
