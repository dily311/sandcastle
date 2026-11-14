<script lang="ts">
	import { ChevronRight, Ellipsis, MessageSquarePlus, Plus, Search, Upload } from "@lucide/svelte";
	import { Avatar, Button, IconButton, Input, Segmented } from "$shared/ui";
	import ChatDetail from "./ChatDetail.svelte";
	import { characters, current, go, relative, store } from "./mock.svelte";

	// 캐릭터 목록 → 캐릭터 상세 → 대화 상세 (대화 목록·폴더는 대화 상세의 왼쪽 사이드바)
	// 위치는 앱 전체 이동 기록(nav)에 있음 → 타이틀바 앞으로/뒤로 버튼으로 오감
	const charId = $derived(current().charId ?? null);
	const convId = $derived(current().convId ?? null);
	let query = $state("");
	let tab: "overview" | "recent" | "assets" | "stats" = $state("overview");

	// 제작자 소개 (카드 creator_notes 등). 실제 구현에서는 DOMPurify로 정리한 뒤 샌드박스 iframe에 넣음
	const creatorHtml = `<!doctype html><meta charset="utf-8"><style>
		body{margin:0;font-family:Georgia,serif;background:#f6efe2;color:#3b2f22}
		.card{padding:28px 32px;border:1px solid #d9c7a4;border-radius:14px;background:linear-gradient(180deg,#fbf6ec,#f1e6d1)}
		h1{margin:0 0 4px;font-size:26px;letter-spacing:.02em} .sub{color:#8a6d45;font-style:italic;margin-bottom:16px}
		.tags span{display:inline-block;margin:0 6px 6px 0;padding:2px 10px;border-radius:999px;background:#e7d6b4;font-size:12px}
		blockquote{margin:16px 0 0;padding-left:14px;border-left:3px solid #b08a52;color:#5a4630}
	</style><div class="card"><h1>Seraphina</h1><div class="sub">엘도리아 숲의 수호자</div>
	<div class="tags"><span>판타지</span><span>힐러</span><span>다정함</span></div>
	<p>숲 깊은 곳의 오두막에서 지친 여행자를 돌봄. 그림자 송곳니 무리로부터 숲을 지키는 오래된 마법의 주인.</p>
	<blockquote>"여기선 아무도 당신을 해치지 못해요."</blockquote></div>`;
	const frameH = 300;

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
		go({ page: "character", charId: cid, convId: conv ?? null });
	}
</script>

{#if charId && convId}
	<ChatDetail {convId} {charId} onopen={(id) => go({ page: "character", charId, convId: id })} />
{:else if charId && character}
	<div class="detail">
		<div class="banner"></div>
		<header class="profile">
			<div class="portrait"><Avatar name={character.name} size={96} /></div>
			<div class="ident">
				<h1>{character.name}</h1>
				<p class="faint">캐릭터 · 판타지 · 제작자</p>
				<p>{character.intro}</p>
			</div>
			<div class="cta">
				<Button variant="primary" onclick={() => recent[0] && open(charId!, recent[0].id)}>
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
			<Segmented label="캐릭터 탭" bind:value={tab} options={[{ value: "overview", label: "개요" }, { value: "recent", label: "최근 대화" }, { value: "assets", label: "에셋" }, { value: "stats", label: "통계" }]} />
		</div>

		<div class="body">
			{#if tab === "overview"}
				<!-- 봇 제작자가 쓴 HTML/CSS. 스크립트·외부 요청이 막힌 샌드박스 iframe에 그대로 표시 -->
				<iframe class="creator" title="{character.name} 소개 (제작자 작성)" sandbox="" srcdoc={creatorHtml} style:height="{frameH}px"></iframe>
			{:else if tab === "recent"}
				<ul class="recent">
					{#each recent as c (c.id)}
						<li>
							<button onclick={() => open(charId!, c.id)}>
								<span><b>{c.title}</b><small>마지막 활동 · {relative(c.ago)} · {c.count}개</small></span>
								<ChevronRight />
							</button>
						</li>
					{:else}
						<li class="none">아직 대화 없음</li>
					{/each}
				</ul>
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

	.creator {
		display: block;
		width: 100%;
		margin-top: var(--space-5);
		border: none;
		border-radius: var(--radius-lg);
		background: transparent;
	}
	.recent {
		margin-top: var(--space-3) !important;
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
