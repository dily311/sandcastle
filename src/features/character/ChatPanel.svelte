<script lang="ts">
	import UtilMacroNote from "./UtilMacroNote.svelte";
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { PictureInPicture2, ArrowUpDown, GripVertical, Bookmark, Check, ChevronDown, Eye, EyeOff, FileArchive, Folder, IdCard, ImagePlus, Pencil, Plus, Search, Trash2, X, ChevronsDownUp, ChevronsUpDown } from "@lucide/svelte";
	import { Avatar, Button, ChipPicker, ContextMenu, IconButton, Input, ProfilePicker, Select, Spinner, Switch, type MenuItem, type ProfileOption } from "$shared/ui";
	import { bookmarks, bookmarksOf, onCurrentSwipe, type ChatBookmark } from "./bookmarks";
	import { assetImport } from "./assetImport.svelte";
	import { importPresetPaths } from "$features/settings/presetImport";
	import NotesPanel from "./NotesPanel.svelte";
	import BranchTree from "./BranchTree.svelte";
	import BookPicker from "$features/lorebook/BookPicker.svelte";
	import ContextMeter from "./ContextMeter.svelte";
	import ModelPicker from "$features/settings/ModelPicker.svelte";
	import OptionLock from "./OptionLock.svelte";
	import VariablesPanel from "./VariablesPanel.svelte";
	import StatusWindow from "$features/status/StatusWindow.svelte";
	import type { Block } from "./format";
	import { currentStatusBlocks } from "$features/status/current";
	import { statusView } from "$features/status/statusView.svelte";
	import { layout } from "$app/layout.svelte";
	import { reasoningOf } from "$features/settings/providers";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import ExportButton from "./ExportButton.svelte";
	import PresetInline from "$features/settings/PresetInline.svelte";
	import RegexEditDialog from "$features/settings/RegexEditDialog.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import type { ContextUsage } from "$ipc/bindings";
	import { panel } from "$app/panel.svelte";
	import type { RegexScript_Serialize as RegexScript, Value } from "$ipc/bindings";
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
	import QuickReplyList from "$features/slash/QuickReplyList.svelte";

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
		/** 입력칸 글 (빠른 답장 탭이 넣거나 {{input}}으로 읽음) */
		draft?: { get: () => string; set: (v: string) => void };
		/** 읽기 화면에서 지금 읽는 메시지 (5-E-6-1): 목차가 그 메시지·챕터를 표시 */
		reading?: string | null;
		/** 읽기 화면: 그 메시지 시점의 상태창 (없으면 가장 최근) */
		status?: Block[];
	}
	let { conv, session, chapters, onjump, oncard, onopen, me, draft, reading = null, status }: Props = $props();

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
	/** 이 대화의 추론 강도 (5-E-7). auto = 프리셋·공급자 기본. 모델이 강도를 받을 때만 보인다 */
	/** 지금 쓰는 모델: 잠근 대화는 대화의 것, 풀린 대화는 전역 모델(5-E-29-3) — 없으면 API 프로필의 기본 */
	const shownModel = $derived(conv.apiProfileId ? conv.model : (appSetting.value?.selectedModel ?? null));
	const reasoning = $derived(reasoningOf(shownModel || effectiveApi?.model || null));
	/** 지금 쓰는 추론 강도: 잠근 대화는 대화의 것, 풀린 대화는 전역 */
	const effort = $derived(((conv.meta as { reasoningEffort?: string | null } | null)?.reasoningEffort ?? appSetting.value?.reasoningEffort ?? "auto") as string);
	/** 모델 고르기: 잠근 대화는 이 대화에, 풀린 대화는 전역 모델 (ADR-028 갱신). 폰 대화는 지금처럼 이 대화에 못 박는다 */
	function pickModel(model: string | null) {
		if (lockable && !conv.apiProfileId) return void appSetting.update({ selectedModel: model });
		if (!conv.apiProfileId && effectiveApi) return setOption({ apiProfileId: effectiveApi.id, model });
		setOption({ model });
	}

	const character = $derived(library.character(conv.characterId));
	const persona = $derived(library.persona(library.convPersonaId(conv)) ?? library.personas[0]);

	// ---- 컨텍스트 사용량: 지금 대화·설정 상태로 계산 ----
	let usage = $state<ContextUsage | null>(null);
	let usageSeq = 0;
	async function loadUsage() {
		const seq = ++usageSeq;
		const next = await call(commands.contextUsage(conv.id), { source: "chat", silent: true }).catch(() => null);
		if (seq === usageSeq) usage = next;
	}
	const msgUsageStamp = $derived(session.messages.map((m) => `${m.id}:${m.updatedAt}:${m.content.length}:${metaOf(m).hidden ? 1 : 0}`).join(","));
	const assetUsageStamp = $derived((library.images[conv.characterId] ?? []).map((a) => `${a.id}:${a.key}:${a.enabled ? 1 : 0}:${a.folder}`).join(","));
	$effect(() => {
		void conv.id;
		void conv.promptPresetId;
		void conv.personaId;
		void conv.imagesEnabled;
		void conv.worldInfoIds;
		void conv.meta;
		void character?.defaultPromptPresetId;
		void character?.defaultPersonaId;
		void character?.description;
		void character?.exampleDialogue;
		void character?.systemPromptOverride;
		void character?.worldInfoIds;
		void persona?.id;
		void persona?.description;
		void appSetting.value?.selectedPreset;
		void appSetting.value?.cgGroupVariants;
		void library.assetVersion[conv.characterId];
		void assetUsageStamp;
		void msgUsageStamp;
		void regexVersion.n;
		void plugins.on("chat.images");
		void plugins.on("summary");
		void plugins.on("imageauto");
		void plugins.on("status");
		if (panel.tab === "context") void panel.tab;
		if (!session.busy) void loadUsage();
	});

	// 프리셋: 컨텍스트 탭 아래에 바로 편집기 (PresetInline)
	const activePresetId = $derived(conv.promptPresetId || usage?.presetId || null);

	// ---- 자물쇠 (5-E-29, ADR-028): 이 대화 > 캐릭터 기본 > 전역. 잠김 = 이 대화에 값이 있음 ----
	// 폰 대화는 폰 주인·폰 설정을 따르므로 지금처럼 (자물쇠 없음)
	const lockable = $derived(conv.mode !== "phone");
	const lockedApi = $derived(!!conv.apiProfileId);
	const lockedPreset = $derived(!!conv.promptPresetId);
	const lockedPersona = $derived(!!conv.personaId);
	const lockedEffort = $derived((conv.meta as { reasoningEffort?: string | null } | null)?.reasoningEffort != null);
	const charPreset = $derived(character?.defaultPromptPresetId || null);
	/** 풀렸을 때 쓰는 프리셋: 캐릭터 기본 → 전역 (백엔드와 같은 순서) */
	const shownPreset = $derived(conv.promptPresetId || charPreset || appSetting.value?.selectedPreset || null);
	const presetFrom = $derived(charPreset ? "캐릭터 기본" : "전역");
	const personaFrom = $derived(character?.defaultPersonaId ? "캐릭터 기본" : "전역");
	/** 잠그면 지금 보이는 값을 이 대화에 넣고, 풀면 비운다 */
	function toggleApi() {
		if (lockedApi) setOption({ apiProfileId: "", model: null });
		else if (effectiveApi) setOption({ apiProfileId: effectiveApi.id, model: null });
	}
	function togglePreset() {
		if (lockedPreset) setOption({ promptPresetId: "" });
		else if (shownPreset) setOption({ promptPresetId: shownPreset });
	}
	function togglePersona() {
		setOption({ personaId: lockedPersona ? "" : library.convPersonaId(conv) });
	}
	function toggleEffort() {
		void library.patchMeta(conv.id, { reasoningEffort: lockedEffort ? null : effort });
	}
	// ---- 고르기 (5-E-29-3, ADR-028 갱신): 잠근 줄은 이 대화에, 풀린 줄은 언제나 전역에 ----
	// 캐릭터 전용 값은 캐릭터 편집에서만 바꾼다. 풀린 줄이 캐릭터 전용 값을 쓰고 있으면 전역을 바꿔도 이 대화는 그대로임을 알린다
	function charStillWins(kind: string, name: string | undefined) {
		toasts.info(`전역 ${kind}를 바꿈 · 이 캐릭터는 전용 ${kind} '${name ?? "?"}'를 씀 (캐릭터 편집에서 바꿈)`);
	}
	function pickApi(id: string) {
		if (lockable && !lockedApi) return void appSetting.update({ selectedApiProfile: id || null, selectedModel: null });
		setOption({ apiProfileId: id });
	}
	function pickEffort(v: string) {
		if (lockable && !lockedEffort) return void appSetting.update({ reasoningEffort: v === "auto" ? null : v });
		void library.patchMeta(conv.id, { reasoningEffort: lockable ? v : v === "auto" ? null : v });
	}
	async function pickPreset(id: string) {
		if (!lockable || lockedPreset) return setOption({ promptPresetId: id });
		await appSetting.update({ selectedPreset: id || "default" });
		if (charPreset && charPreset !== id) charStillWins("프리셋", library.presets.find((p) => p.id === charPreset)?.name);
	}
	async function pickPersona(id: string) {
		if (!lockable || lockedPersona) return setOption({ personaId: id });
		await library.setDefaultPersona(id);
		const own = character?.defaultPersonaId;
		if (own && own !== id) charStillWins("페르소나", library.persona(own)?.name);
	}
	const personaOptions = $derived<ProfileOption[]>(library.personas.map((p) => ({ id: p.id, name: p.name, avatar: p.avatar, avatarFit: p.avatarFit, color: p.color, description: p.description })));

	// ---- 정규식: 이 대화에 걸리는 것 (전역 + 이 캐릭터 + 이 프리셋) ----
	let scripts = $state<RegexScript[]>([]);
	$effect(() => {
		void conv.id;
		void regexVersion.n;
		void call(commands.listRegexScripts(), { source: "regex", silent: true })
			.then((r) => (scripts = r))
			.catch(() => (scripts = []));
	});
	const applied = $derived(
		scripts.filter(
			(s) =>
				s.scope === "global" ||
				(s.scope === "character" && s.scopeId === conv.characterId) ||
				(s.scope === "preset" && !!shownPreset && s.scopeId === shownPreset),
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
		regexVersion.bump();
	}
	const sorters = Object.fromEntries(SCOPES.map((scope) => [scope, new Sortable((from, to) => void reorder(scope, from, to))])) as Record<(typeof SCOPES)[number], Sortable>;
	// 정규식 바로 편집 (설정 화면과 같은 대화상자)
	let regexEditor: RegexEditDialog | undefined = $state();
	function onRegexSaved(saved: RegexScript) {
		scripts = scripts.some((x) => x.id === saved.id) ? scripts.map((x) => (x.id === saved.id ? saved : x)) : [...scripts, saved];
	}
	// 상태창 정규식 (5-F-5-1): 화면에만 쓰는 정규식 중 "상태창으로" 고른 것의 결과는 고정 자리로
	const statusRegexes = $derived(applied.filter((s) => s.target === "display" && s.enabled));
	const isStatusRegex = (s: RegexScript) => !!(s.extraJson as { statusWindow?: boolean } | null)?.statusWindow;
	
	/** 상태 탭의 상태창: 가장 최근 메시지의 것 (5-F-4-2) */
	const selectedPinKeys = $derived(new Set(statusRegexes.filter(isStatusRegex).map((s) => `status:${s.id}`)));
	const statusBlocks = $derived(panel.tab !== "status" ? [] : (status ?? currentStatusBlocks(conv, session.messages, session.display, plugins.on("html"), undefined, selectedPinKeys)));

	async function toggleStatusRegex(s: RegexScript, on: boolean) {
		const plain = JSON.parse(JSON.stringify(s)) as RegexScript;
		const extraJson: Record<string, Value> = { ...((plain.extraJson as Record<string, Value> | null) ?? {}) };
		if (on) extraJson.statusWindow = true;
		else delete extraJson.statusWindow;
		const saved = await call(commands.upsertRegexScript({ ...plain, extraJson }), { source: "regex" });
		regexVersion.bump();
		scripts = scripts.map((x) => (x.id === s.id ? saved : x));
	}
	/** 고른 목록으로 맞춘다 (달라진 것만 저장) */
	async function setStatusRegexes(ids: string[]) {
		for (const s of statusRegexes) {
			const on = ids.includes(s.id);
			if (on !== isStatusRegex(s)) await toggleStatusRegex(s, on);
		}
	}
	async function toggleScript(s: RegexScript, enabled: boolean) {
		const saved = await call(commands.upsertRegexScript({ ...s, enabled }), { source: "regex" });
		regexVersion.bump();
		scripts = scripts.map((x) => (x.id === s.id ? saved : x));
	}

	// ---- 에셋 ----
	const assets = $derived(library.images[conv.characterId] ?? []);
	// 판 폴더 (5-E-21): ZIP의 하위 폴더(v1/·v2/)가 판이 된다. 캐릭터마다 쓸 판을 고르면 그 판과 공통 그림만 대화에 쓴다
	const folders = $derived([...new Set(assets.map((a) => a.folder).filter(Boolean))].sort((a, b) => a.localeCompare(b, "ko")));
	const version = $derived(library.assetVersion[conv.characterId] ?? null);
	const inUse = (a: CharacterImage) => !version || !a.folder || a.folder.toLowerCase() === version.toLowerCase();
	async function moveAsset(a: CharacterImage) {
		const f = await promptText({ title: "판 폴더", message: "이 그림의 판 (비우면 공통 — 어느 판에서나 씀)", input: { value: a.folder, placeholder: "예: v1" }, confirmLabel: "옮기기" });
		if (f === null || f === undefined || f.trim() === a.folder) return;
		await library.updateImage(a, { folder: f.trim() });
	}
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
		{ label: "판", value: a.folder || "공통" },
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
			{ label: a.folder ? `판 옮기기 (지금 ${a.folder})` : "판 폴더 정하기", icon: Folder, onselect: () => moveAsset(a) },
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
	/** 지금 읽는 메시지가 속한 챕터 (5-E-6-1) */
	const readingChapter = $derived(reading ? toc.find((t) => t.kind === "msg" && t.id === reading)?.under : undefined);
	let tocList: HTMLOListElement | undefined = $state();
	// 읽는 자리가 바뀌면 목차에서도 보이게 (목록 안에서만 움직임)
	$effect(() => {
		void reading;
		const el = tocList?.querySelector<HTMLElement>(".current");
		el?.scrollIntoView({ block: "nearest" });
	});
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


	{#if panel.tab === "quickreply" && draft}
		<div class="panel-body">
			<QuickReplyList {session} {conv} getDraft={draft.get} setDraft={draft.set} />
		</div>
	{:else if panel.tab === "status" && plugins.on("status")}
		<div class="panel-body">
			<section class="status-pin">
				{#if !layout.narrow && statusView.floating}
					<div class="status-head">
						<span class="faint">띄운 창에 보이는 중</span>
						<Button size="sm" variant="ghost" onclick={() => statusView.set({ floating: false })}>패널로</Button>
					</div>
				{:else}
					<div class="status-head">
						{#if layout.narrow}
							<Switch checked={statusView.aboveChat} label="채팅방 위에 늘 보이기" onchange={(v) => statusView.set({ aboveChat: v })} />
						{:else}
							<span></span>
							<IconButton label="띄우기 (옮길 수 있는 창)" size="sm" onclick={() => statusView.set({ floating: true, collapsed: false })}><PictureInPicture2 /></IconButton>
						{/if}
					</div>
					<StatusWindow blocks={statusBlocks} />
				{/if}
			</section>
			{#if statusRegexes.length}
				<!-- 상태창 정규식 (5-F-5-3, 사용자 요청): 로어북 고르기처럼 고른 것만 칩으로, '+'로 더 넣기 -->
				<section class="status-regex">
					<h4>상태창 정규식</h4>
					<ChipPicker
						label="상태창 정규식"
						options={statusRegexes.map((s) => ({ id: s.id, label: s.name }))}
						value={statusRegexes.filter(isStatusRegex).map((s) => s.id)}
						onchange={(ids) => void setStatusRegexes(ids)}
					/>
				</section>
			{/if}
			<VariablesPanel {conv} />
		</div>
	{:else if panel.tab === "assets"}
		<div class="panel-body">
			{#if plugins.on("chat.images")}
				<Switch checked={conv.imagesEnabled} label="이 대화에서 캐릭터 에셋 사용" onchange={(v) => library.setImagesEnabled(conv.id, v)} />
				{#if conv.imagesEnabled}<UtilMacroNote presetId={shownPreset} macro="img_inprompt" />{/if}
			{:else}
				<p class="faint">캐릭터 에셋 기능이 꺼져 있음 (설정 > 코어 플러그인)</p>
			{/if}
			{#if assetImport.busy}<p class="abusy" role="status"><Spinner />{assetImport.busy}</p>{/if}
			{#if folders.length}
				<label class="row"
					><span>쓸 판</span><Select
						value={version ?? ""}
						options={[{ value: "", label: "전체" }, ...folders.map((f) => ({ value: f, label: f }))]}
						aria-label="이 캐릭터가 쓸 에셋 판"
						onchange={(e) => void library.setAssetVersion(conv.characterId, e.currentTarget.value || null)}
					/></label
				>
			{/if}
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
						<button class="tile" class:off={!a.enabled || !inUse(a)} class:picked={assetPicked.has(a.id)} title={a.enabled ? a.key : `${a.key} (AI에게 안 보냄)`} onclick={() => (assetPicking ? pickAsset(a) : openAsset(a))} oncontextmenu={(e) => assetMenu(e, a)}>
							<span class="thumb">
								<img src={imageSrc(a, true)} alt="" loading="lazy" />
								{#if !a.enabled}<EyeOff />{/if}
							</span>
							<span class="key">{#if a.folder}<small class="ver">{a.folder}</small> {/if}{a.key}</span>
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
				{#if folded.size}
					<IconButton label="모두 펼치기" size="sm" onclick={() => (folded = new Set())}><ChevronsUpDown /></IconButton>
				{:else}
					<IconButton label="모두 접기" size="sm" onclick={() => (folded = new Set(toc.filter((t) => t.kind === "chapter").map((t) => t.id)))}><ChevronsDownUp /></IconButton>
				{/if}
			{/if}
		</div>
		<ol class="toc" bind:this={tocList}>
			{#each tocShown as t (t.kind + t.id)}
				{@const cur = t.kind === "msg" ? t.id === reading : t.id === readingChapter}
				<li class={t.kind} class:current={cur} aria-current={cur ? "location" : undefined}>
					{#if t.kind === "chapter"}
						<button class="fold" aria-label={folded.has(t.id) ? "챕터 펼치기" : "챕터 접기"} aria-expanded={!folded.has(t.id)} onclick={() => toggleChapter(t.id)}><ChevronDown class={folded.has(t.id) ? "rot" : ""} /></button>
						<button class="chapter" onclick={() => onjump(t.id)} oncontextmenu={(e) => tocContext(e, t.id)}>{t.label}</button>
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
	{:else if panel.tab === "lint"}
		<div class="panel-body">
			{#if plugins.on("lint")}
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
		</div>
	{:else if panel.tab === "memory"}
		<div class="panel-body">
			{#if plugins.on("summary")}
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
			{#if !plugins.on("summary") && !plugins.on("memory")}
				<p class="faint">대화 요약·메모리 검색이 꺼져 있음 (설정 > 코어 플러그인)</p>
			{/if}
		</div>
	{:else if panel.tab === "context"}
		<div class="panel-body preset-edit">
			<section>
				<h4>컨텍스트</h4>
				<ContextMeter {usage} />
			</section>
			<section>
				<h4>프리셋 <span class="faint">{conv.promptPresetId ? "이 대화에서 고름" : "앱·캐릭터 기본값"}</span></h4>
				<Select value={lockable ? (shownPreset ?? DEFAULT) : (conv.promptPresetId ?? DEFAULT)} options={presetOptions} aria-label="이 대화의 프리셋" onchange={(e) => void pickPreset(e.currentTarget.value)} />
			</section>
			<PresetInline presetId={activePresetId} onsaved={loadUsage} model={conv.model || effectiveApi?.model || null} />
		</div>
	{:else}
		<div class="panel-body">
			<section>
				<h4>이 대화</h4>
				{#if lockable}
					<div class="row">
						<span>API</span>
						<div class="with-btn">
							<Select value={effectiveApi?.id ?? DEFAULT} options={effectiveApi ? apiOptions.slice(1) : apiOptions} aria-label="이 대화의 API" onchange={(e) => pickApi(e.currentTarget.value)} />
							<OptionLock locked={lockedApi} from="전역" disabled={!lockedApi && !effectiveApi} onclick={toggleApi} />
						</div>
					</div>
				{:else}
					<label class="row"><span>API</span><Select value={conv.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} /></label>
				{/if}
					<div class="row"><span>모델</span><ModelPicker profile={effectiveApi} value={lockable ? shownModel : conv.model} onchange={pickModel} label="이 대화의 모델" /></div>
				{#if reasoning}
					<label class="row"
						><span>추론 강도</span><span class="with-btn"
							><Select
								value={effort}
								options={[{ value: "auto", label: reasoning.default ? `auto (${reasoning.default})` : "auto" }, ...reasoning.levels.map((l) => ({ value: l, label: l }))]}
								aria-label="이 대화의 추론 강도"
								onchange={(e) => pickEffort(e.currentTarget.value)}
							/>{#if lockable}<OptionLock locked={lockedEffort} from="전역(auto)" onclick={toggleEffort} />{/if}</span
						></label
					>
				{/if}
				<div class="row">
					<span>프리셋</span>
					<div class="with-btn">
						{#if lockable}
							<Select value={shownPreset ?? DEFAULT} options={shownPreset ? presetOptions.slice(1) : presetOptions} aria-label="프리셋" onchange={(e) => void pickPreset(e.currentTarget.value)} />
							<OptionLock locked={lockedPreset} from={presetFrom} disabled={!lockedPreset && !shownPreset} onclick={togglePreset} />
						{:else}
							<Select value={conv.promptPresetId ?? DEFAULT} options={presetOptions} aria-label="프리셋" onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} />
						{/if}
						<IconButton label="이 프리셋 빠르게 편집{usage?.presetName ? ` (${usage.presetName})` : ''}" size="sm" onclick={() => (panel.tab = "context")}><Pencil /></IconButton>
					</div>
				</div>
				<div class="row">
					{#if me}
						<span>{me.label}</span>
						<ProfilePicker label="{me.label} 바꾸기" options={me.options} value={me.value} hint={me.hint} onchange={me.onchange} />
					{:else}
						<span>페르소나</span>
						<div class="with-btn">
							<ProfilePicker label="페르소나 변경" options={personaOptions} value={persona?.id} onchange={(id) => void pickPersona(id)} />
							{#if lockable}<OptionLock locked={lockedPersona} from={personaFrom} onclick={togglePersona} />{/if}
						</div>
					{/if}
				</div>
			</section>
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
				<h4>정규식 <span class="faint">{applied.filter((s) => s.enabled).length}개 적용 중</span></h4>
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
	.status-pin {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.status-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
	}
	.with-btn {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
		overflow: hidden;
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
	/* 읽기 화면에서 지금 읽는 메시지·챕터 (5-E-6-1) */
	.toc li.current > .chapter,
	.toc li.current > :global(.chapter) {
		color: var(--accent-text);
	}
	.toc li.current > button:not(.fold, .chapter),
	.toc li.current > :global(button:not(.fold, .chapter)) {
		color: var(--accent-text);
		background: var(--accent-soft);
	}
	.toc .chapter {
		display: flex;
		align-items: center;
	}
	.toc .chapter .fold {
		flex: 0 0 auto;
		width: 24px;
		height: 24px;
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
	.ver {
		padding: 0 4px;
		border-radius: var(--radius-sm);
		background: var(--surface-2);
		color: var(--text-muted);
		font-size: 0.85em;
	}
</style>
