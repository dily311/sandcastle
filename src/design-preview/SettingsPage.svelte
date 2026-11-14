<script lang="ts">
	import {
		Database,
		Keyboard,
		KeyRound,
		Music,
		Palette,
		Regex,
		ScrollText,
		Search,
		Settings2,
		SlidersHorizontal,
	} from "@lucide/svelte";
	import { Badge, Button, Input, Segmented, Select, Switch } from "$shared/ui";
	import { ACCENTS } from "$shared/ui/accents";

	let { accent = $bindable(), dark = $bindable() }: { accent: string; dark: boolean } = $props();

	type Tab = "general" | "theme" | "shortcut" | "data" | "api" | "preset" | "regex" | "widget" | "log";
	let tab: Tab = $state("theme");
	let query = $state("");

	const groups: { title: string; items: { id: Tab; label: string; icon: typeof Settings2 }[] }[] = [
		{
			title: "앱",
			items: [
				{ id: "general", label: "기본 설정", icon: Settings2 },
				{ id: "theme", label: "테마", icon: Palette },
				{ id: "shortcut", label: "단축키", icon: Keyboard },
				{ id: "widget", label: "위젯", icon: Music },
			],
		},
		{
			title: "AI",
			items: [
				{ id: "api", label: "API 키", icon: KeyRound },
				{ id: "preset", label: "프리셋", icon: SlidersHorizontal },
				{ id: "regex", label: "정규식", icon: Regex },
			],
		},
		{
			title: "관리",
			items: [
				{ id: "data", label: "데이터", icon: Database },
				{ id: "log", label: "로그", icon: ScrollText },
			],
		},
	];

	let mode: "system" | "light" | "dark" = $state("light");
	$effect(() => {
		if (mode !== "system") dark = mode === "dark";
	});
	let fontSize = $state("16");
	let frameless = $state(true);
	let sendOnEnter = $state(true);
	let timestamps = $state(false);
	let lang = $state("ko");

	const shortcuts = [
		{ group: "전역", items: [["위젯 열기/닫기", ["Ctrl", "Shift", "W"]], ["설정 열기", ["Ctrl", ","]], ["검색", ["Ctrl", "K"]]] },
		{ group: "대화", items: [["보내기", ["Enter"]], ["줄바꿈", ["Shift", "Enter"]], ["마지막 응답 다시 생성", ["Ctrl", "R"]], ["이전/다음 응답", ["Alt", "←/→"]], ["대화 패널", ["Ctrl", "\\"]]] },
		{ group: "게임", items: [["로그 보기", ["L"]], ["빠른 저장", ["F5"]]] },
	] as const;

	const logs = [
		{ level: "error", time: "22:41:03", src: "chat", msg: "AI_PROVIDER_RATE_LIMIT · API 요청 한도 초과 (429)" },
		{ level: "warn", time: "22:40:51", src: "import", msg: "Default.json: 성격/시나리오 블록은 비워 둠" },
		{ level: "info", time: "22:40:12", src: "task", msg: "중단된 작업 1개를 interrupted로 표시" },
		{ level: "info", time: "22:40:11", src: "app", msg: "DB 마이그레이션 최신 (111)" },
	];
</script>

