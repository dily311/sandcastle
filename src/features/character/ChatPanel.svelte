<script lang="ts">
	import { ArrowUpDown, GripVertical, Bookmark, Check, ChevronDown, Eye, EyeOff, FileArchive, IdCard, ImagePlus, Pencil, Plus, Search, Trash2, X } from "@lucide/svelte";
	import { Avatar, Button, ContextMenu, IconButton, Input, ProfilePicker, Select, Spinner, Switch, type MenuItem, type ProfileOption } from "$shared/ui";
	import { bookmarks, bookmarksOf, onCurrentSwipe, type ChatBookmark } from "./bookmarks";
	import { assetImport } from "./assetImport.svelte";
	import { importPresetPaths } from "$features/settings/presetImport";
	import NotesPanel from "./NotesPanel.svelte";
	import BranchTree from "./BranchTree.svelte";
	import BookPicker from "$features/lorebook/BookPicker.svelte";
	import ContextMeter from "./ContextMeter.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import ExportButton from "./ExportButton.svelte";
	import PresetInline from "$features/settings/PresetInline.svelte";
	import RegexEditDialog from "$features/settings/RegexEditDialog.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import type { ContextUsage } from "$ipc/bindings";
	import { panel } from "$app/panel.svelte";
	import type { RegexScript_Serialize as RegexScript } from "$ipc/bindings";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { drops, extOf, isImage, storeFile } from "$app/drop.svelte";
	import { Sortable, moveItem } from "$shared/dnd/sortable.svelte";
	import { imageSrc } from "$shared/ipc/file";
	import { imageViewer } from "$shared/stores/imageViewer.svelte";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { convActions } from "./actions";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { preview } from "./format";
	import { library, type CharacterImage, type Conversation } from "./library.svelte";
	import { metaOf, type ChatSession } from "./session.svelte";
	import { lintOf } from "./lint";
	import MemoryIndex from "$features/memory/MemoryIndex.svelte";
	import SummaryPanel from "$features/memory/SummaryPanel.svelte";
	import MemorySearch from "$features/memory/MemorySearch.svelte";

	interface Props {
		conv: Conversation;
		session: ChatSession;
		chapters: Record<string, number>;
		/** 메시지로 이동. 북마크면 그 글이 있는 자리로 */
		onjump: (messageId: string, mark?: ChatBookmark) => void;
		/** 북마크한 글을 이미지 카드로 */
		oncard?: (mark: ChatBookmark) => void;
		/** 다른 대화 열기 (주면 그 메시지로) */
		onopen: (id: string, messageId?: string) => void;
		/** "나" 고르기를 바꿔 끼움 (폰: 폰 주인). 없으면 이 대화의 페르소나 */
		me?: { label: string; options: ProfileOption[]; value: string | null; hint?: string; onchange: (id: string) => void };
	}
	let { conv, session, chapters, onjump, oncard, onopen, me }: Props = $props();

	// ---- 메시지 linter: 이 대화의 덮어쓰기와 마지막 응답에서 걸린 규칙 (3-B-5) ----
	// 스위치는 `memoryOff`. 예전 버전은 메모(`memory`) 칸에 "off"를 썼다 — 그 값도 꺼짐으로 읽고 새로 바꿀 때 지운다
	const legacyMemoryOff = $derived((conv.meta as { memory?: string } | null)?.memory === "off");
	const memoryOff = $derived((conv.meta as { memoryOff?: boolean } | null)?.memoryOff === true || legacyMemoryOff);
	const memoText = $derived.by(() => {
		const m = (conv.meta as { memory?: string } | null)?.memory ?? "";
		return m === "off" ? "" : m;
	});
	const rewriteOff = $derived((conv.meta as { rewriteOff?: boolean } | null)?.rewriteOff === true);
	const lintOff = $derived((conv.meta as { lint?: string } | null)?.lint === "off");
	const lastLint = $derived.by(() => {
		const last = [...session.messages].reverse().find((m) => m.role === "assistant");
		return last ? lintOf(last) : null;
	});

	// ---- 대화 설정: API → 프리셋 → 페르소나 ----
	const DEFAULT = "";
	const apiOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const presetOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	function setOption(p: { apiProfileId?: string; promptPresetId?: string; personaId?: string; model?: string | null }) {
		void library.setOptions(conv.id, {
			personaId: p.personaId ?? conv.personaId,
			promptPresetId: (p.promptPresetId ?? conv.promptPresetId ?? "") || null,
			apiProfileId: (p.apiProfileId ?? conv.apiProfileId ?? "") || null,
			// 모델은 API에 딸린 이름: API를 바꾸면 백엔드가 초기화하므로 그대로 보낸다 (ADR-020)
			model: p.model !== undefined ? p.model : (conv.model ?? null),
		});
	}
	// 이 대화가 쓰는 API 프로필 (대화에 정한 것, 없으면 앱 기본값) — 모델 고르기가 그 프로필의 목록을 보여준다
	const effectiveApi = $derived(library.apis.find((a) => a.id === (conv.apiProfileId || appSetting.value?.selectedApiProfile || "")));
	/** 모델을 고르면 그 API도 이 대화에 못 박는다 (앱 기본 API가 나중에 바뀌어도 이 모델이 엉뚱한 공급자로 가지 않게) */
	function pickModel(model: string | null) {
		if (!conv.apiProfileId && effectiveApi) return setOption({ apiProfileId: effectiveApi.id, model });
		setOption({ model });
	}

	// ---- 컨텍스트 사용량: 대화가 바뀌거나 응답이 끝날 때마다 ----
	let usage = $state<ContextUsage | null>(null);
	async function loadUsage() {
		usage = await call(commands.contextUsage(conv.id), { source: "chat", silent: true }).catch(() => null);
	}
	$effect(() => {
		void conv.id;
		void conv.promptPresetId;
		void session.messages.length;
		if (!session.busy) void loadUsage();
	});

	// 프리셋: 컨텍스트 탭 아래에 바로 편집기 (PresetInline)
	const activePresetId = $derived(conv.promptPresetId || usage?.presetId || null);

	const character = $derived(library.character(conv.characterId));
	const persona = $derived(library.persona(conv.personaId) ?? library.personas[0]);
	const personaOptions = $derived<ProfileOption[]>(library.personas.map((p) => ({ id: p.id, name: p.name, avatar: p.avatar, color: p.color, description: p.description })));

	// ---- 정규식: 이 대화에 걸리는 것 (전역 + 이 캐릭터 + 이 프리셋) ----
	let scripts = $state<RegexScript[]>([]);
	$effect(() => {
		void conv.id;
		void call(commands.listRegexScripts(), { source: "regex", silent: true })
			.then((r) => (scripts = r))
			.catch(() => (scripts = []));
	});
	const applied = $derived(
		scripts.filter(
			(s) =>
				s.scope === "global" ||
				(s.scope === "character" && s.scopeId === conv.characterId) ||
				(s.scope === "preset" && !!conv.promptPresetId && s.scopeId === conv.promptPresetId),
		),
	);
	const scopeName = { global: "전역", character: "캐릭터", preset: "프리셋" } as const;
	// 적용 순서: 전역 → 프리셋 → 캐릭터, 같은 범위 안에서는 sortOrder (regex_processor.rs와 같음)
	const SCOPES = ["global", "preset", "character"] as const;
	const groups = $derived(SCOPES.map((scope) => ({ scope, list: applied.filter((s) => s.scope === scope).sort((a, b) => a.sortOrder - b.sortOrder) })).filter((g) => g.list.length));
	async function reorder(scope: string, from: number, to: number) {
		const list = moveItem(groups.find((g) => g.scope === scope)?.list ?? [], from, to);
		for (const [i, s] of list.entries()) {
			if (s.sortOrder === i * 10) continue;
			const saved = await call(commands.upsertRegexScript({ ...s, sortOrder: i * 10 }), { source: "regex" });
			scripts = scripts.map((x) => (x.id === s.id ? saved : x));
		}
	}
	const sorters = Object.fromEntries(SCOPES.map((scope) => [scope, new Sortable((from, to) => void reorder(scope, from, to))])) as Record<(typeof SCOPES)[number], Sortable>;
	// 정규식 바로 편집 (설정 화면과 같은 대화상자)
	let regexEditor: RegexEditDialog | undefined = $state();
	function onRegexSaved(saved: RegexScript) {
		scripts = scripts.some((x) => x.id === saved.id) ? scripts.map((x) => (x.id === saved.id ? saved : x)) : [...scripts, saved];
	}
	async function toggleScript(s: RegexScript, enabled: boolean) {
		const saved = await call(commands.upsertRegexScript({ ...s, enabled }), { source: "regex" });
		scripts = scripts.map((x) => (x.id === s.id ? saved : x));
	}

	// ---- 에셋 ----
	const assets = $derived(library.images[conv.characterId] ?? []);
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	let menuItems: MenuItem[] = $state([]);
	async function renameAsset(a: CharacterImage) {
		const key = await promptText({ title: "키 이름 바꾸기", message: "AI가 <cg>키</cg>로 부르는 이름", input: { value: a.key }, confirmLabel: "바꾸기" });
		if (!key || key === a.key) return;
		await library.updateImage(a, { key });
		const now = assets.find((x) => x.id === a.id);
		if (now && imageViewer.current?.src === imageSrc(a)) imageViewer.patch({ name: now.key, info: assetInfo(now) });
	}
	async function deleteAsset(a: CharacterImage) {
		if (!(await confirmAction({ title: "에셋 삭제", message: `'${a.key}' 이미지를 지움`, confirmLabel: "삭제", danger: true }))) return;
		await library.removeImage(a);
		imageViewer.close();
	}
	const assetInfo = (a: CharacterImage) => [
		{ label: "이미지 키", value: `<cg>${a.key}</cg>` },
		{ label: "AI에게", value: a.enabled ? "보냄" : "안 보냄" },
	];
	/** 에셋 크게 보기: 앱 공통 그림 뷰어에 이름 바꾸기·삭제를 이어 줌 */
	function openAsset(a: CharacterImage) {
		const src = imageSrc(a);
		if (src) imageViewer.open({ src, name: a.key, rename: () => renameAsset(a), remove: () => deleteAsset(a), info: assetInfo(a) });
	}
	function assetMenu(e: MouseEvent, a: CharacterImage) {
		e.preventDefault();
		menuItems = [
			{ label: a.enabled ? "AI에게 보내지 않기" : "AI에게 보내기", icon: a.enabled ? EyeOff : Eye, onselect: () => library.updateImage(a, { enabled: !a.enabled }) },
			{ label: "키 이름 바꾸기", icon: Pencil, onselect: () => renameAsset(a) },
			{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => deleteAsset(a) },
		];
		menu = { x: e.clientX, y: e.clientY };
	}
	/** 에셋을 넣는 작업은 앱에 두어 이 화면을 나가도 이어진다 (패널엔 로딩으로, 나가면 토스트로 보임). 변환은 한 장씩이라 ZIP이 크면 오래 걸림 */
	const withBusy = assetImport.run.bind(assetImport);
	const zipDone = (n: number) => toasts.success(n ? `에셋 ${n}개 넣음 (같은 이름은 덮어씀)` : "ZIP에 그림이 없음");
	async function addImages() {
		const paths = await pickFiles([IMAGE_FILES]);
		if (paths.length) await withBusy(`이미지 ${paths.length}장 넣는 중…`, () => library.addImages(conv.characterId, paths));
	}
	async function addZip() {
		const [zip] = await pickFiles([{ name: "ZIP", extensions: ["zip"] }], false);
		if (zip) await withBusy("ZIP 속 이미지를 변환하는 중… (많으면 몇 분 걸림)", async () => zipDone(await library.addImagesZip(conv.characterId, zip)));
	}
	// 정렬 · 여러 개 고르기 · 크게 보기
	let assetSort = $state<"order" | "name" | "new">("order");
	const sortedAssets = $derived(
		assetSort === "name" ? [...assets].sort((a, b) => a.key.localeCompare(b.key, "ko")) : assetSort === "new" ? [...assets].sort((a, b) => b.createdAt - a.createdAt) : assets,
	);
	let assetPicking = $state(false);
	let assetPicked = $state<Set<string>>(new Set());
	function pickAsset(a: CharacterImage) {
		const next = new Set(assetPicked);
		if (next.has(a.id)) next.delete(a.id);
		else next.add(a.id);
		assetPicked = next;
	}
	async function removePickedAssets() {
		const list = assets.filter((a) => assetPicked.has(a.id));
		if (!list.length) return;
		if (!(await confirmAction({ title: "에셋 삭제", message: `고른 이미지 ${list.length}개를 지움. 되돌릴 수 없음`, confirmLabel: `${list.length}개 삭제`, danger: true }))) return;
		for (const a of list) await library.removeImage(a);
		assetPicked = new Set();
		assetPicking = false;
	}
	function assetSortMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const opts = [
			{ v: "order", l: "넣은 순서" },
			{ v: "new", l: "최근 추가" },
			{ v: "name", l: "키 이름 순" },
		] as const;
		menuItems = opts.map((o) => ({ label: o.l, icon: assetSort === o.v ? Check : undefined, onselect: () => (assetSort = o.v) }));
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
	/** 오른쪽 패널 영역 (파일 끌어다 놓기를 이 안에서만 받음) */
	const panelZone = () => document.querySelector(".side-panel, .sheet");
	// 에셋 탭: 패널에 끌어다 놓은 그림은 캐릭터 에셋으로
	$effect(() => {
		if (panel.tab !== "assets" || !plugins.on("chat.images")) return;
		const charId = conv.characterId;
		return drops.register({
			priority: 2,
			zone: panelZone,
			label: "캐릭터 에셋에 넣기",
			hint: "파일 이름이 이미지 키가 됨 (<cg>키</cg>). 그림이 든 ZIP도 가능",
			accept: (f) => isImage(f) || extOf(f) === "zip",
			ondrop: (files) =>
				withBusy("파일을 옮기는 중…", async (set) => {
					const paths = await Promise.all(files.filter(isImage).map((f) => storeFile(f)));
					if (paths.length) {
						set(`이미지 ${paths.length}장 넣는 중…`);
						await library.addImages(charId, paths);
					}
					// ZIP은 에셋 추가 메뉴의 "ZIP"과 같은 처리
					for (const z of files.filter((f) => extOf(f) === "zip")) {
						set(`ZIP 옮기는 중… (${z.name})`);
						const path = await storeFile(z);
						set("ZIP 속 이미지를 변환하는 중… (많으면 몇 분 걸림)");
						zipDone(await library.addImagesZip(charId, path));
					}
				}),
		});
	});

	// 프리셋 탭: 패널에 끌어다 놓은 프리셋 JSON을 가져와 이 대화에 적용
	$effect(() => {
		if (panel.tab !== "context") return;
		return drops.register({
			priority: 2,
			zone: panelZone,
			label: "프리셋 가져오기",
			hint: "SillyTavern 프리셋 JSON. 하나만 놓으면 이 대화의 프리셋이 됨 (정규식 JSON을 같이 놓으면 그 프리셋에 묶임)",
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				const paths = await Promise.all(files.map((f) => storeFile(f)));
				const id = await importPresetPaths(paths).catch(() => null);
				if (id) {
					setOption({ promptPresetId: id });
					toasts.info("이 대화의 프리셋으로 골랐음");
				}
			},
		});
	});

	// ---- 북마크 ----
	const marks = $derived(bookmarksOf(conv));
	const msgNo = $derived(new Map(session.messages.map((m, i) => [m.id, i + 1])));
	const msgById = $derived(new Map(session.messages.map((m) => [m.id, m])));
	/** 북마크가 어느 스와이프의 것인지 (스와이프가 하나뿐이면 말하지 않음) */
	function swipeLabel(b: ChatBookmark): string {
		const m = msgById.get(b.messageId);
		const total = m ? (metaOf(m).swipes?.length ?? 0) : 0;
		return b.swipeId !== undefined && total > 1 ? `스와이프 ${b.swipeId + 1}/${total}` : "";
	}
	const onShownSwipe = (b: ChatBookmark) => {
		const m = msgById.get(b.messageId);
		return !m || onCurrentSwipe(b, metaOf(m).swipeId ?? 0);
	};
	const marksOfMsg = $derived.by(() => {
		const map = new Map<string, ChatBookmark[]>();
		for (const b of marks) map.set(b.messageId, [...(map.get(b.messageId) ?? []), b]);
		return map;
	});

	// ---- 목차 ----
	let tocQuery = $state("");
	const toc = $derived.by(() => {
		// under: 이 메시지가 속한 챕터(그 챕터를 시작한 메시지 id)
		const out: { kind: "chapter" | "msg"; id: string; n: number; label: string; under?: string; marks?: ChatBookmark[] }[] = [];
		let cur: string | undefined;
		session.messages.forEach((m, i) => {
			const meta = metaOf(m);
			if (meta.chapter) {
				cur = m.id;
				out.push({ kind: "chapter", id: m.id, n: chapters[m.id], label: `${chapters[m.id]}장 · ${meta.chapter}` });
			}
			out.push({ kind: "msg", id: m.id, n: i + 1, label: meta.hidden ? "(숨긴 메시지)" : preview(m.content) || "(빈 메시지)", under: cur, marks: marksOfMsg.get(m.id) });
		});
		return out;
	});
	// 접은 챕터 (검색 중에는 모두 펼쳐 보임)
	let folded = $state<Set<string>>(new Set());
	function toggleChapter(id: string) {
		const next = new Set(folded);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		folded = next;
	}
	const hasChapters = $derived(toc.some((t) => t.kind === "chapter"));
	const tocShown = $derived.by(() => {
		const q = tocQuery.trim().toLowerCase();
		if (!q) return toc.filter((t) => !(t.kind === "msg" && t.under && folded.has(t.under)));
		const num = Number(q.replace("#", ""));
		return toc.filter((t) => t.label.toLowerCase().includes(q) || (t.kind === "msg" && (t.n === num || !!t.marks?.some((b) => b.text.toLowerCase().includes(q)))));
	});

	// 목차 항목 오른쪽 클릭(폰: 꾹 누르기): 챕터 등록·이름 바꾸기·해제
	let tocMenuAt = $state<{ x: number; y: number } | null>(null);
	let tocMenuItems: MenuItem[] = $state([]);
	function tocContext(e: MouseEvent, id: string) {
		e.preventDefault();
		const m = session.messages.find((x) => x.id === id);
		if (!m) return;
		const cur = metaOf(m).chapter;
		const rename = async () => {
			const t = await promptText({ title: cur ? "챕터 이름 바꾸기" : "챕터로 등록", message: cur ? "비우면 챕터 표시를 지움" : `#${msgNo.get(id)} 메시지부터 새 챕터`, input: { value: cur ?? "", placeholder: "챕터 제목" }, confirmLabel: cur ? "바꾸기" : "만들기" });
			if (t !== null && t !== undefined) await session.setChapter(m, t.trim() || null);
		};
		tocMenuItems = cur
			? [
					{ label: "챕터 이름 바꾸기", icon: Pencil, onselect: rename },
					{ label: "챕터 표시 지우기", icon: X, danger: true, separator: true, onselect: () => session.setChapter(m, null) },
				]
			: [
					// 제목은 메시지 앞부분으로 바로 등록 (나중에 이름 바꾸기)
					{ label: "여기서 챕터 시작", icon: Bookmark, onselect: () => session.setChapter(m, preview(m.content).slice(0, 20).trim() || `#${msgNo.get(id)}`) },
					{ label: "이름 정해서 챕터 시작", icon: Pencil, onselect: rename },
				];
		tocMenuAt = { x: e.clientX, y: e.clientY };
	}
