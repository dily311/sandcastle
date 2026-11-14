<script lang="ts">
	import {
		BatteryFull,
		ChevronLeft,
		ChevronRight,
		Contact,
		Image,
		Menu,
		MessageCircle,
		Plus,
		Search,
		Send,
		Settings,
		Signal,
		StickyNote,
		Wifi,
	} from "@lucide/svelte";
	import { Avatar } from "$shared/ui";

	// ---- 페르소나별 폰 (연락처·대화·배경이 페르소나마다 다름) ----
	interface Persona {
		id: string;
		name: string;
		wallpaper: string;
		contacts: { name: string; status: string }[];
		chats: { name: string; last: string; time: string; unread: number }[];
	}
	const personas: Persona[] = [
		{
			id: "min",
			name: "Min",
			wallpaper: "sea",
			contacts: [
				{ name: "Seraphina", status: "숲의 오두막" },
				{ name: "Aria", status: "기사단 훈련 중" },
				{ name: "카페 사장님", status: "" },
			],
			chats: [
				{ name: "Seraphina", last: "제 오두막이에요. 조금 더 쉬어요.", time: "오후 10:42", unread: 2 },
				{ name: "Aria", last: "내일 성 앞에서 만나자!", time: "오후 7:15", unread: 0 },
				{ name: "카페 사장님", last: "오늘 휴무예요~", time: "11. 2.", unread: 0 },
			],
		},
		{
			id: "haru",
			name: "하루",
			wallpaper: "dusk",
			contacts: [
				{ name: "선배", status: "도서관" },
				{ name: "룸메이트", status: "알바 중" },
			],
			chats: [
				{ name: "선배", last: "과제 자료 보냈어", time: "오후 3:02", unread: 1 },
				{ name: "룸메이트", last: "오늘 늦어?", time: "어제", unread: 0 },
			],
		},
	];
	const wallpapers: Record<string, string> = {
		sea: "linear-gradient(160deg, #9ad3cf 0%, #4f8fa8 55%, #2c4f6b 100%)",
		dusk: "linear-gradient(160deg, #f6c39a 0%, #c9739a 50%, #4b3d74 100%)",
		sand: "linear-gradient(160deg, #f3e3c6 0%, #d8b98a 55%, #8f7356 100%)",
	};

	type Screen = "home" | "messages" | "room" | "contacts" | "settings";
	let screen: Screen = $state("home");
	let personaId = $state("min");
	let wallpaper = $state("sea");
	/** 기기 목업 안에 / 화면 전체로 */
	let display: "mockup" | "full" = $state("mockup");
	let room = $state("Seraphina");
	let draft = $state("");

	const persona = $derived(personas.find((p) => p.id === personaId)!);
	const unread = $derived(persona.chats.reduce((n, c) => n + c.unread, 0));

	function switchPersona(id: string) {
		personaId = id;
		wallpaper = personas.find((p) => p.id === id)!.wallpaper;
	}
	function openRoom(name: string) {
		room = name;
		screen = "room";
	}
</script>

