<script lang="ts">
	import { characters } from "$/lib/chat/characterStore.svelte";
	import { newConversation } from "$/lib/chat/chatStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import Icon from "$/components/ui/Icon.svelte";
	import { push } from "svelte-spa-router";
	import Avatar from "$components/avatar.svelte";
	import Button from "$components/ui/button/Button.svelte";
	import ButtonGroup from "$/components/ui/button/ButtonGroup.svelte";
	import Card from "$/components/ui/card/Card.svelte";
	import CardGrid from "$/components/ui/card/CardGrid.svelte";
	import { cn } from "$/lib/utils/cn";

	type Sort = "recent" | "name" | "favorite";

	// 스토어에서 상태 및 메서드 가져오기
	let query = $state("");
	let activeTags = $state<string[]>([]);
	let sort = $state<Sort>("recent");
	let view = $state<"grid" | "list">("grid");
	let onlyFav = $state(false);

	// 태그 목록 계산
	let allTags = $derived(() => {
		const set = new Map<string, number>();
		characters.value.forEach((c) => c.tags.forEach((t) => set.set(t, (set.get(t) ?? 0) + 1)));
		return [...set.entries()].sort((a, b) => b[1] - a[1]);
	});

	// 필터링 및 정렬된 결과 계산
	let results = $derived(() => {
		const q = query.trim().toLowerCase();
		let list = characters.value.filter((c) => {
			const hay = `${c.name} ${c.tags.toString()} ${c.description} ${c.tags.join(" ")}`.toLowerCase();
			const matchQ = !q || hay.includes(q);
			const matchTags = activeTags.every((t) => c.tags.includes(t));
			const matchFav = !onlyFav || c.favorite;
			return matchQ && matchTags && matchFav;
		});

		list = [...list].sort((a, b) => {
			if (sort === "name") return a.name.localeCompare(b.name, "ja");
			if (sort === "favorite") return Number(b.favorite) - Number(a.favorite) || b.updatedAt - a.updatedAt;
			return b.updatedAt - a.updatedAt;
		});
		return list;
	});

	function open(char: Character) {
		push("/character/details/" + char.id);
	}
	async function newChat(char: Character) {
		const convID = await newConversation(char.id);
		push("/chat/" + convID);
	}

	function toggleTag(t: string) {
		activeTags = activeTags.includes(t) ? activeTags.filter((x) => x !== t) : [...activeTags, t];
	}

	function toggleFavorite(id: string): Character {
		return {
			id: "2bcfbbca-597b-49d6-a5d3-f7af95da1661",
			name: "Mika",
			avatar: "🎮",
			description:
				"Mika went from 0 to 2 million followers in a year. Chaotic good energy, loud reactions, and an encyclopedic memory for game lore.",
			alternateDescriptions: [],
			firstMessage:
				"YO!! You're here!! Okay okay okay, chat's gone wild today. Have you SEEN the new update? We have SO much to unpack.",
			alternateFirstMessage: [],
			exampleDialogue: null,
			systemPromptOverride:
				"You are Mika, an energetic streamer. Use all-caps excitement, gaming slang, and be relentlessly upbeat and playful.",
			worldInfoIds: [],
			color: "#0891b2",
			tags: ["Gaming", "Comedy", "Modern"],
			category: "Slice of Life",
			defaultPersonaId: null,
			favorite: false,
			messageCount: 0,
			createdAt: 1791954311016,
			updatedAt: 1791954311016,
		};
	}
	function duplicateCharacter(id: string): Character {
		return {
			id: "2bcfbbca-597b-49d6-a5d3-f7af95da1661",
			name: "Mika",
			avatar: "🎮",
			description:
				"Mika went from 0 to 2 million followers in a year. Chaotic good energy, loud reactions, and an encyclopedic memory for game lore.",
			alternateDescriptions: [],
			firstMessage:
				"YO!! You're here!! Okay okay okay, chat's gone wild today. Have you SEEN the new update? We have SO much to unpack.",
			alternateFirstMessage: [],
			exampleDialogue: null,
			systemPromptOverride:
				"You are Mika, an energetic streamer. Use all-caps excitement, gaming slang, and be relentlessly upbeat and playful.",
			worldInfoIds: [],
			color: "#0891b2",
			tags: ["Gaming", "Comedy", "Modern"],
			category: "Slice of Life",
			defaultPersonaId: null,
			favorite: false,
			messageCount: 0,
			createdAt: 1791954311016,
			updatedAt: 1791954311016,
		};
	}
