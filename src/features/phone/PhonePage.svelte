<script lang="ts">
	import { _ } from "svelte-i18n";
	import { getAppLocale } from "$lib/utils/i18n";
	import { lastAt } from "$features/character/library.svelte";
	import { onMount } from "svelte";
	import { Camera, Image as ImageIcon } from "@lucide/svelte";
	import { BatteryFull, ChevronLeft, ChevronRight, Contact, MessageCircle, Pencil, Plus, Search, Settings, Signal, Trash2, Users, Wifi } from "@lucide/svelte";
	import { entryTitle, lorebook } from "$features/lorebook/lorebook.svelte";
	import { Avatar, ContextMenu, Spinner, type MenuItem } from "$shared/ui";
	import { fileSrc } from "$shared/ipc/file";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";
	import { convActions } from "$features/character/actions";
	import { Download, Variable } from "@lucide/svelte";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { nav, type PhoneApp } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { lookup, phone, WALLPAPERS, wallpaperCss, type Who } from "./phone.svelte";
	import PhoneRoom from "./PhoneRoom.svelte";
	import PhoneProfile, { type ProfileTarget } from "./PhoneProfile.svelte";
	import PhonePanel from "./PhonePanel.svelte";
	import PhoneAlbum from "./PhoneAlbum.svelte";
	import PhoneCamera from "./PhoneCamera.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import SidePanel from "$app/SidePanel.svelte";
	import { panel } from "$app/panel.svelte";
	import { Bookmark, Database, GitBranch, Gauge, Images, Info, ListTree, SlidersHorizontal, SpellCheck, Zap } from "@lucide/svelte";
	import { ChatSession } from "$features/character/session.svelte";
	import type { ChatBookmark } from "$features/character/bookmarks";
	import { current as currentPluginSetting, ensureLoaded as ensurePluginSetting } from "$features/settings/pluginSettingForm.svelte";

	type Screen = PhoneApp | "room";
	let { app, convId }: { app?: PhoneApp; convId?: string } = $props();
	/** 이 폰 주인 앨범 키 (카메라 그림) */
	const albumKey = $derived(phone.prefs.owner ? `${phone.prefs.owner.kind}:${phone.prefs.owner.id}` : "");
	const room = $derived<Conversation | null>(convId ? (library.conversation(convId) ?? null) : null);
	const screen = $derived<Screen>(convId && room ? "room" : (app ?? "home"));
	let ready = $state(false);
	/** 연락처 추가·새 채팅 고르기 */
	let picker = $state<null | "contact" | "chat">(null);
	let search = $state("");
	/** 메신저(대화) 앱 안의 카카오톡식 탭: 열면 친구 목록이 먼저 표시됨 */
	let messengerTab = $state<"friends" | "chats">("friends");
	/** 친구 목록에서 캐릭터 클릭 시 열리는 상세 페이지(카카오톡 스타일 프로필) */
	let viewingProfile = $state<{ target: ProfileTarget; chatWho: Who } | null>(null);

	$effect(() => {
		void app;
		void convId;
		search = "";
		picker = null;
		viewingProfile = null;
		if (app === "messages" && !convId) {
			messengerTab = "friends";
		}
	});
	$effect(() => {
		if (ready && (screen === "messages" || screen === "home")) {
			void phone.loadPreviews();
		}
	});
	// 다인봇 태그가 붙은 캐릭터는 로어북에서 등장인물(항목)을 불러와 친구 목록에 표시
	const multiBotChars = $derived(
		library.characters.filter(
			(c) => c.tags.some((t) => t.trim().includes("다인봇") || /^(multi[- ]?char|group)$/i.test(t.trim())) && c.worldInfoIds.length > 0, // i18n-ignore: 사용자 캐릭터 카드 태그 매칭
		),
	);
	$effect(() => {
		if (!ready || (screen !== "messages" && screen !== "contacts")) return;
		for (const c of multiBotChars) {
			for (const bid of c.worldInfoIds) {
				if (!lorebook.entries[bid]) void lorebook.loadEntries(bid);
			}
		}
	});
	const multiBotGroups = $derived.by(() => {
		return multiBotChars
			.map((ch) => {
				const raw = ch.worldInfoIds.flatMap((bid) => lorebook.entries[bid] ?? []);
				const entries = raw
					.filter((e) => e.enabled && entryTitle(e).trim())
					.map((e) => {
						const name = entryTitle(e).trim();
						const status = e.content.split("\n").find((l) => l.trim())?.trim().slice(0, 48) ?? e.category ?? "";
						const matchedChar = library.characters.find((x) => x.name === name);
						return { id: e.id, name, status, avatar: matchedChar?.avatar ?? null, avatarFit: matchedChar?.avatarFit ?? null, color: matchedChar?.color ?? null };
					})
					.filter((item) => !q || item.name.toLowerCase().includes(q) || ch.name.toLowerCase().includes(q));
				return { ch, entries };
			})
			.filter((g) => g.entries.length > 0);
	});

	// 오른쪽 패널: 채팅방을 열면 ChatView와 동일한 탭 구성 + 폰 정보
	$effect(() =>
		panel.register([
			{ id: "settings", label: $_("chatView.tabSettings"), icon: SlidersHorizontal },
			{ id: "context", label: $_("chatView.tabContext"), icon: Gauge },
			...(room
				? [
						{ id: "toc", label: $_("chatView.tabToc"), icon: ListTree },
						{ id: "bookmarks", label: $_("chatView.tabBookmarks"), icon: Bookmark },
						...(plugins.on("summary") || plugins.on("memory") ? [{ id: "memory", label: $_("chatView.tabMemory"), icon: Database }] : []),
						...(plugins.on("lint") ? [{ id: "lint", label: $_("chatView.tabLint"), icon: SpellCheck }] : []),
						{ id: "branches", label: $_("chatView.tabBranches"), icon: GitBranch },
						{ id: "assets", label: $_("chatView.tabAssets"), icon: Images },
						...(plugins.on("slash") ? [{ id: "quickreply", label: $_("chatView.tabQuickReply"), icon: Zap }] : []),
						...(plugins.on("status") ? [{ id: "status", label: $_("chatView.tabStatus"), icon: Variable }] : []),
					]
				: []),
			{ id: "info", label: $_("phoneUi.panelInfo"), icon: Info },
		]),
	);
	// 방의 세션은 room.id가 바뀔 때만 $effect에서 만든다 ($derived 안에서 $state 클래스 생성 시 state_unsafe_mutation 발생 방지)
	let roomSession = $state<ChatSession | null>(null);
	$effect(() => {
		const id = room?.id;
		roomSession = id ? new ChatSession(id) : null;
	});
	let roomRef = $state<ReturnType<typeof PhoneRoom> | undefined>(undefined);
	const roomDraft = {
		get: () => roomRef?.getDraft() ?? "",
		set: (v: string) => roomRef?.setDraft(v),
	};
	function jumpInRoom(messageId: string, mark?: ChatBookmark) {
		void roomRef?.jumpTo(messageId, mark);
	}
	function cardFromPanel(mark: ChatBookmark) {
		roomRef?.openCard(mark);
	}
	/** 분기 등에서 다른 폰 대화 열기 */
	function openRoom(id: string) {
		const c = library.conversation(id);
		if (!c) return;
		nav.go({ page: "phone", app: "messages", convId: c.id });
	}

	onMount(async () => {
		ensurePluginSetting();
		await phone.init();
		ready = true;
	});

	// 시계 (30초마다) — 폰 설정에서 직접 지정한 날짜·시간이 켜져 있으면 그 값을 표시
	let now = $state(new Date());
	$effect(() => {
		const t = setInterval(() => (now = new Date()), 30_000);
		return () => clearInterval(t);
	});
	const phoneCfg = $derived(currentPluginSetting()?.phone);
	const displayDate = $derived.by(() => {
		if (phoneCfg?.customDatetimeEnabled && phoneCfg.customDate) {
			const [y, m, d] = phoneCfg.customDate.split("-").map(Number);
			if (y && m && d) return new Date(y, m - 1, d);
		}
		return now;
	});
	const dateLoc = $derived(getAppLocale() === "en" ? "en-US" : "ko-KR");
	const clock = $derived(
		phoneCfg?.customDatetimeEnabled && phoneCfg.customTime
			? phoneCfg.customTime
			: now.toLocaleTimeString(dateLoc, { hour: "numeric", minute: "2-digit", hour12: false }),
	);
	const dateLabel = $derived(displayDate.toLocaleDateString(dateLoc, { month: "long", day: "numeric", weekday: "long" }));

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
	// 메시지가 있는 방 표시 (previews가 아직 로딩 중이어도 messageCount > 0이면 표시)
	const chats = $derived(
		phone.chats
			.filter((c) => !!phone.previews[c.id] || c.messageCount > 0)
			.filter((c) => !q || lookup(phone.contactOf(c))?.name.toLowerCase().includes(q)),
	);

	function go(s: PhoneApp) {
		search = "";
		picker = null;
		nav.go(s === "home" ? { page: "phone" } : { page: "phone", app: s });
	}
	async function openChat(w: Who) {
		picker = null;
		const c = await phone.open(w);
		if (!c) return;
		nav.go({ page: "phone", app: "messages", convId: c.id });
	}
	function closeRoom() {
		nav.go({ page: "phone", app: "messages" });
	}
	function openProfile() {
		const o = phone.prefs.owner;
		if (!o) return;
		nav.go(o.kind === "persona" ? { page: "persona", personaId: o.id } : { page: "character", charId: o.id });
	}

	function when(t: number) {
		const d = new Date(t);
		const today = new Date();
		if (d.toDateString() === today.toDateString()) return d.toLocaleTimeString(dateLoc, { hour: "numeric", minute: "2-digit" });
		const y = new Date(today);
		y.setDate(today.getDate() - 1);
		if (d.toDateString() === y.toDateString()) return $_("phoneUi.yesterday");
		return d.toLocaleDateString(dateLoc, { month: "numeric", day: "numeric" });
	}

	let menu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function contactMenu(e: MouseEvent, w: Who, status: string) {
		e.preventDefault();
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{
					label: $_("phoneUi.editStatusMenu"),
					icon: Pencil,
					onselect: async () => {
						const t = await promptText({ title: $_("phoneUi.statusTitle"), input: { value: status, placeholder: $_("phoneUi.statusPlaceholder") }, confirmLabel: $_("common.save") });
						if (t !== null && t !== undefined) await phone.setStatus(w, t);
					},
				},
				{ label: $_("phoneUi.removeContactMenu"), icon: Trash2, danger: true, separator: true, onselect: () => phone.removeContact(w) },
			],
		};
	}
	/** 채팅방 메뉴 (오른쪽 클릭·꾹 누르기): 일반 대화로 복사, SillyTavern 채팅으로 내보내기 (5-E-20-3) */
	function roomMenu(e: MouseEvent, c: Conversation) {
		e.preventDefault();
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{
					label: $_("phoneUi.copyToChatMenu"),
					icon: MessageCircle,
					onselect: async () => {
						const copy = await call(commands.copyConversationAsChat(c.id), { source: "phone" });
						await library.load();
						toasts.show({ tone: "success", message: $_("phoneUi.copiedChatToast", { values: { title: copy.title } }), action: { label: $_("phoneUi.openBtn"), run: () => nav.go({ page: "character", charId: copy.characterId, convId: copy.id }) } });
					},
				},
				{ label: $_("phoneUi.exportStMenu"), icon: Download, onselect: () => void convActions.exportAs(c, "jsonl") },
			],
		};
	}
	const kindLabel = (k: string) => (k === "persona" ? $_("phoneUi.kindPersona") : $_("phoneUi.kindChar"));
