<script lang="ts">
	import {
		BookOpen,
		ChevronLeft,
		ChevronRight,
		Database,
		Ellipsis,
		EyeOff,
		Gamepad2,
		House,
		Image,
		KeyRound,
		MessageCircle,
		Moon,
		Plus,
		Regex,
		RefreshCw,
		Search,
		Send,
		Settings,
		SlidersHorizontal,
		Sparkles,
		Sun,
		UserRound,
		Users,
	} from "@lucide/svelte";
	import {
		Avatar,
		Badge,
		Button,
		Card,
		Dialog,
		EmptyState,
		Field,
		IconButton,
		Input,
		Segmented,
		Select,
		Switch,
		Textarea,
	} from "$shared/ui";

	let dark = $state(false);
	let accent = $state("#2f8f8a");
	const accents = ["#2f8f8a", "#d9774f", "#7a6ad8", "#d4577f", "#4a7fd0"];

	$effect(() => {
		document.documentElement.dataset.theme = dark ? "dark" : "light";
		document.documentElement.style.setProperty("--accent", accent);
	});

	const nav = [
		{ icon: House, label: "홈" },
		{ icon: MessageCircle, label: "폰", badge: 3, active: true },
		{ icon: Users, label: "캐릭터" },
		{ icon: UserRound, label: "페르소나" },
		{ icon: BookOpen, label: "로어북" },
		{ icon: SlidersHorizontal, label: "프리셋" },
		{ icon: Regex, label: "정규식" },
		{ icon: KeyRound, label: "API" },
		{ icon: Gamepad2, label: "게임" },
	];
	const chats = [
		{ name: "Seraphina", last: "숲은 아직 위험해요. 조금 더 쉬어요.", time: "오후 10:42", unread: 2, mode: "phone" },
		{ name: "Aria", last: "내일 성 앞에서 만나자!", time: "오후 7:15", unread: 0, mode: "chat" },
		{ name: "나레이터", last: "밤이 깊어 간다…", time: "어제", unread: 0, mode: "chat" },
		{ name: "Min", last: "사진 보냈어", time: "월요일", unread: 1, mode: "phone" },
	];

	let mode: "chat" | "phone" = $state("phone");
	let message = $state("");
	let dialogOpen = $state(false);
	let imagesOn = $state(true);
	let preset = $state("default");
	let name = $state("Seraphina");
</script>