</script>

<div class="page">
	<header class="page-head">
		<div>
			<div class="breadcrumb">작업 공간 / 캐릭터</div>
			<h1 class="page-title">캐릭터 찾기</h1>
			<p class="page-sub">대화하고 싶은 상대를 선택하거나 자신만의 페르소나를 만들 수 있습니다.</p>
		</div>
		<Button variant="primary" onclick={() => push("/character/edit/")}>
			<Icon name="plus" size={15} /> 새로 만들기
		</Button>
	</header>

	<div class="searchbar">
		<Icon name="search" size={16} />
		<input class="searchbar-input" placeholder="이름, 설명, 태그로 검색..." bind:value={query} />
		{#if query}
			<Button size="sm" variant="ghost" onclick={() => (query = "")}>필터해제</Button>
		{/if}
	</div>

	<div class="filter-row">
		<div class="tag-row">
			<button class={`tag ${onlyFav ? "selected" : ""}`} onclick={() => (onlyFav = !onlyFav)}>
				<Icon name={onlyFav ? "star-filled" : "star"} size={12} /> 즐겨찾기
			</button>
			{#each allTags() as [t, n]}
				<button class={`tag ${activeTags.includes(t) ? "selected" : ""}`} onclick={() => toggleTag(t)}>
					{t} <span class="tag-count">{n}</span>
				</button>
			{/each}
		</div>

		<div class="filter-right">
			<select class="select mini" bind:value={sort}>
				<option value="recent">업데이트 순서</option>
				<option value="name">이름순</option>
				<option value="favorite">즐겨찾기 순서</option>
			</select>
			<ButtonGroup segmented={true}>
				<Button
					variant="ghost"
					size="sm"
					class={cn(view === "grid" && "on")}
					onclick={() => (view = "grid")}
					title="그리드"
				>
					<Icon name="grid" size={15} />
				</Button>
				<Button
					variant="ghost"
					size="sm"
					class={cn(view === "list" && "on")}
					onclick={() => (view = "list")}
					title="리스트"
				>
					<Icon name="list" size={15} />
				</Button>
			</ButtonGroup>
		</div>
	</div>

	<div class="result-count">
		{results().length} 건 캐릭터
		{#if activeTags.length > 0}
			<button class="btn ghost sm" onclick={() => (activeTags = [])}> 필터 해제 </button>
		{/if}
	</div>

	{#if results().length === 0}
		<div class="empty">
			<Icon name="search" />
			<div>해당 캐릭터가 없습니다.</div>
			<button class="btn sm" onclick={() => push("/character/edit/")}> 이 조건으로 새로 만들기 </button>
		</div>
	{:else if view === "grid"}
		<CardGrid class="card-grid" columns={5}>
			{#each results() as c (c.id)}
				<Card class="char-card" onclick={() => open(c)} role="presentation">
					<div class="char-card-top">
						<Avatar avatar={c.avatar ?? c.color} />
						<Button
							variant="ghost"
							size="icon"
							class={`star-btn ${c.favorite ? "on" : ""}`}
							onclick={(e) => {
								e.stopPropagation();
								toggleFavorite(c.id);
							}}
							title="즐겨찾기"
						>
							<Icon name={c.favorite ? "star-filled" : "star"} size={15} />
						</Button>
					</div>
					<h3 class="char-name">{c.name}</h3>
					<p class="char-tagline">{c.description || "설명 없음"}</p>
					<div class="tag-row small">
						{#each c.tags.slice(0, 3) as t}
							<span class="tag">
								{t}
							</span>
						{/each}
					</div>
					<ButtonGroup>
						<Button
							variant="primary"
							size="sm"
							onclick={(e) => {
								e.stopPropagation();
								newChat(c);
							}}
						>
							<Icon name="message-circle" size={13} /> 대화하기
						</Button>
						<Button
							variant="ghost"
							size="sm"
							onclick={(e) => {
								e.stopPropagation();
								push("/character/edit/" + c.id);
							}}
						>
							<Icon name="edit" size={13} /> 편집
						</Button>
						<Button
							variant="ghost"
							size="icon-xs"
							title="복제"
							onclick={(e) => {
								e.stopPropagation();
								const copy = duplicateCharacter(c.id);
								push("/character/edit/" + copy.id);
							}}
						>
							<Icon name="copy" size={13} />
						</Button>
						<Button
							variant="danger"
							size="icon-xs"
							title="삭제"
							onclick={(e) => {
								e.stopPropagation();
								characters.remove(c.id);
							}}
						>
							<Icon name="trash" size={13} />
						</Button>
					</ButtonGroup>
				</Card>
			{/each}
		</CardGrid>
	{:else}
		<div class="char-list">
			{#each results() as c (c.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<div class="char-row" onclick={() => open(c)} role="presentation">
					<Avatar avatar={c.avatar ?? c.color} />
					<div class="char-row-main">
						<div class="char-row-name">
							{c.name}
						</div>
						<div class="char-row-sub ellipsis">{c.description}</div>
					</div>
					<div class="tag-row small hide-sm">
						{#each c.tags.slice(0, 2) as t}
							<span class="tag">
								{t}
							</span>
						{/each}
					</div>
					<div class="char-row-actions">
						<button
							class={`star-btn ${c.favorite ? "on" : ""}`}
							onclick={(e) => {
								e.stopPropagation();
								toggleFavorite(c.id);
							}}
						>
							<Icon name={c.favorite ? "star-filled" : "star"} size={15} />
						</button>
						<button
							class="clickable-icon"
							onclick={(e) => {
								e.stopPropagation();
								push("/character/edit/" + c.id);
							}}
						>
							<Icon name="edit" size={15} />
						</button>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>

<style>
	.workspace {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		padding: 4px 4px;
		border-radius: var(--radius-sm);
	}
	.ws-badge {
		width: 22px;
		height: 22px;
		border-radius: 5px;
		background: var(--text-color);
		color: var(--card-bg);
		font-size: 10px;
		font-weight: 700;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		letter-spacing: 0.03em;
		flex: none;
	}
	.ws-text {
		min-width: 0;
	}
	.ws-name {
		font-size: 13.5px;
		font-weight: 600;
		line-height: 1.25;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ws-sub {
		font-size: 11.5px;
		color: var(--text-muted);
		line-height: 1.3;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sidebar-nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-bottom: 10px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-width: 0;
		padding: 5px 8px;
		border-radius: var(--radius-sm);
		color: var(--text-italic);
		font-size: 13.5px;
		text-align: left;
		transition:
			background 100ms ease,
			color 100ms ease;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.nav-item:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}
	.nav-item.active {
		background: var(--bg-active);
		color: var(--text-color);
		font-weight: 500;
	}
	.nav-item svg {
		flex: none;
		opacity: 0.8;
	}
	.nav-emoji {
		width: 18px;
		text-align: center;
		flex: none;
		font-size: 13px;
	}

	.nav-item.row {
		padding: 0;
		gap: 0;
	}
	.nav-item.row .row-main {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		padding: 5px 4px 5px 8px;
		color: inherit;
		font-size: 13.5px;
		text-align: left;
	}
	.row-action {
		width: 24px;
		height: 24px;
		margin-right: 4px;
		border-radius: 4px;
		color: var(--text-muted);
		display: none;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.nav-item.row:hover .row-action {
		display: inline-flex;
	}
	.row-action:hover {
		background: var(--bg-active);
		color: var(--danger);
	}

	.sidebar-section {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-bottom: 12px;
	}
	.sidebar-section.grow {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
	}
	.section-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
		padding: 6px 8px 3px;
	}
	.section-empty {
		font-size: 12px;
		color: var(--text-muted);
		padding: 2px 8px 6px;
	}

	.sidebar-foot {
		display: flex;
		align-items: center;
		gap: 7px;
		border-top: 1px solid var(--border-color);
		padding: 9px 10px;
		font-size: 11.5px;
		color: var(--text-muted);
	}
	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #4caf7d;
		flex: none;
	}

	/* ============================================================
   Page (search / edit / settings)
   ============================================================ */

	.page {
		flex: 1;
		overflow-y: auto;
		padding: 40px 56px 90px;
	}
	.page.narrow > * {
		max-width: 720px;
		margin-left: auto;
		margin-right: auto;
	}

	.page-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 20px;
		margin-bottom: 26px;
	}
	.page-head.sticky {
		position: sticky;
		top: -40px;
		z-index: 5;
		background: var(--card-bg);
		padding: 12px 0;
		margin: -12px 0 20px;
		border-bottom: 1px solid var(--border-color);
		align-items: center;
	}
	.head-left {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}
	.head-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}
	.breadcrumb {
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 4px;
	}
	.page-title {
		font-size: 30px;
		font-weight: 700;
		letter-spacing: -0.02em;
		line-height: 1.2;
	}
	.page-title.sm {
		font-size: 18px;
		font-weight: 600;
	}
	.page-sub {
		color: var(--text-italic);
		margin-top: 6px;
		font-size: 13.5px;
	}
	.saved-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: var(--color-green);
		margin-right: 4px;
	}

	/* search */
	.filter-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-top: 14px;
		flex-wrap: wrap;
	}
	.tag {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		height: 22px;
		padding: 0 8px;
		border-radius: 4px;
		background: var(--background-secondary);
		border: 1px solid var(--border-color);
		color: var(--text-italic);
		font-size: 12px;
		white-space: nowrap;
	}
	.tag-row {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}
	.tag-row.small .tag {
		height: 20px;
		font-size: 11.5px;
	}
	.tag-count {
		color: var(--text-muted);
		font-size: 10.5px;
	}
	.filter-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.select.mini {
		width: auto;
		height: 28px;
		padding: 0 8px;
		font-size: 12.5px;
	}

	.result-count {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 22px 0 12px;
		font-size: 12.5px;
		color: var(--text-muted);
	}

	.avatar {
		width: 30px;
		height: 30px;
		font-size: 15px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.avatar.lg {
		width: 40px;
		height: 40px;
		font-size: 20px;
	}
	.avatar.xl {
		width: 56px;
		height: 56px;
		font-size: 28px;
		cursor: pointer;
	}
	.user-avatar {
		background: var(--text-color);
		color: var(--card-bg);
		font-size: 12px;
		font-weight: 600;
		border-color: transparent;
	}
	.star-btn {
		color: var(--text-muted);
		padding: 4px;
		border-radius: 4px;
		display: inline-flex;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.star-btn:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}
	.star-btn.on {
		color: var(--color-yellow);
	}
	.char-name {
		font-size: 15px;
		font-weight: 600;
	}
	.char-tagline {
		font-size: 13px;
		color: var(--text-italic);
		line-height: 1.5;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		min-height: 39px;
	}
	.char-card-actions {
		display: flex;
		gap: 4px;
		margin-top: 6px;
		padding-top: 10px;
		border-top: 1px solid var(--border-color);
	}

	.char-list {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}
	.char-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
		cursor: pointer;
		transition: background 100ms ease;
	}
	.char-row + .char-row {
		border-top: 1px solid var(--border-color);
	}
	.char-row:hover {
		background: var(--bg-hover);
	}
	.char-row-main {
		flex: 1;
		min-width: 0;
	}
	.char-row-name {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 500;
	}
	.chip {
		font-size: 10.5px;
		color: var(--text-muted);
		border: 1px solid var(--border-color);
		border-radius: 4px;
		padding: 0 5px;
		line-height: 16px;
	}
	.char-row-sub {
		font-size: 12.5px;
		color: var(--text-muted);
	}
	.char-row-actions {
		display: flex;
		gap: 2px;
		flex: none;
	}

	/* ============================================================
   Character editor
   ============================================================ */

	.editor-hero {
		display: flex;
		gap: 18px;
		align-items: center;
		margin-top: 8px;
	}
	.emoji-picker-wrap {
		position: relative;
		flex: none;
	}
	.emoji-pop {
		position: absolute;
		z-index: 20;
		top: 62px;
		left: 0;
		width: 232px;
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 2px;
		padding: 8px;
		background: var(--card-bg);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
	}

	.editor-hero-fields {
		flex: 1;
		min-width: 0;
	}
	.title-input {
		width: 100%;
		border: none;
		outline: none;
		background: transparent;
		font-size: 28px;
		font-weight: 700;
		letter-spacing: -0.02em;
		padding: 2px 0;
	}
	.title-input::placeholder {
		color: var(--text-muted);
		opacity: 0.55;
	}
	.sub-input {
		width: 100%;
		border: none;
		outline: none;
		background: transparent;
		font-size: 13.5px;
		color: var(--text-italic);
		padding: 2px 0;
	}
	.sub-input::placeholder {
		color: var(--text-muted);
	}
	.textarea.tall {
		min-height: 170px;
	}
	.tag-input {
		border: none;
		outline: none;
		background: transparent;
		min-width: 160px;
		flex: 1;
		font-size: 13px;
		padding: 2px 0;
	}
	.range {
		width: 100%;
		max-width: 320px;
		appearance: none;
		height: 4px;
		border-radius: 3px;
		background: var(--border-strong);
		outline: none;
	}
	.range::-webkit-slider-thumb {
		appearance: none;
		width: 15px;
		height: 15px;
		border-radius: 50%;
		background: var(--card-bg);
		border: 1px solid var(--border-strong);
		box-shadow: var(--shadow-sm);
		cursor: pointer;
	}
	.range::-moz-range-thumb {
		width: 15px;
		height: 15px;
		border-radius: 50%;
		background: var(--card-bg);
		border: 1px solid var(--border-strong);
		cursor: pointer;
	}
	.range-legend {
		display: flex;
		justify-content: space-between;
		max-width: 320px;
		font-size: 11px;
		color: var(--text-muted);
	}

	.preview-block {
		margin-bottom: 40px;
	}
	.preview-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
		margin-bottom: 10px;
	}
	.preview-bubble {
		display: flex;
		gap: 12px;
		padding: 14px 16px;
		border-radius: var(--radius-lg);
		background: var(--background-secondary);
		border: 1px solid var(--border-color);
	}
	.preview-name {
		font-size: 13px;
		font-weight: 600;
		margin-bottom: 2px;
	}
	.preview-text {
		font-size: 13.5px;
		color: var(--text-italic);
		white-space: pre-wrap;
	}

	/* ============================================================
   Chat
   ============================================================ */

	.chat {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.chat-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 20px;
		border-bottom: 1px solid var(--border-color);
		background: var(--card-bg);
		flex: none;
	}
	.chat-title-wrap {
		min-width: 0;
	}
	.chat-title {
		font-size: 14.5px;
		font-weight: 600;
		padding: 1px 4px;
		margin-left: -4px;
		border-radius: 4px;
		text-align: left;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.chat-title:hover {
		background: var(--bg-hover);
	}
	.title-inline {
		font-size: 14.5px;
		font-weight: 600;
		border: 1px solid var(--accent);
		border-radius: 4px;
		padding: 0 4px;
		background: var(--bg-input);
		outline: none;
	}
	.chat-sub {
		font-size: 11.5px;
		color: var(--text-muted);
	}

	.chat-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 30px 20px 20px;
	}
	.chat-inner {
		max-width: 720px;
		margin: 0 auto;
	}
	.chat-intro {
		text-align: center;
		padding: 12px 0 36px;
	}

	.msg {
		display: flex;
		gap: 12px;
		padding: 12px 8px;
		border-radius: var(--radius-lg);
	}
	.msg:hover {
		background: var(--background-secondary);
	}
	.msg-avatar {
		flex: none;
		padding-top: 2px;
	}
	.msg-body {
		min-width: 0;
		flex: 1;
	}
	.msg-meta {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 2px;
	}
	.msg-name {
		font-size: 13px;
		font-weight: 600;
	}
	.msg-time {
		font-size: 11px;
		color: var(--text-muted);
	}

	.md-space {
		height: 10px;
	}
	.md-li {
		display: flex;
		gap: 8px;
		line-height: 1.72;
		padding-left: 2px;
	}
	.md-marker {
		color: var(--text-muted);
		flex: none;
		min-width: 14px;
	}

	.typing {
		display: flex;
		gap: 4px;
		padding: 8px 0 4px;
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-muted);
		animation: blink 1.1s infinite ease-in-out;
	}
	.typing span:nth-child(2) {
		animation-delay: 0.16s;
	}
	.typing span:nth-child(3) {
		animation-delay: 0.32s;
	}
	@keyframes blink {
		0%,
		80%,
		100% {
			opacity: 0.25;
			transform: translateY(0);
		}
		40% {
			opacity: 1;
			transform: translateY(-2px);
		}
	}

	.suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding: 18px 8px 4px;
	}
	.suggestion {
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		padding: 6px 13px;
		font-size: 12.5px;
		color: var(--text-italic);
		transition:
			background 120ms ease,
			color 120ms ease;
		background: transparent;
		cursor: pointer;
	}
	.suggestion:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}

	.composer-wrap {
		flex: none;
		padding: 8px 20px 18px;
		background: var(--card-bg);
	}
	.composer {
		max-width: 720px;
		margin: 0 auto;
		display: flex;
		align-items: flex-end;
		gap: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--bg-input);
		padding: 8px 8px 8px 12px;
		box-shadow: var(--shadow-sm);
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}
	.composer:focus-within {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}
	.composer-input {
		flex: 1;
		border: none;
		outline: none;
		resize: none;
		background: transparent;
		font-size: 14px;
		line-height: 1.6;
		max-height: 180px;
		padding: 3px 0;
	}
	.composer-hint {
		max-width: 720px;
		margin: 7px auto 0;
		font-size: 11.5px;
		color: var(--text-muted);
		text-align: center;
	}

	/* ============================================================
   Settings
   ============================================================ */

	.tabs {
		display: flex;
		gap: 18px;
		border-bottom: 1px solid var(--border-color);
		margin-bottom: 8px;
	}
	.tab {
		padding: 8px 2px;
		font-size: 13.5px;
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.tab:hover {
		color: var(--text-color);
	}
	.tab.on {
		color: var(--text-color);
		font-weight: 500;
		border-bottom-color: var(--text-color);
	}

	.set-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 24px;
		padding: 16px 0;
		border-bottom: 1px solid var(--border-color);
	}
	.set-row-text {
		min-width: 0;
	}
	.set-row-title {
		font-size: 13.5px;
		font-weight: 500;
	}
	.set-row-desc {
		font-size: 12.5px;
		color: var(--text-muted);
		margin-top: 2px;
	}
	.set-row-control {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.input.compact,
	.select.compact {
		width: 200px;
		height: 30px;
		padding: 0 9px;
	}
	.key-row {
		display: flex;
		gap: 8px;
	}
	.field {
		margin-top: 22px;
	}

	.stat-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 10px;
		margin: 20px 0 8px;
	}
	.stat {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-lg);
		padding: 14px 16px;
	}
	.stat-num {
		font-size: 22px;
		font-weight: 700;
		letter-spacing: -0.02em;
	}
	.stat-label {
		font-size: 12px;
		color: var(--text-muted);
	}
	.foot-note {
		font-size: 12.5px;
		color: var(--text-muted);
		line-height: 1.7;
		padding-bottom: 40px;
	}
	.empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 8px;
		padding: 64px 20px;
		color: var(--text-muted);
		text-align: center;
	}
	/* ============================================================
   Responsive
   ============================================================ */

	@media (max-width: 900px) {
		.page {
			padding: 28px 22px 80px;
		}
		.hide-sm {
			display: none;
		}
		.stat-grid {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 700px) {
		.sidebar {
			width: 190px;
		}
		.page-title {
			font-size: 24px;
		}
		.set-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 10px;
		}
	}
</style>