<div class="stage" class:full={display === "full"}>
	<div class="device" class:framed={display === "mockup"} style:--wall={wallpapers[wallpaper]}>
		<div class="status" class:on-wall={screen === "home"}>
			<span>10:42</span><span class="icons"><Signal /><Wifi /><BatteryFull /></span>
		</div>

		{#if screen === "home"}
			<div class="home">
				<div class="clock"><b>10:42</b><span>11월 8일 일요일</span></div>
				<div class="apps">
					<button class="app" onclick={() => (screen = "messages")}>
						<span class="ic msg"><MessageCircle />{#if unread}<i>{unread}</i>{/if}</span>메시지
					</button>
					<button class="app" onclick={() => (screen = "contacts")}><span class="ic contact"><Contact /></span>연락처</button>
					<button class="app"><span class="ic photo"><Image /></span>사진</button>
					<button class="app"><span class="ic memo"><StickyNote /></span>메모</button>
					<button class="app" onclick={() => (screen = "settings")}><span class="ic set"><Settings /></span>설정</button>
				</div>
				<div class="dock">
					<button class="app" aria-label="메시지" onclick={() => (screen = "messages")}><span class="ic msg"><MessageCircle /></span></button>
					<button class="app" aria-label="연락처" onclick={() => (screen = "contacts")}><span class="ic contact"><Contact /></span></button>
					<button class="app" aria-label="설정" onclick={() => (screen = "settings")}><span class="ic set"><Settings /></span></button>
				</div>
			</div>
		{:else if screen === "messages"}
			<div class="app-bar"><h2>채팅</h2><span class="bar-icons"><Search /><Plus /></span></div>
			<ul class="list">
				{#each persona.chats as c (c.name)}
					<li>
						<button onclick={() => openRoom(c.name)}>
							<Avatar name={c.name} size={46} />
							<span class="main">
								<span class="row"><strong>{c.name}</strong><span class="time">{c.time}</span></span>
								<span class="row"><span class="last">{c.last}</span>{#if c.unread}<span class="badge">{c.unread}</span>{/if}</span>
							</span>
						</button>
					</li>
				{/each}
			</ul>
		{:else if screen === "room"}
			<div class="room">
				<div class="room-bar">
					<button class="plain" aria-label="뒤로" onclick={() => (screen = "messages")}><ChevronLeft /></button>
					<strong>{room}</strong>
					<Menu />
				</div>
				<div class="msgs">
					<div class="date">2026년 11월 8일 일요일</div>
					<div class="them">
						<Avatar name={room} size={36} />
						<div class="stack">
							<span class="name">{room}</span>
							<div class="line"><div class="b">눈 떴어요?</div></div>
							<div class="line"><div class="b">어제 숲에서 쓰러져 있길래 데려왔어요</div><span class="t">오후 10:40</span></div>
						</div>
					</div>
					<div class="me">
						<div class="line"><span class="t"><span class="read">1</span>오후 10:41</span><div class="b">여기 어디예요??</div></div>
					</div>
				</div>
				<form class="input" onsubmit={(e) => e.preventDefault()}>
					<button type="button" class="plain" aria-label="첨부"><Plus /></button>
					<input bind:value={draft} placeholder="메시지 입력" aria-label="메시지" />
					<button class="send" class:on={!!draft.trim()} aria-label="보내기"><Send /></button>
				</form>
			</div>
		{:else if screen === "contacts"}
			<div class="app-bar"><h2>연락처</h2><span class="bar-icons"><Search /><Plus /></span></div>
			<div class="me-card">
				<Avatar name={persona.name} size={52} />
				<div><strong>{persona.name}</strong><span>내 프로필</span></div>
			</div>
			<p class="section">친구 {persona.contacts.length}</p>
			<ul class="list">
				{#each persona.contacts as c (c.name)}
					<li>
						<button onclick={() => openRoom(c.name)}>
							<Avatar name={c.name} size={40} />
							<span class="main"><strong>{c.name}</strong>{#if c.status}<span class="last">{c.status}</span>{/if}</span>
						</button>
					</li>
				{/each}
			</ul>
		{:else if screen === "settings"}
			<div class="app-bar"><h2>설정</h2></div>
			<div class="settings">
				<button class="profile">
					<Avatar name={persona.name} size={52} />
					<span><strong>{persona.name}</strong><small>이 폰의 주인 (페르소나)</small></span>
					<ChevronRight />
				</button>
				<p class="section">페르소나 바꾸기</p>
				<div class="group">
					{#each personas as p (p.id)}
						<button class="opt" onclick={() => switchPersona(p.id)}>
							<Avatar name={p.name} size={28} />{p.name}
							{#if p.id === personaId}<span class="check">✓</span>{/if}
						</button>
					{/each}
				</div>
				<p class="section">배경화면</p>
				<div class="walls">
					{#each Object.entries(wallpapers) as [id, bg] (id)}
						<button
							class="wall"
							class:on={wallpaper === id}
							style:background={bg}
							aria-label="배경 {id}"
							onclick={() => (wallpaper = id)}
						></button>
					{/each}
				</div>
				<p class="section">화면 표시</p>
				<div class="group">
					<button class="opt" onclick={() => (display = "mockup")}>기기 모양으로 보기{#if display === "mockup"}<span class="check">✓</span>{/if}</button>
					<button class="opt" onclick={() => (display = "full")}>화면 가득 보기{#if display === "full"}<span class="check">✓</span>{/if}</button>
				</div>
			</div>
		{/if}

		<button class="home-indicator" aria-label="홈으로" onclick={() => (screen = "home")}></button>
	</div>
</div>

<style>
	.stage {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: var(--space-6);
		background:
			radial-gradient(circle at 30% 20%, var(--accent-soft), transparent 55%),
			var(--bg-sunken);
		overflow: auto;
	}
	.stage.full {
		padding: 0;
	}
	.device {
		position: relative;
		display: flex;
		flex-direction: column;
		width: 100%;
		height: 100%;
		background: #ffffff;
		color: #191919;
		overflow: hidden;
		--phone-accent: var(--accent);
	}
	.device.framed {
		width: 380px;
		height: min(780px, 100%);
		flex: none;
		border-radius: 46px;
		border: 10px solid #1b1a18;
		box-shadow: var(--shadow-lg), inset 0 0 0 1px #000;
	}
	.status {
		display: flex;
		justify-content: space-between;
		padding: 10px 22px 6px;
		font-size: 13px;
		font-weight: 600;
		flex: none;
	}
	.status.on-wall {
		position: absolute;
		inset: 0 0 auto;
		z-index: 1;
		color: white;
	}
	.icons {
		display: flex;
		gap: 4px;
	}
	.icons :global(svg) {
		width: 14px;
		height: 14px;
	}

	/* ---- 홈 ---- */
	.home {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: 64px 22px 30px;
		background: var(--wall);
		color: white;
	}
	.clock {
		display: flex;
		flex-direction: column;
		align-items: center;
		margin-bottom: 40px;
		text-shadow: 0 1px 6px rgb(0 0 0 / 0.2);
	}
	.clock b {
		font-size: 58px;
		font-weight: 300;
		line-height: 1;
	}
	.clock span {
		font-size: 14px;
	}
	.apps {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 18px 8px;
	}
	.app {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		border: none;
		background: none;
		color: inherit;
		font-size: 11.5px;
		cursor: pointer;
	}
	.ic {
		position: relative;
		display: grid;
		place-items: center;
		width: 56px;
		height: 56px;
		border-radius: 16px;
		color: white;
		box-shadow: 0 4px 10px rgb(0 0 0 / 0.15);
	}
	.ic :global(svg) {
		width: 28px;
		height: 28px;
	}
	.ic i {
		position: absolute;
		top: -5px;
		right: -5px;
		min-width: 20px;
		height: 20px;
		padding: 0 5px;
		border-radius: 10px;
		background: #ff3b30;
		font-style: normal;
		font-size: 12px;
		font-weight: 700;
		display: grid;
		place-items: center;
	}
	.msg {
		background: var(--accent);
	}
	.contact {
		background: #8e8e93;
	}
	.photo {
		background: linear-gradient(135deg, #ffb340, #ff5e7e);
	}
	.memo {
		background: #f2c94c;
	}
	.set {
		background: #636366;
	}
	.dock {
		margin-top: auto;
		display: flex;
		justify-content: space-around;
		padding: 12px;
		border-radius: 30px;
		background: rgb(255 255 255 / 0.25);
		backdrop-filter: blur(12px);
	}

	/* ---- 공통 앱 화면 ---- */
	.app-bar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 8px 18px 12px;
	}
	.app-bar h2 {
		font-size: 22px;
	}
	.bar-icons {
		display: flex;
		gap: 16px;
	}
	.bar-icons :global(svg) {
		width: 22px;
		height: 22px;
	}
	.list {
		list-style: none;
		margin: 0;
		padding: 0;
		overflow: auto;
	}
	.list button {
		display: flex;
		gap: 12px;
		align-items: center;
		width: 100%;
		padding: 10px 18px;
		border: none;
		background: none;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.list button:hover {
		background: #f4f4f4;
	}
	.list :global(.avatar),
	.me-card :global(.avatar),
	.them :global(.avatar),
	.profile :global(.avatar) {
		border-radius: 38% !important;
	}
	.list :global(.avatar img) {
		border-radius: inherit;
	}
	.main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.row {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 8px;
	}
	.time {
		font-size: 11px;
		color: #8a8a8a;
	}
	.last {
		font-size: 13px;
		color: #777;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.badge {
		min-width: 18px;
		height: 18px;
		padding: 0 5px;
		border-radius: 9px;
		background: #ff3b30;
		color: white;
		font-size: 11px;
		font-weight: 700;
		display: grid;
		place-items: center;
	}
	.section {
		padding: 14px 18px 6px;
		font-size: 12px;
		color: #8a8a8a;
	}
	.me-card {
		display: flex;
		gap: 12px;
		align-items: center;
		padding: 4px 18px 10px;
	}
	.me-card div {
		display: flex;
		flex-direction: column;
	}
	.me-card span {
		font-size: 12px;
		color: #8a8a8a;
	}

	/* ---- 채팅방 ---- */
	.room {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: color-mix(in oklab, var(--accent) 12%, #eef1f4);
	}
	.room-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 14px 10px;
	}
	.room-bar strong {
		flex: 1;
		font-size: 16px;
	}
	.room-bar :global(svg) {
		width: 22px;
		height: 22px;
	}
	.plain {
		display: grid;
		place-items: center;
		border: none;
		background: none;
		color: inherit;
		padding: 0;
		cursor: pointer;
	}
	.msgs {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 6px 12px;
		overflow: auto;
	}
	.date {
		align-self: center;
		padding: 3px 12px;
		border-radius: 12px;
		background: rgb(0 0 0 / 0.12);
		color: white;
		font-size: 11px;
	}
	.them {
		display: flex;
		gap: 8px;
		align-items: flex-start;
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-width: 78%;
	}
	.name {
		font-size: 12px;
		color: #333;
	}
	.line {
		display: flex;
		align-items: flex-end;
		gap: 5px;
	}
	.me {
		align-self: flex-end;
		max-width: 78%;
	}
	.b {
		padding: 7px 10px;
		border-radius: 14px;
		background: white;
		font-size: 14px;
		line-height: 1.4;
	}
	.me .b {
		background: var(--phone-accent);
		color: white;
		border-top-right-radius: 4px;
	}
	.t {
		flex: none;
		font-size: 10px;
		color: #555;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.read {
		color: var(--phone-accent);
		font-weight: 700;
	}
	.input {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 12px 26px;
		background: white;
	}
	.input input {
		flex: 1;
		height: 36px;
		padding: 0 14px;
		border: none;
		border-radius: 18px;
		background: #f1f1f1;
		outline: none;
		color: #191919;
	}
	.input :global(svg) {
		width: 22px;
		height: 22px;
		color: #666;
	}
	.send {
		display: grid;
		place-items: center;
		width: 34px;
		height: 34px;
		border: none;
		border-radius: 50%;
		background: transparent;
		cursor: pointer;
	}
	.send.on {
		background: var(--phone-accent);
	}
	.send.on :global(svg) {
		color: white;
		width: 18px;
		height: 18px;
	}

	/* ---- 설정 앱 ---- */
	.settings {
		flex: 1;
		overflow: auto;
		background: #f2f2f7;
		padding-bottom: 30px;
	}
	.profile {
		display: flex;
		align-items: center;
		gap: 12px;
		width: calc(100% - 32px);
		margin: 8px 16px 0;
		padding: 12px;
		border: none;
		border-radius: 14px;
		background: white;
		text-align: left;
		color: inherit;
		cursor: pointer;
	}
	.profile span {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.profile small {
		color: #8a8a8a;
	}
	.profile :global(svg) {
		color: #c4c4c6;
	}
	.group {
		margin: 0 16px;
		border-radius: 14px;
		background: white;
		overflow: hidden;
	}
	.opt {
		display: flex;
		align-items: center;
		gap: 10px;
		width: 100%;
		padding: 12px 14px;
		border: none;
		background: none;
		color: inherit;
		text-align: left;
		font-size: 15px;
		cursor: pointer;
	}
	.opt + .opt {
		border-top: 1px solid #e5e5ea;
	}
	.check {
		margin-left: auto;
		color: var(--phone-accent);
		font-weight: 700;
	}
	.walls {
		display: flex;
		gap: 10px;
		margin: 0 16px;
	}
	.wall {
		width: 64px;
		height: 110px;
		border-radius: 12px;
		border: 3px solid transparent;
		cursor: pointer;
	}
	.wall.on {
		border-color: var(--phone-accent);
	}

	.home-indicator {
		position: absolute;
		bottom: 8px;
		left: 50%;
		width: 120px;
		height: 5px;
		margin-left: -60px;
		padding: 0;
		border: none;
		border-radius: 3px;
		background: #111;
		opacity: 0.8;
		cursor: pointer;
	}
	.device:has(.home) .home-indicator {
		background: white;
	}
	@media (max-width: 720px) {
		.stage {
			padding: 0;
		}
		.device.framed {
			width: 100%;
			height: 100%;
			border: none;
			border-radius: 0;
		}
	}
</style>