<div class="page">
	<header class="toolbar">
		<div class="brand">
			<span class="logo"><Sparkles /></span>
			<strong>Sandcastle</strong>
			<span class="faint">디자인 미리보기</span>
		</div>
		<div class="tools">
			<div class="swatches" role="radiogroup" aria-label="강조색">
				{#each accents as a (a)}
					<button
						class="swatch"
						class:on={accent === a}
						style:background={a}
						role="radio"
						aria-checked={accent === a}
						aria-label="강조색 {a}"
						onclick={() => (accent = a)}
					></button>
				{/each}
			</div>
			<IconButton label={dark ? "밝은 테마" : "어두운 테마"} variant="soft" onclick={() => (dark = !dark)}>
				{#if dark}<Sun />{:else}<Moon />{/if}
			</IconButton>
		</div>
	</header>

	<!-- ===================== 앱 화면 ===================== -->
	<section class="shell" aria-label="앱 화면 예시">
		<nav class="rail" aria-label="주 메뉴">
			<div class="rail-brand"><span class="logo sm"><Sparkles /></span> Sandcastle</div>
			<ul>
				{#each nav as n (n.label)}
					<li>
						<a href="#{n.label}" class:active={n.active} aria-current={n.active ? "page" : undefined}>
							<n.icon />
							<span>{n.label}</span>
							{#if n.badge}<span class="count">{n.badge}</span>{/if}
						</a>
					</li>
				{/each}
			</ul>
			<ul class="rail-bottom">
				<li><a href="#data"><Database /><span>데이터</span></a></li>
				<li><a href="#settings"><Settings /><span>설정</span></a></li>
			</ul>
		</nav>

		<aside class="list">
			<div class="list-head">
				<h2>폰</h2>
				<IconButton label="새 대화" variant="soft" size="sm"><Plus /></IconButton>
			</div>
			<div class="search">
				<Input placeholder="대화 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
			</div>
			<ul class="conversations">
				{#each chats as c, i (c.name)}
					<li>
						<button class="conv" class:selected={i === 0}>
							<Avatar name={c.name} size={40} status={c.unread ? "unread" : "none"} />
							<span class="conv-main">
								<span class="conv-top">
									<strong>{c.name}</strong>
									<span class="time">{c.time}</span>
								</span>
								<span class="conv-bottom">
									<span class="last">{c.last}</span>
									{#if c.unread}<span class="unread">{c.unread}</span>{/if}
								</span>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		</aside>

		<main class="thread">
			<header class="thread-head">
				<Avatar name="Seraphina" size={34} status="online" />
				<div class="who">
					<strong>Seraphina</strong>
					<span class="faint">숲의 수호자 · Default 프리셋</span>
				</div>
				<Segmented label="대화 모드" bind:value={mode} options={[{ value: "chat", label: "채팅" }, { value: "phone", label: "폰" }]} />
				<IconButton label="대화 설정" onclick={() => (dialogOpen = true)}><Ellipsis /></IconButton>
			</header>

			<div class="messages">
				<div class="day"><span>2026년 11월 8일</span></div>

				<div class="msg ai">
					<Avatar name="Seraphina" size={30} />
					<div class="col">
						<span class="sender">Seraphina</span>
						<div class="bubble">눈을 떴군요. 다행이에요… 숲의 짐승들에게 쫓기다 쓰러졌었어요.</div>
						<figure class="cg">
							<div class="cg-img"><Image /></div>
							<figcaption>amusement</figcaption>
						</figure>
						<div class="meta">
							<span class="swipe">
								<IconButton label="이전 응답" size="sm"><ChevronLeft /></IconButton>
								<span>2 / 3</span>
								<IconButton label="다음 응답" size="sm"><ChevronRight /></IconButton>
							</span>
							<span class="faint">gemini-3.7-flash · 18초</span>
						</div>
					</div>
				</div>

				<div class="msg user">
					<div class="col">
						<div class="bubble">여기가 어디죠? 숲인가요?</div>
						<span class="faint time">오후 10:41</span>
					</div>
				</div>

				<div class="msg ai hidden">
					<Avatar name="Seraphina" size={30} />
					<div class="col">
						<div class="bubble">
							<span class="hidden-tag"><EyeOff /> AI에게 보내지 않는 메시지</span>
							(숨긴 메시지)
						</div>
					</div>
				</div>

				<div class="msg ai">
					<Avatar name="Seraphina" size={30} />
					<div class="col">
						<div class="bubble typing" aria-label="입력 중"><span></span><span></span><span></span></div>
					</div>
				</div>
			</div>

			<form class="composer" onsubmit={(e) => e.preventDefault()}>
				<IconButton label="이미지" size="md"><Image /></IconButton>
				<Textarea bind:value={message} rows={1} autosize maxRows={6} placeholder="메시지 보내기" aria-label="메시지" />
				<IconButton label="다시 생성" size="md"><RefreshCw /></IconButton>
				<IconButton label="보내기" variant="solid" disabled={!message.trim()}><Send /></IconButton>
			</form>
		</main>
	</section>

	<!-- ===================== 컴포넌트 ===================== -->
	<section class="gallery">
		<Card title="버튼" description="주요 동작은 하나만 primary로">
			<div class="row">
				<Button variant="primary">저장</Button>
				<Button>취소</Button>
				<Button variant="soft">
					{#snippet icon()}<Plus />{/snippet}
					캐릭터 추가
				</Button>
				<Button variant="ghost">더 보기</Button>
				<Button variant="danger">삭제</Button>
				<Button variant="primary" loading>생성 중</Button>
			</div>
			<div class="row">
				<Button size="sm">작게</Button>
				<Button size="lg" variant="primary">크게</Button>
				<Badge>chat</Badge>
				<Badge tone="accent">phone</Badge>
				<Badge tone="success">연결됨</Badge>
				<Badge tone="warning">경고 1</Badge>
				<Badge tone="danger">실패</Badge>
			</div>
		</Card>

		<Card title="입력">
			<div class="form">
				<Field label="캐릭터 이름" for="name" required aside={nameCount}>
					<Input id="name" bind:value={name} maxlength={40} />
				</Field>
				<Field label="프리셋" for="preset" hint="대화에 쓸 프롬프트 배치">
					<Select
						id="preset"
						bind:value={preset}
						options={[
							{ value: "default", label: "기본 배치" },
							{ value: "st", label: "Default (SillyTavern)" },
						]}
					/>
				</Field>
				<Field label="API 키" for="key" error="API 인증에 실패했습니다 (401)">
					<Input id="key" type="password" value="sk-••••" invalid />
				</Field>
				<Field label="설명" for="desc">
					<Textarea id="desc" rows={3} placeholder={"{{char}}는…"} />
				</Field>
				<Switch bind:checked={imagesOn} label="캐릭터 이미지" description="키 목록을 보내고 <cg>키</cg>로 받기" />
			</div>
		</Card>

		<Card title="빈 화면" padding="none">
			<EmptyState title="아직 대화가 없어요" description="캐릭터를 고르거나 SillyTavern에서 가져와 대화를 시작하세요.">
				{#snippet icon()}<MessageCircle />{/snippet}
				{#snippet action()}<Button variant="primary">새 대화</Button>{/snippet}
			</EmptyState>
		</Card>
	</section>
</div>

{#snippet nameCount()}{name.length}/40{/snippet}

<Dialog bind:open={dialogOpen} title="대화 설정" description="이 대화에만 적용됩니다">
	<div class="form">
		<Switch bind:checked={imagesOn} label="캐릭터 이미지 사용" />
		<Field label="대화 이름" for="title"><Input id="title" value="Seraphina - Branch #1" /></Field>
	</div>
	{#snippet footer()}
		<Button onclick={() => (dialogOpen = false)}>취소</Button>
		<Button variant="primary" onclick={() => (dialogOpen = false)}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.page {
		display: flex;
		flex-direction: column;
		gap: var(--space-6);
		max-width: 1280px;
		margin: 0 auto;
		padding: var(--space-5);
	}
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.brand,
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-3);
	}
	.logo {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		border-radius: var(--radius-sm);
		background: var(--accent);
		color: var(--text-on-accent);
	}
	.logo :global(svg) {
		width: 17px;
		height: 17px;
	}
	.logo.sm {
		width: 24px;
		height: 24px;
	}
	.logo.sm :global(svg) {
		width: 14px;
		height: 14px;
	}
	.swatches {
		display: flex;
		gap: 6px;
	}
	.swatch {
		width: 20px;
		height: 20px;
		border-radius: 50%;
		border: 2px solid var(--surface);
		box-shadow: 0 0 0 1px var(--border-strong);
		cursor: pointer;
	}
	.swatch.on {
		box-shadow: 0 0 0 2px var(--text);
	}

	/* ---------------- 앱 셸 ---------------- */
	.shell {
		display: grid;
		grid-template-columns: var(--rail-width) 300px 1fr;
		grid-template-rows: minmax(0, 1fr);
		height: 720px;
		border: 1px solid var(--border);
		border-radius: var(--radius-xl);
		overflow: hidden;
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.rail {
		display: flex;
		flex-direction: column;
		padding: var(--space-4) var(--space-3);
		background: var(--bg);
		border-right: 1px solid var(--border);
	}
	.rail-brand {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: 0 var(--space-2) var(--space-4);
		font-weight: 700;
	}
	.rail ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.rail-bottom {
		margin-top: auto !important;
	}
	.rail a {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		height: 36px;
		padding: 0 var(--space-3);
		border-radius: var(--radius-sm);
		color: var(--text-muted);
		font-weight: 500;
		text-decoration: none;
	}
	.rail a :global(svg) {
		width: 18px;
		height: 18px;
	}
	.rail a:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.rail a.active {
		background: var(--surface);
		color: var(--text);
		box-shadow: var(--shadow-sm);
	}
	.rail a.active :global(svg) {
		color: var(--accent);
	}
	.count {
		margin-left: auto;
		min-width: 20px;
		height: 20px;
		padding: 0 6px;
		border-radius: var(--radius-full);
		background: var(--accent);
		color: var(--text-on-accent);
		font-size: var(--text-xs);
		font-weight: 700;
		display: grid;
		place-items: center;
	}

	/* ---------------- 대화 목록 ---------------- */
	.list {
		display: flex;
		flex-direction: column;
		border-right: 1px solid var(--border);
		background: var(--surface);
		min-width: 0;
	}
	.list-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: var(--space-4) var(--space-4) var(--space-3);
	}
	.search {
		padding: 0 var(--space-4) var(--space-3);
	}
	.conversations {
		list-style: none;
		margin: 0;
		padding: 0 var(--space-2);
		overflow: auto;
	}
	.conv {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-3);
		border: none;
		border-radius: var(--radius-md);
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.conv:hover {
		background: var(--surface-hover);
	}
	.conv.selected {
		background: var(--accent-soft);
	}
	.conv-main {
		display: flex;
		flex-direction: column;
		gap: 2px;
		min-width: 0;
		flex: 1;
	}
	.conv-top,
	.conv-bottom {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.time {
		font-size: var(--text-xs);
		color: var(--text-faint);
		flex: none;
	}
	.last {
		font-size: var(--text-sm);
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.unread {
		flex: none;
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		border-radius: var(--radius-full);
		background: var(--accent);
		color: var(--text-on-accent);
		font-size: 11px;
		font-weight: 700;
		display: grid;
		place-items: center;
	}

	/* ---------------- 대화 ---------------- */
	.thread {
		display: flex;
		flex-direction: column;
		min-width: 0;
		background: var(--bg);
	}
	.thread-head {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.who {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		line-height: 1.3;
	}
	.who span {
		font-size: var(--text-xs);
	}
	.messages {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		padding: var(--space-5);
		overflow: auto;
	}
	.day {
		align-self: center;
		font-size: var(--text-xs);
		color: var(--text-faint);
		background: var(--bg-sunken);
		padding: 2px var(--space-3);
		border-radius: var(--radius-full);
	}
	.msg {
		display: flex;
		gap: var(--space-2);
		max-width: 78%;
	}
	.msg.user {
		align-self: flex-end;
	}
	.col {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}
	.user .col {
		align-items: flex-end;
	}
	.sender {
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-muted);
	}
	.bubble {
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-lg);
		background: var(--bubble-ai);
		color: var(--bubble-ai-text);
		border: 1px solid var(--border);
		border-top-left-radius: var(--radius-xs);
		box-shadow: var(--shadow-sm);
	}
	.user .bubble {
		background: var(--bubble-user);
		color: var(--bubble-user-text);
		border: none;
		border-top-left-radius: var(--radius-lg);
		border-top-right-radius: var(--radius-xs);
	}
	.cg {
		margin: 0;
		width: 220px;
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid var(--border);
		background: var(--surface);
	}
	.cg-img {
		display: grid;
		place-items: center;
		height: 150px;
		background: linear-gradient(135deg, var(--accent-soft), var(--bg-sunken));
		color: var(--accent-text);
	}
	.cg figcaption {
		padding: 4px var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--text-xs);
	}
	.swipe {
		display: inline-flex;
		align-items: center;
		gap: 2px;
		color: var(--text-muted);
	}
	.hidden .bubble {
		background: transparent;
		border-style: dashed;
		color: var(--text-faint);
		box-shadow: none;
	}
	.hidden-tag {
		display: flex;
		align-items: center;
		gap: 4px;
		font-size: var(--text-xs);
		font-weight: 600;
	}
	.hidden-tag :global(svg) {
		width: 13px;
		height: 13px;
	}
	.typing {
		display: flex;
		gap: 4px;
		padding: var(--space-3);
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-faint);
		animation: blink 1.2s infinite ease-in-out;
	}
	.typing span:nth-child(2) {
		animation-delay: 0.15s;
	}
	.typing span:nth-child(3) {
		animation-delay: 0.3s;
	}
	@keyframes blink {
		0%,
		80%,
		100% {
			opacity: 0.3;
			transform: translateY(0);
		}
		40% {
			opacity: 1;
			transform: translateY(-2px);
		}
	}
	.composer {
		display: flex;
		align-items: flex-end;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-4);
		border-top: 1px solid var(--border);
		background: var(--surface);
	}
	.composer :global(textarea) {
		resize: none;
		border-radius: var(--radius-lg);
	}

	/* ---------------- 갤러리 ---------------- */
	.gallery {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
		gap: var(--space-5);
		align-items: start;
	}
	.row {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.row + .row {
		margin-top: var(--space-3);
	}
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
</style>
