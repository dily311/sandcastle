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
		PanelLeft,
		PanelRight,
		Pencil,
		Plus,
		RefreshCw,
		Send,
		Trash2,
	} from "@lucide/svelte";
	import { Avatar, Badge, Button, IconButton, Segmented, Select, Switch, Textarea } from "$shared/ui";

	let showList = $state(true);
	let showPanel = $state(true);
	let panelTab: "settings" | "toc" = $state("settings");
	let draft = $state("");
	let persona = $state("min");
	let preset = $state("st-default");
	let api = $state("gemini");
	let imagesOn = $state(true);
	let regexOn = $state([true, false, true]);

	const convs = [
		{ title: "숲에서 깨어남", char: "Seraphina", n: 12 },
		{ title: "Branch #1", char: "Seraphina", n: 5 },
		{ title: "성 앞의 약속", char: "Aria", n: 48 },
	];
	/** n분 전 → 상대 시간 (실제 구현은 공용 유틸로) */
	function relative(min: number): string {
		if (min < 1) return "방금";
		if (min < 60) return `${min}분 전`;
		if (min < 60 * 24) return `${Math.floor(min / 60)}시간 전`;
		return `${Math.floor(min / 60 / 24)}일 전`;
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
				<strong>대화</strong>
				<IconButton label="새 대화" size="sm" variant="soft"><Plus /></IconButton>
			</div>
			<ul>
				{#each convs as c, i (c.title)}
					<li><button class:on={i === 0}><Avatar name={c.char} size={28} /><span><b>{c.title}</b><small>{c.char} · {c.n}개</small></span></button></li>
				{/each}
			</ul>
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
						<IconButton label="다음 응답" size="sm"><ChevronRight /></IconButton>
						<IconButton label="다시 생성" size="sm"><RefreshCw /></IconButton>
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
				<Segmented label="패널" bind:value={panelTab} options={[{ value: "settings", label: "설정" }, { value: "toc", label: "목차" }]} />
			</div>
			{#if panelTab === "settings"}
				<div class="panel-body">
					<section>
						<h4>이 대화</h4>
						<label class="row"><span>페르소나</span><Select bind:value={persona} options={[{ value: "min", label: "Min" }, { value: "default", label: "기본" }]} /></label>
						<label class="row"><span>프리셋</span><Select bind:value={preset} options={[{ value: "st-default", label: "Default (ST)" }, { value: "none", label: "기본 배치" }]} /></label>
						<label class="row"><span>API</span><Select bind:value={api} options={[{ value: "gemini", label: "Gemini · 3.7 flash" }, { value: "claude", label: "Claude" }]} /></label>
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
		grid-template-columns: 240px minmax(0, 1fr) 300px;
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.chat.no-list {
		grid-template-columns: minmax(0, 1fr) 300px;
	}
	.chat.no-panel {
		grid-template-columns: 240px minmax(0, 1fr);
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
		justify-content: space-between;
		align-items: center;
		padding: var(--space-3) var(--space-3) var(--space-2) var(--space-4);
	}
	.convs ul {
		list-style: none;
		margin: 0;
		padding: 0 var(--space-2);
	}
	.convs li button {
		display: flex;
		gap: var(--space-2);
		align-items: center;
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.convs li button:hover {
		background: var(--surface-hover);
	}
	.convs li button.on {
		background: var(--surface);
		box-shadow: var(--shadow-sm);
	}
	.convs span {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.convs b {
		font-size: var(--text-sm);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.convs small {
		color: var(--text-faint);
		font-size: var(--text-xs);
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