</script>


	{#if panel.tab === "assets"}
		<div class="panel-body">
			{#if plugins.on("chat.images")}
				<Switch checked={conv.imagesEnabled} label="이 대화에서 캐릭터 에셋 사용" onchange={(v) => library.setImagesEnabled(conv.id, v)} />
			{:else}
				<p class="faint">캐릭터 에셋 기능이 꺼져 있음 (설정 > 코어 플러그인)</p>
			{/if}
			{#if assetImport.busy}<p class="abusy" role="status"><Spinner />{assetImport.busy}</p>{/if}
			<div class="atools">
				<span class="faint">{assets.length}개 · 오른쪽 클릭으로 관리</span>
				<IconButton label="정렬" size="sm" onclick={assetSortMenu}><ArrowUpDown /></IconButton>
				<IconButton label="여러 개 삭제" size="sm" active={assetPicking} disabled={!assets.length} onclick={() => ((assetPicking = !assetPicking), (assetPicked = new Set()))}><Trash2 /></IconButton>
				<IconButton label="이미지 가져오기 (여러 장 선택)" size="sm" onclick={addImages}><ImagePlus /></IconButton>
				<IconButton label="ZIP으로 가져오기 (같은 이름은 덮어씀)" size="sm" onclick={addZip}><FileArchive /></IconButton>
			</div>
			{#if assetPicking}
				<div class="apick">
					<button class="link" onclick={() => (assetPicked = new Set(assets.map((a) => a.id)))}>모두 선택</button>
					<span class="sp"></span>
					<Button size="sm" variant="danger" disabled={!assetPicked.size} onclick={removePickedAssets}>{#snippet icon()}<Trash2 />{/snippet}{assetPicked.size}개 삭제</Button>
				</div>
			{/if}
			{#if assets.length}
				<div class="agrid">
					{#each sortedAssets as a (a.id)}
						<button class="tile" class:off={!a.enabled} class:picked={assetPicked.has(a.id)} title={a.enabled ? a.key : `${a.key} (AI에게 안 보냄)`} onclick={() => (assetPicking ? pickAsset(a) : openAsset(a))} oncontextmenu={(e) => assetMenu(e, a)}>
							<span class="thumb">
								<img src={imageSrc(a, true)} alt="" loading="lazy" />
								{#if !a.enabled}<EyeOff />{/if}
							</span>
							<span class="key">{a.key}</span>
						</button>
					{/each}
				</div>
			{:else}
				<p class="none">이미지 없음. 표정 이미지를 넣으면 AI가 장면에 맞게 골라 보여줌</p>
			{/if}
			<ContextMenu at={menu} items={menuItems} label="에셋 메뉴" onclose={() => (menu = null)} />
		</div>
	{:else if panel.tab === "branches"}
		<BranchTree {conv} {onopen} />
	{:else if panel.tab === "bookmarks"}
			<ol class="marks">
				{#each [...marks].reverse() as b (b.id)}
					<li class:dim={!onShownSwipe(b)}>
						<button class="mark" onclick={() => onjump(b.messageId, b)} title={onShownSwipe(b) ? "이 글로 이동" : "지금 보는 스와이프가 아님 (메시지로 이동)"} disabled={!msgNo.has(b.messageId)}>
							<span class="mtext">{b.text}</span>
							<span class="mmeta">{msgNo.has(b.messageId) ? `#${msgNo.get(b.messageId)}` : "지운 메시지"}{swipeLabel(b) ? ` · ${swipeLabel(b)}` : ""} · {new Date(b.createdAt).toLocaleDateString()}</span>
						</button>
						{#if oncard}<IconButton label="이미지 카드로 만들기" size="sm" onclick={() => oncard(b)}><IdCard /></IconButton>{/if}
						<IconButton label="북마크 지우기" size="sm" onclick={() => bookmarks.remove(conv, b.id)}><X /></IconButton>
					</li>
				{:else}
					<li class="none"><Bookmark /><br />메시지 글을 끌어서 고른 뒤 '북마크'를 누르면 여기 모임</li>
				{/each}
			</ol>
	{:else if panel.tab === "toc"}
		<div class="toc-search">
			<Input bind:value={tocQuery} placeholder="챕터·메시지·번호 검색" aria-label="목차 검색">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			{#if hasChapters}
				<button class="fold-all" onclick={() => (folded = folded.size ? new Set() : new Set(toc.filter((t) => t.kind === "chapter").map((t) => t.id)))}>{folded.size ? "모두 펼치기" : "모두 접기"}</button>
			{/if}
		</div>
		<ol class="toc">
			{#each tocShown as t (t.kind + t.id)}
				<li class={t.kind}>
					{#if t.kind === "chapter"}
						<button class="fold" aria-label={folded.has(t.id) ? "챕터 펼치기" : "챕터 접기"} aria-expanded={!folded.has(t.id)} onclick={() => toggleChapter(t.id)}><ChevronDown class={folded.has(t.id) ? "rot" : ""} /></button>
						<button onclick={() => onjump(t.id)} oncontextmenu={(e) => tocContext(e, t.id)}>{t.label}</button>
					{:else}
						<button onclick={() => onjump(t.id)} oncontextmenu={(e) => tocContext(e, t.id)}><span class="no">#{t.n}</span>{t.label}</button>
					{/if}
					{#if t.marks?.length}
						<ul class="tbm">
							{#each t.marks as b (b.id)}
								<li class:dim={!onShownSwipe(b)}><button onclick={() => onjump(t.id, b)} title="이 글로 이동"><Bookmark />{b.text}</button></li>
							{/each}
						</ul>
					{/if}
				</li>
			{:else}
				<li class="none">{tocQuery ? "찾는 항목 없음" : "메시지 없음"}</li>
			{/each}
		</ol>
		<ContextMenu at={tocMenuAt} items={tocMenuItems} label="목차 메뉴" onclose={() => (tocMenuAt = null)} />
	{:else if panel.tab === "context"}
		<div class="panel-body preset-edit">
			<section>
				<h4>컨텍스트</h4>
				<ContextMeter {usage} />
			</section>
			<section>
				<h4>프리셋 <span class="faint">{conv.promptPresetId ? "이 대화에서 고름" : "앱·캐릭터 기본값"}</span></h4>
				<Select value={conv.promptPresetId ?? DEFAULT} options={presetOptions} aria-label="이 대화의 프리셋" onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} />
			</section>
			<PresetInline presetId={activePresetId} onsaved={loadUsage} model={conv.model || effectiveApi?.model || null} />
		</div>
	{:else}
		<div class="panel-body">
			<section>
				<h4>이 대화</h4>
				<label class="row"><span>API</span><Select value={conv.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} /></label>
					<div class="row"><span>모델</span><ModelPicker profile={effectiveApi} value={conv.model} onchange={pickModel} label="이 대화의 모델" /></div>
				<div class="row">
					<span>프리셋</span>
					<div class="with-btn">
						<Select value={conv.promptPresetId ?? DEFAULT} options={presetOptions} aria-label="프리셋" onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} />
						<IconButton label="이 프리셋 빠르게 편집{usage?.presetName ? ` (${usage.presetName})` : ''}" size="sm" onclick={() => (panel.tab = "context")}><Pencil /></IconButton>
					</div>
				</div>
				<div class="row">
					{#if me}
						<span>{me.label}</span>
						<ProfilePicker label="{me.label} 바꾸기" options={me.options} value={me.value} hint={me.hint} onchange={me.onchange} />
					{:else}
						<span>페르소나</span>
						<ProfilePicker label="페르소나 바꾸기" options={personaOptions} value={persona?.id} onchange={(id) => setOption({ personaId: id })} />
					{/if}
				</div>
			</section>
			{#if plugins.on("lint")}
				<hr />
				<section>
					<h4>메시지 linter <span class="faint">응답이 저장될 때 검사</span></h4>
					<Switch checked={!lintOff} label="이 대화에서 linter 쓰기" description="끄면 이 대화의 새 응답은 검사하지 않음 (규칙은 설정 > 메시지 linter)" onchange={(v) => library.patchMeta(conv.id, { lint: v ? null : "off" })} />
					{#if plugins.on("rewrite")}
						<Switch checked={!rewriteOff} label="이 대화에서 자동 재생성" onchange={(v) => library.patchMeta(conv.id, { rewriteOff: v ? null : true })} />
					{/if}
					{#if !lintOff}
						{#if lastLint}
							<p class="faint">마지막 응답에서 걸린 것</p>
							<ul class="lint-hits">
								{#each lastLint.findings as f, i (i)}
									<li class:warn={f.level === "warn"}>{f.text}{f.fixed ? " → 고침" : ""}</li>
								{/each}
							</ul>
						{:else}
							<p class="faint">마지막 응답에서 걸린 것 없음</p>
						{/if}
					{/if}
				</section>
			{/if}
			{#if plugins.on("summary")}
				<hr />
				<SummaryPanel {conv} memo={memoText} />
			{/if}
			{#if plugins.on("memory")}
				<hr />
				<section>
					<h4>메모리 검색</h4>
					<Switch checked={!memoryOff} label="이 대화를 메모리에 포함" description="끄면 색인과 전체 검색에서 이 대화를 뺌" onchange={(v) => library.patchMeta(conv.id, { memoryOff: v ? null : true, ...(legacyMemoryOff ? { memory: null } : {}) })} />
					{#if !memoryOff}
						<div class="mem-actions"><MemoryIndex conversationId={conv.id} /></div>
						<MemorySearch conversationId={conv.id} onopen={(h) => onopen(conv.id, h.firstMessageId)} />
					{/if}
				</section>
			{/if}
			<hr />
			<section>
				<h4>월드인포</h4>
				<div class="wi">
					<span>{character?.name ?? "캐릭터"} <small>캐릭터에 붙음 · 이 캐릭터의 모든 대화</small></span>
					<BookPicker label="캐릭터 월드인포" value={character?.worldInfoIds ?? []} onchange={(ids) => library.setCharacterWorldInfo(conv.characterId, ids)} />
				</div>
				<div class="wi">
					<span>이 대화 전용 <small>이 대화에만</small></span>
					<BookPicker label="이 대화 전용 월드인포" value={conv.worldInfoIds} onchange={(ids) => library.setWorldInfo(conv.id, ids)} />
				</div>
			</section>
			<hr />
			<NotesPanel {conv} bare />
			<hr />
			<section>
				<h4>정규식 <span class="faint">적용 중 {applied.filter((s) => s.enabled).length}개 · 위에서부터 적용, 끌어서 순서 바꾸기</span></h4>
				{#each groups as g (g.scope)}
					<p class="rx-scope">{scopeName[g.scope]}</p>
					<ul class="rx">
						{#each g.list as s, i (s.id)}
							{@const sort = sorters[g.scope]}
							<li {...sort.item(i)} class:dragging={sort.from === i} class:over={sort.over === i && sort.from !== i}>
								<span class="grip" aria-hidden="true" {...sort.handle(i)}><GripVertical /></span>
								<IconButton label="{s.name} 편집" size="sm" onclick={() => regexEditor?.startEdit(s)}><Pencil /></IconButton>
								<Switch checked={s.enabled} label={s.name} onchange={(v) => toggleScript(s, v)} />
							</li>
						{/each}
					</ul>
				{:else}
					<p class="none small">이 대화에 걸리는 정규식 없음</p>
				{/each}
				<Button size="sm" variant="ghost" onclick={() => regexEditor?.startNew({ scope: "character", scopeId: conv.characterId })}>{#snippet icon()}<Plus />{/snippet}이 캐릭터 정규식 추가</Button>
				<RegexEditDialog bind:this={regexEditor} sortOrder={applied.length * 10} onsaved={onRegexSaved} />
			</section>
			<hr />
			<section>
				<h4>내보내기</h4>
				<ExportButton {conv} />
			</section>
			<hr />
			<section>
				<Button block variant="danger" onclick={() => convActions.remove(conv)}>
					{#snippet icon()}<Trash2 />{/snippet}
					이 대화 삭제
				</Button>
			</section>
		</div>
	{/if}

<style>
	.wi {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: var(--text-sm);
	}
	.wi small {
		margin-left: 4px;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.with-btn {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
	}
	.with-btn > :global(:first-child) {
		flex: 1;
		min-width: 0;
	}
	hr {
		width: 100%;
		margin: 0;
		border: none;
		border-top: 1px solid var(--border);
	}
	/* 좁은 패널: 설정 화면용 줄을 세로로 */
	.preset-edit :global(.sliders) {
		grid-template-columns: minmax(0, 1fr);
	}
	.preset-edit :global(.row:not(.stacked)) {
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.panel-body {
		display: flex;
		flex-direction: column;
		gap: var(--space-5);
		padding: var(--space-3) var(--space-4) var(--space-4);
		overflow: auto;
	}
	.mem-actions {
		margin: var(--space-2) 0;
	}
	.lint-hits {
		margin: var(--space-1) 0 0;
		padding-left: var(--space-4);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.lint-hits li.warn {
		color: var(--warning);
	}
	section {
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
		grid-template-columns: 64px minmax(0, 1fr);
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.persona {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.atools {
		display: flex;
		align-items: center;
		gap: 2px;
		margin-bottom: calc(var(--space-2) * -1);
	}
	.abusy {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin: 0;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		background: var(--accent-soft);
		font-size: var(--text-sm);
	}
	.atools span {
		margin-right: auto;
		font-size: var(--text-xs);
	}
	.rx-scope {
		margin: var(--space-2) 0 2px;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.rx {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.rx li {
		display: flex;
		align-items: center;
		gap: 4px;
		margin-bottom: 4px;
		border-radius: var(--radius-sm);
		cursor: grab;
	}
	.rx li:last-child {
		margin-bottom: 0;
	}
	.rx li :global(.switch-row),
	.rx li > :global(:last-child) {
		flex: 1;
		min-width: 0;
	}
	.rx li.dragging {
		opacity: 0.4;
	}
	.rx li.over {
		box-shadow: inset 0 2px 0 var(--accent);
	}
	.rx .grip {
		display: grid;
		color: var(--text-faint);
	}
	.rx .grip :global(svg) {
		width: 14px;
		height: 14px;
	}
	.apick {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.apick .sp {
		flex: 1;
	}
	.apick .link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.tile.picked {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
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
		cursor: zoom-in;
	}
	.tile:hover {
		border-color: var(--border-strong);
	}
	.thumb {
		position: relative;
		display: grid;
		place-items: center;
		aspect-ratio: 1;
		background: linear-gradient(135deg, var(--accent-soft-hover), var(--bg-sunken));
		color: var(--text-muted);
	}
	.thumb img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.thumb :global(svg) {
		position: relative;
	}
	.tile.off .thumb img {
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
	.toc-search {
		padding: var(--space-3) var(--space-3) var(--space-2);
	}
	.toc {
		list-style: none;
		margin: 0;
		padding: 0 var(--space-2) var(--space-4);
		overflow: auto;
	}

	.toc li {
		overflow: hidden;
	}
	/* 메시지 아래에 붙는 북마크 */
	.tbm {
		list-style: none;
		margin: 0 0 var(--space-1);
		padding: 0 0 0 calc(var(--space-2) + 30px + var(--space-2));
	}
	.toc .tbm button {
		display: block;
		padding: 2px var(--space-2);
		border-left: 2px solid var(--warning);
		border-radius: 0 var(--radius-xs) var(--radius-xs) 0;
		font-size: var(--text-xs);
		white-space: normal;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
	}
	.tbm :global(svg) {
		display: inline;
		width: 11px;
		height: 11px;
		margin-right: 4px;
		vertical-align: -1px;
		color: var(--warning);
	}
	/* 지금 보는 스와이프의 북마크가 아님 */
	.dim {
		opacity: 0.45;
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
	.toc .chapter {
		display: flex;
		align-items: center;
	}
	.toc .chapter .fold {
		flex: 0 0 auto;
		width: 24px;
		padding: 0;
		display: grid;
		place-items: center;
	}
	.toc .fold :global(svg) {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.toc .fold :global(.rot) {
		transform: rotate(-90deg);
	}
	.fold-all {
		margin-top: 6px;
		padding: 2px 6px;
		border: none;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.toc .chapter button {
		font-weight: 700;
		color: var(--text);
		margin-top: var(--space-2);
	}
	.toc .no {
		width: 30px;
		flex: none;
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.none {
		padding: var(--space-4);
		text-align: center;
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.marks {
		list-style: none;
		margin: 0;
		padding: var(--space-2);
		overflow: auto;
	}
	.marks li:not(.none) {
		display: flex;
		align-items: flex-start;
		gap: 2px;
		border-radius: var(--radius-md);
	}
	.marks li:not(.none):hover {
		background: var(--surface-hover);
	}
	.mark {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: var(--space-2);
		border: none;
		background: transparent;
		text-align: left;
		cursor: pointer;
	}
	.mark:disabled {
		cursor: default;
	}
	.mtext {
		font-size: var(--text-sm);
		line-height: 1.5;
		border-left: 2px solid var(--accent);
		padding-left: var(--space-2);
		display: -webkit-box;
		-webkit-line-clamp: 4;
		line-clamp: 4;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.mmeta {
		font-size: var(--text-xs);
		color: var(--text-faint);
		padding-left: calc(var(--space-2) + 2px);
	}
	.none.small {
		padding: 0;
		text-align: left;
	}
</style>