<div class="settings">
	<nav class="side" aria-label="설정 메뉴">
		<div class="search">
			<Input bind:value={query} placeholder="설정 검색" aria-label="설정 검색">
				{#snippet leading()}<Search />{/snippet}
			</Input>
		</div>
		{#each groups as g (g.title)}
			<p class="group">{g.title}</p>
			<ul>
				{#each g.items as it (it.id)}
					<li>
						<button class:on={tab === it.id} aria-current={tab === it.id ? "page" : undefined} onclick={() => (tab = it.id)}>
							<it.icon />{it.label}
						</button>
					</li>
				{/each}
			</ul>
		{/each}
	</nav>

	<div class="body">
		<div class="inner">
			{#if tab === "theme"}
				<h1>테마</h1>
				<div class="group-card">
					<div class="item">
						<div class="txt"><b>화면 모드</b><span>시스템 설정을 따르거나 직접 고름</span></div>
						<Segmented label="화면 모드" bind:value={mode} options={[{ value: "system", label: "시스템" }, { value: "light", label: "밝게" }, { value: "dark", label: "어둡게" }]} />
					</div>
					<div class="item">
						<div class="txt"><b>강조색</b><span>버튼·선택 표시·내 메시지 색</span></div>
						<div class="accents" role="radiogroup" aria-label="강조색">
							{#each ACCENTS as a (a.id)}
								<button
									role="radio"
									aria-checked={accent === a.id}
									aria-label={a.label}
									title={a.label}
									class:on={accent === a.id}
									style:background={dark ? a.dark : a.light}
									onclick={() => (accent = a.id)}
								></button>
							{/each}
						</div>
					</div>
					<div class="item">
						<div class="txt"><b>글자 크기</b><span>앱 전체 기본 글자 크기</span></div>
						<Select bind:value={fontSize} options={["14", "15", "16", "17", "18"].map((v) => ({ value: v, label: `${v}px` }))} />
					</div>
					<div class="item">
						<div class="txt"><b>프레임 없는 창</b><span>운영체제 제목 표시줄 대신 앱 헤더 사용</span></div>
						<Switch bind:checked={frameless} label="프레임 없는 창" hideLabel />
					</div>
				</div>
				<h3 class="sub">사용자 테마</h3>
				<div class="group-card">
					<div class="item">
						<div class="txt"><b>Catppuccin</b><span>v1.0.0 · test</span></div>
						<div class="btns"><Button size="sm">편집</Button><Button size="sm" variant="soft">적용</Button></div>
					</div>
					<div class="item">
						<div class="txt"><b>테마 추가</b><span>CSS 변수로 토큰을 덮어씀</span></div>
						<Button size="sm">새 테마</Button>
					</div>
				</div>
			{:else if tab === "general"}
				<h1>기본 설정</h1>
				<div class="group-card">
					<div class="item">
						<div class="txt"><b>언어</b><span>표시 언어</span></div>
						<Select bind:value={lang} options={[{ value: "ko", label: "한국어" }, { value: "en", label: "English" }]} />
					</div>
					<div class="item">
						<div class="txt"><b>Enter로 보내기</b><span>끄면 Ctrl+Enter로 보냄</span></div>
						<Switch bind:checked={sendOnEnter} label="Enter로 보내기" hideLabel />
					</div>
					<div class="item">
						<div class="txt"><b>메시지 시각 표시</b><span>대화에서 각 메시지 시각 보이기</span></div>
						<Switch bind:checked={timestamps} label="메시지 시각" hideLabel />
					</div>
				</div>
			{:else if tab === "shortcut"}
				<h1>단축키</h1>
				<p class="lead">항목을 눌러 새 키 조합 입력. 겹치면 경고.</p>
				{#each shortcuts as g (g.group)}
					<h3 class="sub">{g.group}</h3>
					<div class="group-card">
						{#each g.items as [label, keys] (label)}
							<div class="item">
								<div class="txt"><b>{label}</b></div>
								<button class="keys" aria-label="{label} 단축키 바꾸기">
									{#each keys as k, i (k + i)}<kbd>{k}</kbd>{/each}
								</button>
							</div>
						{/each}
					</div>
				{/each}
			{:else if tab === "log"}
				<h1>로그</h1>
				<p class="lead">이번 실행 동안 생긴 오류·알림. 앱을 끄면 지워짐.</p>
				<div class="group-card log">
					{#each logs as l (l.time + l.msg)}
						<div class="log-row">
							<Badge tone={l.level === "error" ? "danger" : l.level === "warn" ? "warning" : "neutral"}>{l.level}</Badge>
							<span class="t">{l.time}</span>
							<span class="src">{l.src}</span>
							<span class="m">{l.msg}</span>
						</div>
					{/each}
				</div>
			{:else}
				<h1>{groups.flatMap((g) => g.items).find((i) => i.id === tab)?.label}</h1>
				<p class="lead">이 탭의 화면은 다음 단계에서 만듦.</p>
			{/if}
		</div>
	</div>
</div>

<style>
	.settings {
		display: grid;
		grid-template-columns: 240px minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.side {
		padding: var(--space-3);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: auto;
	}
	.search {
		margin-bottom: var(--space-3);
	}
	.group {
		margin: var(--space-4) var(--space-2) var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.side ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.side button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.side button :global(svg) {
		width: 16px;
		height: 16px;
	}
	.side button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.side button.on {
		background: var(--surface-active);
		color: var(--text);
		font-weight: 600;
	}
	.body {
		overflow: auto;
		background: var(--surface-2);
	}
	.inner {
		max-width: 760px;
		margin: 0 auto;
		padding: var(--space-8) var(--space-6);
	}
	h1 {
		font-size: var(--text-xl);
		margin-bottom: var(--space-4);
	}
	.sub {
		margin: var(--space-6) 0 var(--space-3);
	}
	.lead {
		margin: -8px 0 var(--space-4);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.group-card {
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
		padding: 0 var(--space-5);
	}
	.item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-6);
		padding: var(--space-4) 0;
	}
	.item + .item {
		border-top: 1px solid var(--border);
	}
	.txt {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.txt b {
		font-weight: 600;
	}
	.txt span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.item :global(.select) {
		min-width: 140px;
	}
	.btns {
		display: flex;
		gap: var(--space-2);
	}
	.accents {
		display: flex;
		gap: 8px;
		flex-wrap: wrap;
		justify-content: flex-end;
	}
	.accents button {
		width: 24px;
		height: 24px;
		border-radius: 50%;
		border: 2px solid var(--bg-sunken);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
	}
	.accents button.on {
		box-shadow: 0 0 0 2px var(--text);
	}
	.keys {
		display: flex;
		gap: 4px;
		padding: 4px 6px;
		border: 1px solid transparent;
		border-radius: var(--radius-xs);
		background: transparent;
		cursor: pointer;
	}
	.keys:hover {
		border-color: var(--border-strong);
		background: var(--surface);
	}
	kbd {
		min-width: 24px;
		padding: 2px 6px;
		border: 1px solid var(--border-strong);
		border-bottom-width: 2px;
		border-radius: 5px;
		background: var(--surface);
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		font-weight: 600;
		text-align: center;
	}
	.log {
		padding: var(--space-2) var(--space-4);
		font-size: var(--text-sm);
	}
	.log-row {
		display: grid;
		grid-template-columns: 56px 64px 56px 1fr;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-2) 0;
	}
	.log-row + .log-row {
		border-top: 1px solid var(--border);
	}
	.t,
	.src {
		color: var(--text-faint);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
	@media (max-width: 720px) {
		.settings {
			grid-template-columns: minmax(0, 1fr);
		}
		.side {
			display: none;
		}
	}
</style>