</script>

<div class="phone-page sc-phone">
<div class="stage" class:full={phone.prefs.display === "full"}>
	<div class="device" class:framed={phone.prefs.display === "mockup"} style:--wall={wallpaperCss(phone.wallpaper)}>
		<div class="status" class:on-wall={screen === "home"}>
			<span>{clock}</span><span class="icons"><Signal /><Wifi /><BatteryFull /></span>
		</div>

		{#if !ready}
			<div class="center"><Spinner /></div>
		{:else if !owner}
			<div class="center empty">
				<p>{$_("phoneUi.noOwnerMsg")}</p>
				<button class="link" onclick={() => nav.go({ page: "persona" })}>{$_("phoneUi.goCreatePersona")}</button>
			</div>
		{:else if screen === "home"}
			<div class="home">
				<div class="clock"><b>{clock}</b><span>{dateLabel}</span>{#if phone.isCharacterPhone}<em class="whose">{$_("phoneUi.ownerPhoneLabel", { values: { name: owner.name } })}</em>{/if}</div>
				<div class="apps">
					<button class="app" onclick={() => go("messages")}>
						<span class="ic msg"><MessageCircle />{#if phone.unread}<i>{phone.unread}</i>{/if}</span>{$_("phoneUi.appMessages")}
					</button>
					<button class="app" onclick={() => go("contacts")}><span class="ic contact"><Contact /></span>{$_("phoneUi.appContacts")}</button>
					<button class="app" onclick={() => go("photos")}><span class="ic photos"><ImageIcon /></span>{$_("phoneUi.appPhotos")}</button>
					{#if plugins.on("imagegen")}<button class="app" onclick={() => go("camera")}><span class="ic camera"><Camera /></span>{$_("phoneUi.appCamera")}</button>{/if}
					<button class="app" onclick={() => go("settings")}><span class="ic set"><Settings /></span>{$_("phoneUi.appSettings")}</button>
				</div>
				<div class="dock">
					<button class="app" aria-label={$_("phoneUi.appMessages")} onclick={() => go("messages")}><span class="ic msg"><MessageCircle /></span></button>
					<button class="app" aria-label={$_("phoneUi.appContacts")} onclick={() => go("contacts")}><span class="ic contact"><Contact /></span></button>
					<button class="app" aria-label={$_("phoneUi.appSettings")} onclick={() => go("settings")}><span class="ic set"><Settings /></span></button>
				</div>
			</div>
		{:else if screen === "photos"}
			<PhoneAlbum album={albumKey} chatIds={phone.chats.map((c) => c.id)} onback={() => go("home")} oncamera={plugins.on("imagegen") ? () => go("camera") : undefined} />
		{:else if screen === "camera"}
			<PhoneCamera album={albumKey} onback={() => go("home")} onalbum={() => go("photos")} />
		{:else if screen === "room" && room}
			{#key room.id}
				<PhoneRoom bind:this={roomRef} conv={room} onback={closeRoom} session={roomSession ?? undefined} />
			{/key}
		{:else if viewingProfile}
			{@const vp = viewingProfile}
			<PhoneProfile
				target={vp.target}
				onback={() => (viewingProfile = null)}
				onchat={() => {
					const who = viewingProfile?.chatWho;
					viewingProfile = null;
					if (who) void openChat(who);
				}}
			/>
		{:else if picker}
			<div class="app-bar">
				<button class="plain" aria-label={$_("phoneUi.backAria")} onclick={() => (picker = null)}><ChevronLeft /></button>
				<h2 class="grow">{picker === "contact" ? $_("phoneUi.addContact") : $_("phoneUi.newChat")}</h2>
			</div>
			<ul class="list">
				{#each pickList as w (`${w.kind}:${w.id}`)}
					<li>
						<button onclick={() => (picker === "contact" ? phone.addContact(w).then(() => (picker = null)) : openChat(w))}>
							<Avatar name={w.name} src={fileSrc(w.avatar)} fit={w.avatarFit} color={w.color} size={40} />
							<span class="main"><strong>{w.name}</strong>{#if phone.isCharacterPhone}<span class="last">{kindLabel(w.kind)}</span>{/if}</span>
						</button>
					</li>
				{:else}
					<li class="none">{picker === "contact" ? $_("phoneUi.noMoreContacts") : $_("phoneUi.noChars")}</li>
				{/each}
			</ul>
		{:else if screen === "messages"}
			{#if messengerTab === "friends"}
				<div class="app-bar">
					<h2>{$_("phoneUi.friendsTitle")}</h2>
					<span class="bar-icons"><button class="plain" aria-label={$_("phoneUi.addContact")} onclick={() => (picker = "contact")}><Plus /></button></span>
				</div>
				<div class="me-card">
					<Avatar name={owner.name} src={fileSrc(owner.avatar)} fit={owner.avatarFit} color={owner.color} size={52} />
					<div><strong>{owner.name}</strong><span>{$_("phoneUi.myProfile")}{phone.isCharacterPhone ? $_("phoneUi.charSuffix") : ""}</span></div>
				</div>
				{#if phone.contacts.length > 4 || multiBotGroups.length > 0}
					<label class="search"><Search /><input bind:value={search} placeholder={$_("phoneUi.searchFriendPlaceholder")} aria-label={$_("phoneUi.searchFriendAria")} /></label>
				{/if}
				<div class="kakao-scroll">
					<p class="section">{$_("phoneUi.friendsCount", { values: { count: contacts.length } })}</p>
					<ul class="list inline-list">
						{#each contacts as { c, who } (`${c.contactKind}:${c.contactId}`)}
							<li>
								<button
									onclick={() =>
										(viewingProfile = {
											target: {
												key: `${who!.kind}:${who!.id}`,
												name: who!.name,
												avatar: who!.avatar,
												avatarFit: who!.avatarFit,
												color: who!.color,
												who: who!,
											},
											chatWho: who!,
										})}
									oncontextmenu={(e) => contactMenu(e, who!, c.status)}
								>
									<Avatar name={who!.name} src={fileSrc(who!.avatar)} fit={who!.avatarFit} color={who!.color} size={42} />
									<span class="main"><strong>{who!.name}</strong>{#if c.status}<span class="last">{c.status}</span>{:else if phone.isCharacterPhone && c.contactKind === "persona"}<span class="last">{$_("phoneUi.kindPersona")}</span>{/if}</span>
								</button>
							</li>
						{:else}
							{#if !multiBotGroups.length}
								<li class="none">
									{$_("phoneUi.noFriends")}<br />
									<button class="link" onclick={() => (picker = "contact")}>{$_("phoneUi.addFriendBtn")}</button>
								</li>
							{/if}
						{/each}
					</ul>
					{#each multiBotGroups as g (g.ch.id)}
						<p class="section">{$_("phoneUi.multiBotSection", { values: { name: g.ch.name, count: g.entries.length } })}</p>
						<ul class="list inline-list">
							{#each g.entries as entry (entry.id)}
								<li>
									<button
										onclick={() =>
											(viewingProfile = {
												target: {
													key: `multibot:${g.ch.id}:${entry.id}`,
													name: entry.name,
													avatar: entry.avatar,
													avatarFit: entry.avatarFit,
													color: entry.color,
													chatWho: { kind: "character", id: g.ch.id },
												},
												chatWho: { kind: "character", id: g.ch.id },
											})}
									>
										<Avatar name={entry.name} src={fileSrc(entry.avatar)} fit={entry.avatarFit} color={entry.color} size={42} />
										<span class="main"><strong>{entry.name}</strong>{#if entry.status}<span class="last">{entry.status}</span>{/if}</span>
									</button>
								</li>
							{/each}
						</ul>
					{/each}
				</div>
			{:else}
				<div class="app-bar">
					<h2>{$_("phoneUi.chatsTitle")}</h2>
					<span class="bar-icons"><button class="plain" aria-label={$_("phoneUi.newChat")} onclick={() => (picker = "chat")}><Plus /></button></span>
				</div>
				{#if chats.length > 4 || q}
					<label class="search"><Search /><input bind:value={search} placeholder={$_("phoneUi.searchChatPlaceholder")} aria-label={$_("phoneUi.searchChatPlaceholder")} /></label>
				{/if}
				<ul class="list">
					{#each chats as c (c.id)}
						{@const w = lookup(phone.contactOf(c))}
						{@const p = phone.previews[c.id]}
						<li>
							<button onclick={() => w && openChat(w)} oncontextmenu={(e) => roomMenu(e, c)}>
								<Avatar name={w?.name ?? "?"} src={fileSrc(w?.avatar)} fit={w?.avatarFit} color={w?.color} size={46} />
								<span class="main">
									<span class="row"><strong>{w?.name ?? c.title}</strong><span class="time">{when(p?.at ?? lastAt(c))}</span></span>
									<span class="row"><span class="last">{p?.text || $_("phoneUi.startChatHint")}</span>{#if p?.unread}<span class="badge">{p.unread}</span>{/if}</span>
								</span>
							</button>
						</li>
					{:else}
						<li class="none">
							{$_("phoneUi.noChats")}<br />
							<button class="link" onclick={() => (picker = "chat")}>{$_("phoneUi.startNewChatBtn")}</button>
						</li>
					{/each}
				</ul>
			{/if}
			<nav class="kakao-tabs" aria-label={$_("phoneUi.messengerTabsAria")}>
				<button type="button" class:on={messengerTab === "friends"} onclick={() => ((messengerTab = "friends"), (search = ""))}>
					<Users /><span>{$_("phoneUi.friendsTitle")}</span>
				</button>
				<button type="button" class:on={messengerTab === "chats"} onclick={() => ((messengerTab = "chats"), (search = ""), void phone.loadPreviews())}>
					<span class="tab-ic"><MessageCircle />{#if phone.unread}<i>{phone.unread}</i>{/if}</span><span>{$_("phoneUi.chatsTitle")}</span>
				</button>
			</nav>
		{:else if screen === "contacts"}
			<div class="app-bar">
				<h2>{$_("phoneUi.appContacts")}</h2>
				<span class="bar-icons"><button class="plain" aria-label={$_("phoneUi.addContact")} onclick={() => (picker = "contact")}><Plus /></button></span>
			</div>
			<div class="me-card">
				<Avatar name={owner.name} src={fileSrc(owner.avatar)} fit={owner.avatarFit} color={owner.color} size={52} />
				<div><strong>{owner.name}</strong><span>{$_("phoneUi.myProfile")}{phone.isCharacterPhone ? $_("phoneUi.charSuffix") : ""}</span></div>
			</div>
			{#if phone.contacts.length > 6}
				<label class="search"><Search /><input bind:value={search} placeholder={$_("phoneUi.searchNamePlaceholder")} aria-label={$_("phoneUi.searchContactsAria")} /></label>
			{/if}
			<p class="section">{$_("phoneUi.friendsCount", { values: { count: phone.contacts.length } })}</p>
			<ul class="list">
				{#each contacts as { c, who } (`${c.contactKind}:${c.contactId}`)}
					<li>
						<button
							onclick={() =>
								(viewingProfile = {
									target: {
										key: `${who!.kind}:${who!.id}`,
										name: who!.name,
										avatar: who!.avatar,
										avatarFit: who!.avatarFit,
										color: who!.color,
										who: who!,
									},
									chatWho: who!,
								})}
							oncontextmenu={(e) => contactMenu(e, who!, c.status)}
						>
							<Avatar name={who!.name} src={fileSrc(who!.avatar)} fit={who!.avatarFit} color={who!.color} size={40} />
							<span class="main"><strong>{who!.name}</strong>{#if c.status}<span class="last">{c.status}</span>{:else if phone.isCharacterPhone && c.contactKind === "persona"}<span class="last">{$_("phoneUi.kindPersona")}</span>{/if}</span>
						</button>
					</li>
				{:else}
					<li class="none">
						{$_("phoneUi.noContacts")}<br />
						<button class="link" onclick={() => (picker = "contact")}>{$_("phoneUi.addContact")}</button>
					</li>
				{/each}
			</ul>
		{:else if screen === "settings"}
			<div class="app-bar"><h2>{$_("phoneUi.appSettings")}</h2></div>
			<div class="settings">
				<button class="profile" onclick={openProfile}>
					<Avatar name={owner.name} src={fileSrc(owner.avatar)} fit={owner.avatarFit} color={owner.color} size={52} />
					<span><strong>{owner.name}</strong><small>{$_("phoneUi.phoneOwnerMeta", { values: { kind: phone.isCharacterPhone ? $_("phoneUi.kindChar") : $_("phoneUi.kindPersonaShort") } })}</small></span>
					<ChevronRight />
				</button>
				<p class="section">{$_("phoneUi.personaPhonesSec")}</p>
				<div class="group">
					{#each library.personas as p (p.id)}
						{@const on = phone.prefs.owner?.kind === "persona" && phone.prefs.owner.id === p.id}
						<button class="opt" onclick={() => phone.switchOwner({ kind: "persona", id: p.id })}>
							<Avatar name={p.name} src={fileSrc(p.avatar)} fit={p.avatarFit} color={p.color || null} size={28} />{p.name}
							{#if on}<span class="check">✓</span>{/if}
						</button>
					{/each}
				</div>
				<p class="section">{$_("phoneUi.charPhonesSec")}</p>
				<div class="group">
					{#each library.characters as c (c.id)}
						{@const on = phone.prefs.owner?.kind === "character" && phone.prefs.owner.id === c.id}
						<button class="opt" onclick={() => phone.switchOwner({ kind: "character", id: c.id })}>
							<Avatar name={c.name} src={fileSrc(c.avatar)} fit={c.avatarFit} color={c.color} size={28} />{c.name}
							{#if on}<span class="check">✓</span>{/if}
						</button>
					{/each}
				</div>
				<p class="section">{$_("phoneUi.wallpaperSec")}</p>
				<div class="walls">
					{#each Object.entries(WALLPAPERS) as [id, bg] (id)}
						<button class="wall" class:on={phone.wallpaper === id} style:background={bg} aria-label={$_("phoneUi.wallpaperAria", { values: { id } })} onclick={() => phone.setWallpaper(id)}></button>
					{/each}
				</div>
				<p class="section">{$_("phoneUi.displaySec")}</p>
				<div class="group">
					<button class="opt" onclick={() => phone.setDisplay("mockup")}>{$_("phoneUi.displayMockup")}{#if phone.prefs.display === "mockup"}<span class="check">✓</span>{/if}</button>
					<button class="opt" onclick={() => phone.setDisplay("full")}>{$_("phoneUi.displayFull")}{#if phone.prefs.display === "full"}<span class="check">✓</span>{/if}</button>
				</div>
			</div>
		{/if}

		<button class="home-indicator" aria-label={$_("phoneUi.homeAria")} onclick={() => go("home")}></button>
	</div>
</div>
{#if panel.open && ready}
	<SidePanel label={$_("phoneUi.panelAria")}><PhonePanel {room} session={roomSession} onopen={openRoom} onjump={jumpInRoom} oncard={cardFromPanel} draft={roomDraft} /></SidePanel>
{/if}
</div>

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label={$_("phoneUi.contactMenuAria")} onclose={() => (menu = null)} />
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
		background: var(--surface);
		color: var(--text);
		overflow: hidden;
	}
	.device.framed {
		width: 380px;
		height: min(780px, 100%);
		flex: none;
		border-radius: 46px;
		border: 10px solid #1b1a18;
		box-shadow: var(--shadow-lg);
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
		background: linear-gradient(180deg, rgb(0 0 0 / 0.28) 0%, transparent 38%), var(--wall);
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
		background: var(--bg-sunken);
		color: var(--text-faint);
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
		color: var(--text);
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
		background: var(--surface-hover);
	}
	.none {
		padding: 40px 18px;
		color: var(--text-faint);
		font-size: 14px;
		text-align: center;
		line-height: 2;
	}
	.list .none .link {
		display: inline;
		width: auto;
		padding: 0;
		text-align: center;
		color: var(--accent);
	}
	.list .none .link:hover {
		background: none;
		text-decoration: underline;
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
		color: var(--text-faint);
	}
	.last {
		font-size: 13px;
		color: var(--text-muted);
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
		color: var(--text-faint);
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
		color: var(--text-faint);
	}

	/* ---- 설정 앱 ---- */
	.settings {
		flex: 1;
		overflow: auto;
		background: var(--bg);
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
		border: 1px solid var(--border);
		border-radius: 14px;
		background: var(--surface);
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
		color: var(--text-faint);
	}
	.profile :global(svg) {
		color: var(--text-faint);
	}
	.group {
		margin: 0 16px;
		border: 1px solid var(--border);
		border-radius: 14px;
		background: var(--surface);
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
	.opt:hover {
		background: var(--surface-hover);
	}
	.opt + .opt {
		border-top: 1px solid var(--border);
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
		background: var(--text-muted);
		opacity: 0.65;
		cursor: pointer;
	}
	.kakao-scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
		padding-bottom: 16px;
	}
	.inline-list {
		flex: none;
		padding: 0;
		overflow: visible;
	}
	.kakao-tabs {
		display: flex;
		border-top: 1px solid var(--border);
		background: var(--surface-2);
		padding: 6px 0 22px;
		flex: none;
	}
	.kakao-tabs button {
		flex: 1;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 2px;
		border: none;
		background: none;
		color: var(--text-faint);
		font: inherit;
		font-size: 11px;
		cursor: pointer;
	}
	.kakao-tabs button.on {
		color: var(--text);
		font-weight: 600;
	}
	.kakao-tabs :global(svg) {
		width: 22px;
		height: 22px;
	}
	.tab-ic {
		position: relative;
		display: inline-flex;
	}
	.tab-ic i {
		position: absolute;
		top: -4px;
		right: -10px;
		min-width: 16px;
		height: 16px;
		padding: 0 4px;
		border-radius: 8px;
		background: #ff3b30;
		color: #fff;
		font-style: normal;
		font-size: 10px;
		font-weight: 700;
		display: grid;
		place-items: center;
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
