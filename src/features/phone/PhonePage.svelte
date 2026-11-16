<script lang="ts">
	import { onMount } from "svelte";
	import { Camera, Image as ImageIcon } from "@lucide/svelte";
	import { BatteryFull, ChevronLeft, ChevronRight, Contact, MessageCircle, Pencil, Plus, Search, Settings, Signal, Trash2, Wifi } from "@lucide/svelte";
	import { Avatar, ContextMenu, Spinner, type MenuItem } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { lookup, phone, WALLPAPERS, wallpaperCss, type Who } from "./phone.svelte";
	import PhoneRoom from "./PhoneRoom.svelte";
	import PhonePanel from "./PhonePanel.svelte";
	import PhoneAlbum from "./PhoneAlbum.svelte";
	import ImageCreate from "$features/images/ImageCreate.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import SidePanel from "$app/SidePanel.svelte";
	import { panel } from "$app/panel.svelte";
	import { GitBranch, Gauge, Info, SlidersHorizontal } from "@lucide/svelte";
	import { ChatSession } from "$features/character/session.svelte";

	type Screen = "home" | "messages" | "room" | "contacts" | "settings" | "photos" | "camera";
	/** 이 폰 주인 앨범 키 (카메라 그림) */
	const albumKey = $derived(phone.prefs.owner ? `${phone.prefs.owner.kind}:${phone.prefs.owner.id}` : "");
	let screen = $state<Screen>("home");
	let room = $state<Conversation | null>(null);
	let ready = $state(false);
	/** 연락처 추가·새 채팅 고르기 */
	let picker = $state<null | "contact" | "chat">(null);
	let search = $state("");

	// 오른쪽 패널 (대화방이면 작가 노트·메모도 정보 탭 안에)
	// 대화방이면 대화 화면과 같은 공통 탭 + 정보
	$effect(() =>
		panel.register(
			// 대화·게임처럼 늘 같은 탭 (채팅방을 열기 전에는 안내만)
			[
				{ id: "settings", label: "대화 설정", icon: SlidersHorizontal },
				{ id: "context", label: "컨텍스트·프리셋", icon: Gauge },
				...(room ? [{ id: "branches", label: "분기", icon: GitBranch }] : []),
				{ id: "info", label: "정보", icon: Info },
			],
		),
	);
	// 방의 세션은 여기서 만들어 방과 패널이 같이 씀
	const roomSession = $derived(room ? new ChatSession(room.id) : null);
	/** 분기 등에서 다른 폰 대화 열기 */
	function openRoom(id: string) {
		const c = library.conversation(id);
		if (!c) return;
		room = c;
		screen = "room";
	}

	onMount(async () => {
		await phone.init();
		ready = true;
	});

	// 시계 (30초마다)
	let now = $state(new Date());
	$effect(() => {
		const t = setInterval(() => (now = new Date()), 30_000);
		return () => clearInterval(t);
	});
	const clock = $derived(now.toLocaleTimeString("ko-KR", { hour: "numeric", minute: "2-digit", hour12: false }));
	const dateLabel = $derived(now.toLocaleDateString("ko-KR", { month: "long", day: "numeric", weekday: "long" }));

	const owner = $derived(phone.owner);
	const q = $derived(search.trim().toLowerCase());
	const contacts = $derived(
		phone.contacts
			.map((c) => ({ c, who: lookup({ kind: c.contactKind as Who["kind"], id: c.contactId }) }))
			.filter((x) => x.who && (!q || x.who.name.toLowerCase().includes(q))),
	);
	/** 고를 수 있는 상대: 페르소나 폰은 캐릭터만, 캐릭터 폰은 다른 캐릭터와 내 페르소나 */
	const candidates = $derived.by(() => {
		const o = phone.prefs.owner;
		const chars = library.characters.filter((c) => !(o?.kind === "character" && o.id === c.id)).map((c) => lookup({ kind: "character", id: c.id })!);
		const personas = o?.kind === "character" ? library.personas.map((p) => lookup({ kind: "persona", id: p.id })!) : [];
		return [...chars, ...personas].filter(Boolean);
	});
	const pickList = $derived(picker === "contact" ? candidates.filter((w) => !phone.hasContact(w)) : candidates);
	// 메시지가 하나도 없는 방은 목록에 안 보임 (연락처에서 열었다가 그냥 나온 경우)
	const chats = $derived(phone.chats.filter((c) => phone.previews[c.id]).filter((c) => !q || lookup(phone.contactOf(c))?.name.toLowerCase().includes(q)));

	function go(s: Screen) {
		search = "";
		picker = null;
		screen = s;
	}
	async function openChat(w: Who) {
		picker = null;
		const c = await phone.open(w);
		if (!c) return;
		room = c;
		screen = "room";
	}
	function closeRoom() {
		room = null;
		screen = "messages";
	}
	function openProfile() {
		const o = phone.prefs.owner;
		if (!o) return;
		nav.go(o.kind === "persona" ? { page: "persona", personaId: o.id } : { page: "character", charId: o.id });
	}

	function when(t: number) {
		const d = new Date(t);
		const today = new Date();
		if (d.toDateString() === today.toDateString()) return d.toLocaleTimeString("ko-KR", { hour: "numeric", minute: "2-digit" });
		const y = new Date(today);
		y.setDate(today.getDate() - 1);
		if (d.toDateString() === y.toDateString()) return "어제";
		return d.toLocaleDateString("ko-KR", { month: "numeric", day: "numeric" });
	}

	let menu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function contactMenu(e: MouseEvent, w: Who, status: string) {
		e.preventDefault();
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{
					label: "상태 메시지 바꾸기",
					icon: Pencil,
					onselect: async () => {
						const t = await promptText({ title: "상태 메시지", input: { value: status, placeholder: "예: 기사단 훈련 중" }, confirmLabel: "저장" });
						if (t !== null && t !== undefined) await phone.setStatus(w, t);
					},
				},
				{ label: "연락처에서 지우기", icon: Trash2, danger: true, separator: true, onselect: () => phone.removeContact(w) },
			],
		};
	}
	const kindLabel = (k: string) => (k === "persona" ? "내 페르소나" : "캐릭터");
