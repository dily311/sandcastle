<script lang="ts">
	import { ArrowLeft, ArrowUp, ChevronDown, ChevronRight, Ellipsis, MessageSquarePlus, Plus, Search, Upload } from "@lucide/svelte";
	import { Avatar, Button, IconButton, Input, Segmented } from "$shared/ui";
	import ChatDetail from "./ChatDetail.svelte";
	import { characters, relative, store } from "./mock.svelte";

	// 캐릭터 목록 → 캐릭터 상세 → 대화 상세 (대화 목록·폴더는 대화 상세의 왼쪽 사이드바)
	let charId: string | null = $state(null);
	let convId: string | null = $state(null);
	let query = $state("");
	let tab: "overview" | "assets" | "stats" = $state("overview");
	let more = $state(false);
	let ask = $state("");

	const character = $derived(characters.find((c) => c.id === charId));
	const convsOf = (id: string) => store.convs.filter((c) => c.charId === id).sort((a, b) => a.ago - b.ago);
	const list = $derived(
		characters
			.filter((c) => !query.trim() || c.name.toLowerCase().includes(query.trim().toLowerCase()))
			.map((c) => ({ ...c, convs: convsOf(c.id) }))
			.sort((a, b) => (a.convs[0]?.ago ?? Infinity) - (b.convs[0]?.ago ?? Infinity)),
	);
	const recent = $derived(charId ? convsOf(charId) : []);

	function open(cid: string, conv?: string) {
		charId = cid;
		convId = conv ?? null;
	}
</script>