</script>

<div class="phone-page">
<div class="stage" class:full={phone.prefs.display === "full"}>
	<div class="device" class:framed={phone.prefs.display === "mockup"} style:--wall={wallpaperCss(phone.wallpaper)}>
		<div class="status" class:on-wall={screen === "home"}>
			<span>{clock}</span><span class="icons"><Signal /><Wifi /><BatteryFull /></span>
		</div>

		{#if !ready}
			<div class="center"><Spinner /></div>
		{:else if !owner}
			<div class="center empty">
				<p>폰 주인이 될 페르소나가 없음</p>
				<button class="link" onclick={() => nav.go({ page: "persona" })}>페르소나 만들러 가기</button>
			</div>
		{:else if screen === "home"}
			<div class="home">
				<div class="clock"><b>{clock}</b><span>{dateLabel}</span>{#if phone.isCharacterPhone}<em class="whose">{owner.name}의 폰</em>{/if}</div>
				<div class="apps">
					<button class="app" onclick={() => go("messages")}>
						<span class="ic msg"><MessageCircle />{#if phone.unread}<i>{phone.unread}</i>{/if}</span>메시지
					</button>
					<button class="app" onclick={() => go("contacts")}><span class="ic contact"><Contact /></span>연락처</button>
					<button class="app" onclick={() => go("photos")}><span class="ic photos"><ImageIcon /></span>사진</button>
					{#if plugins.on("imagegen")}<button class="app" onclick={() => go("camera")}><span class="ic camera"><Camera /></span>카메라</button>{/if}
					<button class="app" onclick={() => go("settings")}><span class="ic set"><Settings /></span>설정</button>
				</div>
				<div class="dock">
					<button class="app" aria-label="메시지" onclick={() => go("messages")}><span class="ic msg"><MessageCircle /></span></button>
					<button class="app" aria-label="연락처" onclick={() => go("contacts")}><span class="ic contact"><Contact /></span></button>
					<button class="app" aria-label="설정" onclick={() => go("settings")}><span class="ic set"><Settings /></span></button>
				</div>
			</div>
		{:else if screen === "photos"}
			<PhoneAlbum album={albumKey} chatIds={phone.chats.map((c) => c.id)} onback={() => go("home")} oncamera={plugins.on("imagegen") ? () => go("camera") : undefined} />
		{:else if screen === "camera"}
			<div class="app-bar">
				<button class="plain" aria-label="뒤로" onclick={() => go("home")}><ChevronLeft /></button>
				<h2 class="grow">카메라</h2>
				<button class="plain small" onclick={() => go("photos")}>사진</button>
			</div>
			<div class="cam-wrap"><ImageCreate source="phone" album={albumKey} compact /></div>
		{:else if screen === "room" && room}
			{#key room.id}
				<PhoneRoom conv={room} onback={closeRoom} session={roomSession ?? undefined} />
			{/key}
		{:else if picker}
			<div class="app-bar">
				<button class="plain" aria-label="뒤로" onclick={() => (picker = null)}><ChevronLeft /></button>
				<h2 class="grow">{picker === "contact" ? "연락처 추가" : "새 채팅"}</h2>
			</div>
			<ul class="list">
				{#each pickList as w (`${w.kind}:${w.id}`)}
					<li>
						<button onclick={() => (picker === "contact" ? phone.addContact(w).then(() => (picker = null)) : openChat(w))}>
							<Avatar name={w.name} src={fileSrc(w.avatar)} color={w.color} size={40} />
							<span class="main"><strong>{w.name}</strong>{#if phone.isCharacterPhone}<span class="last">{kindLabel(w.kind)}</span>{/if}</span>
						</button>
					</li>
				{:else}
					<li class="none">{picker === "contact" ? "더 넣을 사람 없음" : "캐릭터 없음"}</li>
				{/each}
			</ul>
		{:else if screen === "messages"}
			<div class="app-bar">
				<h2>채팅</h2>
				<span class="bar-icons"><button class="plain" aria-label="새 채팅" onclick={() => (picker = "chat")}><Plus /></button></span>
			</div>
			{#if phone.chats.length > 4}
				<label class="search"><Search /><input bind:value={search} placeholder="검색" aria-label="채팅 검색" /></label>
			{/if}
			<ul class="list">
				{#each chats as c (c.id)}
					{@const w = lookup(phone.contactOf(c))}
					{@const p = phone.previews[c.id]}
					<li>
						<button onclick={() => w && openChat(w)}>
							<Avatar name={w?.name ?? "?"} src={fileSrc(w?.avatar)} color={w?.color} size={46} />
							<span class="main">
								<span class="row"><strong>{w?.name ?? c.title}</strong><span class="time">{when(p?.at ?? c.updatedAt)}</span></span>
								<span class="row"><span class="last">{p?.text || "대화를 시작해 봄"}</span>{#if p?.unread}<span class="badge">{p.unread}</span>{/if}</span>
							</span>
						</button>
					</li>
				{:else}
					<li class="none">
						아직 채팅 없음<br />
						<button class="link" onclick={() => (picker = "chat")}>새 채팅 시작</button>
					</li>
				{/each}
			</ul>
		{:else if screen === "contacts"}
			<div class="app-bar">
				<h2>연락처</h2>
				<span class="bar-icons"><button class="plain" aria-label="연락처 추가" onclick={() => (picker = "contact")}><Plus /></button></span>
			</div>
			<div class="me-card">
				<Avatar name={owner.name} src={fileSrc(owner.avatar)} color={owner.color} size={52} />
				<div><strong>{owner.name}</strong><span>내 프로필{phone.isCharacterPhone ? " (캐릭터)" : ""}</span></div>
			</div>
			{#if phone.contacts.length > 6}
				<label class="search"><Search /><input bind:value={search} placeholder="이름 검색" aria-label="연락처 검색" /></label>
			{/if}
			<p class="section">친구 {phone.contacts.length}</p>
			<ul class="list">
				{#each contacts as { c, who } (`${c.contactKind}:${c.contactId}`)}
					<li>
						<button onclick={() => openChat(who!)} oncontextmenu={(e) => contactMenu(e, who!, c.status)}>
							<Avatar name={who!.name} src={fileSrc(who!.avatar)} color={who!.color} size={40} />
							<span class="main"><strong>{who!.name}</strong>{#if c.status}<span class="last">{c.status}</span>{:else if phone.isCharacterPhone && c.contactKind === "persona"}<span class="last">내 페르소나</span>{/if}</span>
						</button>
					</li>
				{:else}
					<li class="none">
						연락처 없음<br />
						<button class="link" onclick={() => (picker = "contact")}>연락처 추가</button>
					</li>
				{/each}
			</ul>
		{:else if screen === "settings"}
			<div class="app-bar"><h2>설정</h2></div>
			<div class="settings">
				<button class="profile" onclick={openProfile}>
					<Avatar name={owner.name} src={fileSrc(owner.avatar)} color={owner.color} size={52} />
					<span><strong>{owner.name}</strong><small>이 폰의 주인 ({phone.isCharacterPhone ? "캐릭터" : "페르소나"})</small></span>
					<ChevronRight />
				</button>
				<p class="section">내 페르소나의 폰</p>
				<div class="group">
					{#each library.personas as p (p.id)}
						{@const on = phone.prefs.owner?.kind === "persona" && phone.prefs.owner.id === p.id}
						<button class="opt" onclick={() => phone.switchOwner({ kind: "persona", id: p.id })}>
							<Avatar name={p.name} src={fileSrc(p.avatar)} color={p.color || null} size={28} />{p.name}
							{#if on}<span class="check">✓</span>{/if}
						</button>
					{/each}
				</div>
				<p class="section">캐릭터의 폰 (내가 그 캐릭터가 되거나, 대화를 엿봄)</p>
				<div class="group">
					{#each library.characters as c (c.id)}
						{@const on = phone.prefs.owner?.kind === "character" && phone.prefs.owner.id === c.id}
						<button class="opt" onclick={() => phone.switchOwner({ kind: "character", id: c.id })}>
							<Avatar name={c.name} src={fileSrc(c.avatar)} color={c.color} size={28} />{c.name}
							{#if on}<span class="check">✓</span>{/if}
						</button>
					{/each}
				</div>
				<p class="section">배경화면</p>
				<div class="walls">
					{#each Object.entries(WALLPAPERS) as [id, bg] (id)}
						<button class="wall" class:on={phone.wallpaper === id} style:background={bg} aria-label="배경 {id}" onclick={() => phone.setWallpaper(id)}></button>
					{/each}
				</div>
				<p class="section">화면 표시</p>
				<div class="group">
					<button class="opt" onclick={() => phone.setDisplay("mockup")}>기기 모양으로 보기{#if phone.prefs.display === "mockup"}<span class="check">✓</span>{/if}</button>
					<button class="opt" onclick={() => phone.setDisplay("full")}>화면 가득 보기{#if phone.prefs.display === "full"}<span class="check">✓</span>{/if}</button>
				</div>
			</div>
		{/if}

		<button class="home-indicator" aria-label="홈으로" onclick={() => go("home")}></button>
	</div>
</div>
{#if panel.open && ready}
	<SidePanel label="폰 패널"><PhonePanel {room} session={roomSession} onopen={openRoom} /></SidePanel>
{/if}
</div>

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label="연락처 메뉴" onclose={() => (menu = null)} />
{/if}

<style>
	.phone-page {
		display: flex;
		height: 100%;
		min-height: 0;
	}
	.phone-page > .stage {
		flex: 1;
		min-width: 0;
	}
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
		z-index: var(--z-above);
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
	.center {
		flex: 1;
		display: grid;
		place-items: center;
		align-content: center;
		gap: 8px;
		color: #6b6b6b;
		font-size: 14px;
	}
	.link {
		border: none;
		background: none;
		color: var(--accent);
		font: inherit;
		font-weight: 600;
		cursor: pointer;
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
	.whose {
		margin-top: 6px;
		padding: 2px 10px;
		border-radius: 999px;
		background: rgb(0 0 0 / 0.25);
		font-size: 12px;
		font-style: normal;
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
	.set {
		background: #636366;
	}
	.photos {
		background: linear-gradient(135deg, #ffb35c, #ff6b8b 55%, #8f6fe0);
	}
	.camera {
		background: #2c2c2e;
	}
	.cam-wrap {
		background: none;
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding: 0 14px 16px;
	}
	.small {
		font-size: 14px;
		color: var(--accent);
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
		gap: 8px;
		padding: 8px 18px 12px;
	}
	.app-bar h2 {
		font-size: 22px;
	}
	.grow {
		flex: 1;
	}
	.bar-icons {
		display: flex;
		gap: 16px;
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
	.app-bar :global(svg) {
		width: 22px;
		height: 22px;
	}
	.search {
		display: flex;
		align-items: center;
		gap: 6px;
		margin: 0 18px 8px;
		padding: 0 10px;
		height: 34px;
		border-radius: 10px;
		background: #f1f1f1;
		color: #8a8a8a;
	}
	.search :global(svg) {
		width: 16px;
		height: 16px;
	}
	.search input {
		flex: 1;
		border: none;
		outline: none;
		background: none;
		color: #191919;
		font: inherit;
		font-size: 14px;
	}
	.list {
		flex: 1;
		list-style: none;
		margin: 0;
		padding: 0 0 30px;
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
	.none {
		padding: 40px 18px;
		color: #8a8a8a;
		font-size: 14px;
		text-align: center;
		line-height: 2;
	}
	.list :global(.avatar),
	.me-card :global(.avatar),
	.profile :global(.avatar) {
		border-radius: 38% !important;
		overflow: hidden;
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
		flex: none;
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
		flex: none;
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

	/* ---- 설정 앱 ---- */
	.settings {
		flex: 1;
		overflow: auto;
		background: #f2f2f7;
		padding-bottom: 30px;
	}
	.settings .section {
		padding-left: 30px;
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
		font: inherit;
		font-size: 15px;
		cursor: pointer;
	}
	.opt + .opt {
		border-top: 1px solid #e5e5ea;
	}
	.opt :global(.avatar) {
		border-radius: 38% !important;
	}
	.check {
		margin-left: auto;
		color: var(--accent);
		font-weight: 700;
	}
	.walls {
		display: flex;
		flex-wrap: wrap;
		gap: 10px;
		margin: 0 16px;
	}
	.wall {
		width: 56px;
		height: 96px;
		border-radius: 12px;
		border: 3px solid transparent;
		cursor: pointer;
	}
	.wall.on {
		border-color: var(--accent);
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