{#if charId && convId}
	<ChatDetail {convId} {charId} onback={() => (convId = null)} onopen={(id) => (convId = id)} />
{:else if charId && character}
	<div class="detail">
		<div class="banner">
			<IconButton label="캐릭터 목록" variant="soft" onclick={() => (charId = null)}><ArrowLeft /></IconButton>
		</div>
		<header class="profile">
			<div class="portrait"><Avatar name={character.name} size={96} /></div>
			<div class="ident">
				<h1>{character.name}</h1>
				<p class="faint">캐릭터 · 판타지 · 제작자</p>
				<p>{character.intro}</p>
			</div>
			<div class="cta">
				<Button variant="primary" onclick={() => recent[0] && (convId = recent[0].id)}>
					{#snippet icon()}<MessageSquarePlus />{/snippet}
					{recent.length ? "이어서 대화" : "대화 시작"}
				</Button>
				<div class="sub">
					<Button size="sm" variant="ghost">새 대화</Button>
					<IconButton label="더 보기 (편집·내보내기·삭제)" size="sm"><Ellipsis /></IconButton>
				</div>
			</div>
		</header>

		<div class="tabs">
			<Segmented label="캐릭터 탭" bind:value={tab} options={[{ value: "overview", label: "개요" }, { value: "assets", label: "에셋" }, { value: "stats", label: "통계" }]} />
		</div>

		<div class="body">
			{#if tab === "overview"}
				<section>
					<h2>소개</h2>
					<p class="intro" class:clamp={!more}>
						{character.intro}. 숲 깊은 곳의 오두막에서 지친 여행자를 돌보며 살아감. 엘도리아 숲의 오래된 마법과 그림자 송곳니 무리에 대해 누구보다 잘 앎. 다정하지만 숲을 위협하는 존재에게는 단호함.
					</p>
					<button class="more" onclick={() => (more = !more)}>{more ? "접기" : "더 보기"} <ChevronDown class={more ? "up" : ""} /></button>
					<dl class="stats">
						<div><dt>함께한 시간</dt><dd>12.5시간</dd></div>
						<div><dt>대화 수</dt><dd>{recent.length}</dd></div>
						<div><dt>메시지</dt><dd>{recent.reduce((n, c) => n + c.count, 0).toLocaleString()}</dd></div>
					</dl>
				</section>

				<section>
					<div class="sec-head"><h2>바로 질문하기</h2><span class="faint">기록되지 않음</span></div>
					<form class="ask" onsubmit={(e) => e.preventDefault()}>
						<p class="faint">{character.name}의 성격이나 세계관에 대해 질문하기</p>
						<div class="ask-row">
							<input bind:value={ask} placeholder="무엇이든 물어보기…" aria-label="질문" />
							<IconButton label="질문 보내기" size="sm" variant={ask.trim() ? "solid" : "ghost"} disabled={!ask.trim()}><ArrowUp /></IconButton>
						</div>
					</form>
				</section>

				<section>
					<div class="sec-head">
						<h2>최근 대화</h2>
						{#if recent.length}<button class="link" onclick={() => (convId = recent[0].id)}>모두 보기 →</button>{/if}
					</div>
					<ul class="recent">
						{#each recent.slice(0, 3) as c (c.id)}
							<li>
								<button onclick={() => (convId = c.id)}>
									<span><b>{c.title}</b><small>마지막 활동 · {relative(c.ago)} · {c.count}개</small></span>
									<ChevronRight />
								</button>
							</li>
						{:else}
							<li class="none">아직 대화 없음</li>
						{/each}
					</ul>
				</section>
			{:else}
				<p class="none">이 탭은 실제 화면에서 만듦 (에셋은 대화 패널 에셋 탭과 같은 관리 화면)</p>
			{/if}
		</div>
	</div>
{:else}
	<div class="page">
		<header class="head">
			<h1>캐릭터</h1>
			<div class="tools">
				<Input bind:value={query} placeholder="캐릭터 검색" aria-label="캐릭터 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<Button>
					{#snippet icon()}<Upload />{/snippet}
					가져오기
				</Button>
				<Button variant="primary">
					{#snippet icon()}<Plus />{/snippet}
					새 캐릭터
				</Button>
			</div>
		</header>
		<div class="grid">
			{#each list as ch (ch.id)}
				<article class="char">
					<button class="body-btn" onclick={() => open(ch.id)}>
						<Avatar name={ch.name} size={64} />
						<b>{ch.name}</b>
						<span class="muted">{ch.intro}</span>
						<span class="faint">대화 {ch.convs.length}개{#if ch.convs.length} · {relative(ch.convs[0].ago)}{/if}</span>
					</button>
					{#if ch.convs.length}
						<button class="resume" onclick={() => open(ch.id, ch.convs[0].id)}>이어서: {ch.convs[0].title}</button>
					{/if}
				</article>
			{/each}
		</div>
	</div>
{/if}

<style>
	.page {
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 1100px;
		margin: 0 auto var(--space-5);
	}
	.tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: var(--space-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-3);
		max-width: 1100px;
		margin: 0 auto;
	}
	.char {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
		transition: border-color var(--duration-fast) var(--ease);
	}
	.char:hover {
		border-color: var(--accent);
	}
	.body-btn {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-5) var(--space-4) var(--space-4);
		border: none;
		background: none;
		text-align: center;
		cursor: pointer;
	}
	.body-btn b {
		margin-top: var(--space-2);
	}
	.body-btn span {
		font-size: var(--text-sm);
	}
	.resume {
		padding: var(--space-2) var(--space-3);
		border: none;
		border-top: 1px solid var(--border);
		background: var(--surface-2);
		color: var(--accent-text);
		font-size: var(--text-xs);
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.resume:hover {
		background: var(--accent-soft);
	}

	/* ---- 상세 ---- */
	.detail {
		height: 100%;
		overflow: auto;
	}
	.banner {
		height: 200px;
		padding: var(--space-3);
		background:
			radial-gradient(ellipse at 70% 0%, rgb(255 250 220 / 0.55), transparent 40%),
			linear-gradient(180deg, #2f3d47 0%, #24343c 55%, #1a2328 100%);
	}
	.profile {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: var(--space-4);
		align-items: end;
		max-width: 960px;
		margin: -48px auto 0;
		padding: 0 var(--space-6);
	}
	.portrait :global(.avatar) {
		border-radius: var(--radius-lg) !important;
		border: 4px solid var(--bg);
		box-shadow: var(--shadow-md);
	}
	.portrait :global(.avatar img) {
		border-radius: inherit;
	}
	/* 프로필 사진만 배너에 걸치고 글자·버튼은 배너 아래 */
	.ident,
	.cta {
		margin-top: 56px;
	}
	.ident {
		padding-bottom: var(--space-1);
	}
	.ident p {
		font-size: var(--text-sm);
	}
	.cta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--space-1);
		padding-bottom: var(--space-1);
	}
	.sub {
		display: flex;
		align-items: center;
	}
	.tabs {
		max-width: 960px;
		margin: var(--space-5) auto 0;
		padding: 0 var(--space-6) var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.body {
		max-width: 960px;
		margin: 0 auto;
		padding: 0 var(--space-6) var(--space-10);
	}
	section {
		padding: var(--space-6) 0;
		border-bottom: 1px solid var(--border);
	}
	section:last-child {
		border-bottom: none;
	}
	h2 {
		margin-bottom: var(--space-2);
	}
	.intro {
		color: var(--text-muted);
	}
	.intro.clamp {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.more {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		margin-top: var(--space-1);
		padding: 0;
		border: none;
		background: none;
		color: var(--text-muted);
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.more :global(svg) {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.more :global(.up) {
		transform: rotate(180deg);
	}
	.stats {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		margin: var(--space-5) 0 0;
	}
	.stats dt {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.stats dd {
		margin: 0;
		font-size: var(--text-2xl);
		font-weight: 700;
	}
	.sec-head {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
	}
	.sec-head span {
		font-size: var(--text-sm);
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--text-muted);
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.link:hover {
		color: var(--text);
	}
	.ask {
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--surface);
	}
	.ask p {
		font-size: var(--text-sm);
	}
	.ask-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
	.ask input {
		flex: 1;
		border: none;
		background: none;
		outline: none;
	}
	.recent {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.recent button {
		display: flex;
		align-items: center;
		width: 100%;
		padding: var(--space-3) 0;
		border: none;
		border-bottom: 1px solid var(--border);
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.recent li:last-child button {
		border-bottom: none;
	}
	.recent button span {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.recent small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.recent button :global(svg) {
		color: var(--text-faint);
	}
	.recent button:hover b {
		color: var(--accent-text);
	}
	.none {
		padding: var(--space-6);
		text-align: center;
		color: var(--text-faint);
	}
</style>
