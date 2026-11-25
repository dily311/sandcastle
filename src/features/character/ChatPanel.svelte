<script lang="ts">
	import { _ } from "svelte-i18n";
	import UtilMacroNote from "./UtilMacroNote.svelte";
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { PictureInPicture2, ArrowUpDown, ChevronRight, GripVertical, Bookmark, Check, ChevronDown, Eye, EyeOff, FileArchive, Folder, IdCard, ImagePlus, Pencil, Plus, Search, Trash2, UserCog, X, ChevronsDownUp, ChevronsUpDown } from "@lucide/svelte";
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
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import ExportButton from "./ExportButton.svelte";
	import PresetInline from "$features/settings/PresetInline.svelte";
	import RegexEditDialog from "$features/settings/RegexEditDialog.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import type { ContextUsage, ModelPriceInfo } from "$ipc/bindings";
	import { panel } from "$app/panel.svelte";
	import { nav } from "$app/nav.svelte";
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
	import ThemePanel from "./ThemePanel.svelte";

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

	// ---- 메시지 linter: 마지막 응답에서 걸린 규칙 (3-B-5) ----
	const memoText = $derived.by(() => {
		const m = (conv.meta as { memory?: string } | null)?.memory ?? "";
		return m === "off" ? "" : m;
	});
	const lastLint = $derived.by(() => {
		const last = [...session.messages].reverse().find((m) => m.role === "assistant");
		return last ? lintOf(last) : null;
	});

	// ---- 대화 설정: API → 프리셋 → 페르소나 ----
	const DEFAULT = "";
	const apiOptions = $derived([{ value: DEFAULT, label: $_("chatPanel.appDefault") }, ...library.apis.map((a) => ({ value: a.id, label: a.model ? `${a.name} · ${a.model}` : `${a.name} (${a.provider})` }))]);
	const presetOptions = $derived([{ value: DEFAULT, label: $_("chatPanel.appDefault") }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
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
	/** 지금 쓰는 모델: 모델이 잠긴 대화는 대화의 것, 풀린 대화는 전역 모델 (API와 독립 저장) */
	const shownModel = $derived(conv.model || appSetting.value?.selectedModel || null);
	/** 지금 쓰는 추론 강도: 모델과 함께 잠근 대화는 대화의 것, 풀린 대화는 전역 */
	const effort = $derived(
		(conv.model
			? ((conv.meta as { reasoningEffort?: string | null } | null)?.reasoningEffort ?? "auto")
			: (appSetting.value?.reasoningEffort ?? "auto")) as string,
	);
	/** 모델 고르기: 모델이 잠긴 대화는 이 대화에, 풀린 대화는 전역 모델. 폰 대화는 이 대화에 저장 */
	function pickModel(model: string | null) {
		if (lockable && !lockedModel) return void appSetting.update({ selectedModel: model });
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
	// 예상 비용용 모델 단가 (모델이 바뀔 때마다 조회)
	const activeModel = $derived(shownModel || effectiveApi?.model || null);
	let modelPrice = $state<ModelPriceInfo | null | undefined>(undefined);
	let priceSeq = 0;
	$effect(() => {
		const m = activeModel;
		const seq = ++priceSeq;
		if (!m) {
			modelPrice = undefined;
			return;
		}
		modelPrice = undefined;
		void call(commands.getModelPrice(m), { source: "chat", silent: true })
			.then((r) => {
				if (seq === priceSeq) modelPrice = r;
			})
			.catch(() => {
				if (seq === priceSeq) modelPrice = null;
			});
	});
	// 입력칸에 작성 중인 글의 추정 토큰 (ASCII ~4자/토큰, 한글·CJK ~1.5자/토큰)
	const draftTokens = $derived.by(() => {
		const t = draft?.get().trim() ?? "";
		if (!t) return 0;
		let ascii = 0;
		let nonAscii = 0;
		for (const ch of t) {
			if (ch.charCodeAt(0) < 128) ascii++;
			else nonAscii++;
		}
		return Math.ceil(ascii / 4 + nonAscii / 1.5);
	});

	// ---- 자물쇠 (5-E-29, ADR-028): 이 대화 > 캐릭터 기본 > 전역. 잠김 = 이 대화에 값이 있음 ----
	// 폰 대화도 일반 대화와 동일하게 자물쇠로 고정/해제 가능
	const lockable = true;
	const lockedApi = $derived(!!conv.apiProfileId);
	const lockedModel = $derived(!!conv.model);
	const lockedPreset = $derived(!!conv.promptPresetId);
	const lockedPersona = $derived(!!conv.personaId);
	const charPreset = $derived(character?.defaultPromptPresetId || null);
	/** 풀렸을 때 쓰는 프리셋: 캐릭터 기본 → 전역 (백엔드와 같은 순서) */
	const shownPreset = $derived(conv.promptPresetId || charPreset || appSetting.value?.selectedPreset || null);
	const presetFrom = $derived(charPreset ? $_("chatPanel.fromCharDefault") : $_("chatPanel.fromGlobal"));
	const personaFrom = $derived(character?.defaultPersonaId ? $_("chatPanel.fromCharDefault") : $_("chatPanel.fromGlobal"));
	/** API만 단독으로 이 대화에 고정/해제 */
	function toggleApi() {
		if (lockedApi) setOption({ apiProfileId: "" });
		else if (effectiveApi) setOption({ apiProfileId: effectiveApi.id });
	}
	/** 모델과 추론 강도를 함께 이 대화에 고정/해제 */
	function toggleModel() {
		if (lockedModel) {
			setOption({ model: null });
			void library.patchMeta(conv.id, { reasoningEffort: null });
		} else if (shownModel) {
			setOption({ model: shownModel });
			void library.patchMeta(conv.id, { reasoningEffort: effort });
		}
	}
	function togglePreset() {
		if (lockedPreset) setOption({ promptPresetId: "" });
		else if (shownPreset) setOption({ promptPresetId: shownPreset });
	}
	function togglePersona() {
		setOption({ personaId: lockedPersona ? "" : library.convPersonaId(conv) });
	}
	// ---- 고르기 (5-E-29-3, ADR-028 갱신): 잠근 줄은 이 대화에, 풀린 줄은 언제나 전역에 ----
	// 캐릭터 전용 값은 캐릭터 편집에서만 바꾼다. 풀린 줄이 캐릭터 전용 값을 쓰고 있으면 전역을 바꿔도 이 대화는 그대로임을 알린다
	function charStillWins(kind: string, name: string | undefined) {
		toasts.info($_("chatPanel.charStillWins", { values: { kind, name: name ?? "?" } }));
	}
	function pickApi(id: string) {
		if (lockable && !lockedApi) return void appSetting.update({ selectedApiProfile: id || null });
		setOption({ apiProfileId: id });
	}
	function pickEffort(v: string) {
		if (lockable && !lockedModel) return void appSetting.update({ reasoningEffort: v === "auto" ? null : v });
		void library.patchMeta(conv.id, { reasoningEffort: v });
	}
	async function pickPreset(id: string) {
		if (!lockable || lockedPreset) return setOption({ promptPresetId: id });
		await appSetting.update({ selectedPreset: id || "default" });
		if (charPreset && charPreset !== id) charStillWins($_("chatPanel.kindPreset"), library.presets.find((p) => p.id === charPreset)?.name);
	}
	async function pickPersona(id: string) {
		if (!lockable || lockedPersona) return setOption({ personaId: id });
		await library.setDefaultPersona(id);
		const own = character?.defaultPersonaId;
		if (own && own !== id) charStillWins($_("chatPanel.kindPersona"), library.persona(own)?.name);
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
	const scopeName = $derived({ global: $_("chatPanel.scopeGlobal"), character: $_("chatPanel.scopeChar"), preset: $_("chatPanel.scopePreset") } as const);
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
	async function removeScript(s: RegexScript) {
		if (!(await confirmAction({ title: $_("chatPanel.deleteRegexTitle"), message: $_("chatPanel.deleteRegexMsg", { values: { name: s.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.deleteRegexScript(s.id), { source: "regex" });
		regexVersion.bump();
		scripts = scripts.filter((x) => x.id !== s.id);
	}

	// ---- 에셋 ----
	const assets = $derived(library.images[conv.characterId] ?? []);
	// 판 폴더 (5-E-21): ZIP의 하위 폴더(v1/·v2/)가 판이 된다. 캐릭터마다 쓸 판을 고르면 그 판과 공통 그림만 대화에 쓴다
	const folders = $derived([...new Set(assets.map((a) => a.folder).filter(Boolean))].sort((a, b) => a.localeCompare(b, "ko")));
	const version = $derived(library.assetVersion[conv.characterId] ?? null);
	const inUse = (a: CharacterImage) => !version || !a.folder || a.folder.toLowerCase() === version.toLowerCase();
	async function moveAsset(a: CharacterImage) {
		const f = await promptText({ title: $_("chatPanel.folderPromptTitle"), message: $_("chatPanel.folderPromptMsg"), input: { value: a.folder, placeholder: $_("chatPanel.folderPlaceholder") }, confirmLabel: $_("chatPanel.moveConfirm") });
		if (f === null || f === undefined || f.trim() === a.folder) return;
		await library.updateImage(a, { folder: f.trim() });
	}
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	let menuItems: MenuItem[] = $state([]);
	async function renameAsset(a: CharacterImage) {
		const key = await promptText({ title: $_("chatPanel.renameKeyTitle"), message: $_("chatPanel.renameKeyMsg"), input: { value: a.key }, confirmLabel: $_("chatPanel.renameConfirm") });
		if (!key || key === a.key) return;
		await library.updateImage(a, { key });
		const now = assets.find((x) => x.id === a.id);
		if (now && imageViewer.current?.src === imageSrc(a)) imageViewer.patch({ name: now.key, info: assetInfo(now) });
	}
	async function deleteAsset(a: CharacterImage) {
		if (!(await confirmAction({ title: $_("chatPanel.deleteAssetTitle"), message: $_("chatPanel.deleteAssetMsg", { values: { key: a.key } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await library.removeImage(a);
		imageViewer.close();
	}
	const assetInfo = (a: CharacterImage) => [
		{ label: $_("chatPanel.infoKey"), value: `<cg>${a.key}</cg>` },
		{ label: $_("chatPanel.infoToAi"), value: a.enabled ? $_("chatPanel.infoSend") : $_("chatPanel.infoNoSend") },
		{ label: $_("chatPanel.infoVersion"), value: a.folder || $_("chatPanel.infoCommon") },
	];
	/** 에셋 크게 보기: 앱 공통 그림 뷰어에 이름 바꾸기·삭제를 이어 줌 */
	function openAsset(a: CharacterImage) {
		const src = imageSrc(a);
		if (src) imageViewer.open({ src, name: a.key, rename: () => renameAsset(a), remove: () => deleteAsset(a), info: assetInfo(a) });
	}
	function assetMenu(e: MouseEvent, a: CharacterImage) {
		e.preventDefault();
		menuItems = [
			{ label: a.enabled ? $_("chatPanel.hideFromAi") : $_("chatPanel.sendToAi"), icon: a.enabled ? EyeOff : Eye, onselect: () => library.updateImage(a, { enabled: !a.enabled }) },
			{ label: $_("chatPanel.renameKeyTitle"), icon: Pencil, onselect: () => renameAsset(a) },
			{ label: a.folder ? $_("chatPanel.moveVersion", { values: { folder: a.folder } }) : $_("chatPanel.setVersion"), icon: Folder, onselect: () => moveAsset(a) },
			{ label: $_("common.delete"), icon: Trash2, danger: true, separator: true, onselect: () => deleteAsset(a) },
		];
		menu = { x: e.clientX, y: e.clientY };
	}
	/** 에셋을 넣는 작업은 앱에 두어 이 화면을 나가도 이어진다 (패널엔 로딩으로, 나가면 토스트로 보임). 변환은 한 장씩이라 ZIP이 크면 오래 걸림 */
	const withBusy = assetImport.run.bind(assetImport);
	const zipDone = (n: number) => toasts.success(n ? $_("chatPanel.zipDone", { values: { count: n } }) : $_("chatPanel.zipEmpty"));
	async function addImages() {
		const paths = await pickFiles([IMAGE_FILES]);
		if (paths.length) await withBusy($_("chatPanel.addingImages", { values: { count: paths.length } }), () => library.addImages(conv.characterId, paths));
	}
	async function addZip() {
		const [zip] = await pickFiles([{ name: "ZIP", extensions: ["zip"] }], false);
		if (zip) await withBusy($_("chatPanel.convertingZip"), async () => zipDone(await library.addImagesZip(conv.characterId, zip)));
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
		if (!(await confirmAction({ title: $_("chatPanel.deleteAssetTitle"), message: $_("chatPanel.deleteAssetsMsg", { values: { count: list.length } }), confirmLabel: $_("chatPanel.deleteCountBtn", { values: { count: list.length } }), danger: true }))) return;
		for (const a of list) await library.removeImage(a);
		assetPicked = new Set();
		assetPicking = false;
	}
	function assetSortMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const opts = [
			{ v: "order", l: $_("chatPanel.sortOrder") },
			{ v: "new", l: $_("chatPanel.sortNew") },
			{ v: "name", l: $_("chatPanel.sortName") },
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
			label: $_("chatPanel.dropAssetLabel"),
			hint: $_("chatPanel.dropAssetHint"),
			accept: (f) => isImage(f) || extOf(f) === "zip",
			ondrop: (files) =>
				withBusy($_("chatPanel.movingFiles"), async (set) => {
					const paths = await Promise.all(files.filter(isImage).map((f) => storeFile(f)));
					if (paths.length) {
						set($_("chatPanel.addingImages", { values: { count: paths.length } }));
						await library.addImages(charId, paths);
					}
					// ZIP은 에셋 추가 메뉴의 "ZIP"과 같은 처리
					for (const z of files.filter((f) => extOf(f) === "zip")) {
						set($_("chatPanel.movingZip", { values: { name: z.name } }));
						const path = await storeFile(z);
						set($_("chatPanel.convertingZip"));
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
			label: $_("chatPanel.dropPresetLabel"),
			hint: $_("chatPanel.dropPresetHint"),
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				const paths = await Promise.all(files.map((f) => storeFile(f)));
				const id = await importPresetPaths(paths).catch(() => null);
				if (id) {
					setOption({ promptPresetId: id });
					toasts.info($_("chatPanel.presetAppliedToast"));
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
		return b.swipeId !== undefined && total > 1 ? $_("chatPanel.swipeLabel", { values: { cur: b.swipeId + 1, total } }) : "";
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
				out.push({ kind: "chapter", id: m.id, n: chapters[m.id], label: $_("chatPanel.chapterLabel", { values: { num: chapters[m.id], title: meta.chapter } }) });
			}
			out.push({ kind: "msg", id: m.id, n: i + 1, label: meta.hidden ? $_("chatPanel.hiddenMsg") : preview(m.content) || $_("chatPanel.emptyMsg"), under: cur, marks: marksOfMsg.get(m.id) });
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
			const t = await promptText({ title: cur ? $_("chatPanel.chapterRenameTitle") : $_("chatPanel.chapterAddTitle"), message: cur ? $_("chatPanel.chapterRenameMsg") : $_("chatPanel.chapterAddMsg", { values: { num: msgNo.get(id) ?? 0 } }), input: { value: cur ?? "", placeholder: $_("chatPanel.chapterPlaceholder") }, confirmLabel: cur ? $_("chatPanel.renameConfirm") : $_("chatPanel.createBtn") });
			if (t !== null && t !== undefined) await session.setChapter(m, t.trim() || null);
		};
		tocMenuItems = cur
			? [
					{ label: $_("chatPanel.chapterRenameTitle"), icon: Pencil, onselect: rename },
					{ label: $_("chatPanel.removeChapterMenu"), icon: X, danger: true, separator: true, onselect: () => session.setChapter(m, null) },
				]
			: [
					// 제목은 메시지 앞부분으로 바로 등록 (나중에 이름 바꾸기)
					{ label: $_("chatPanel.startChapterHere"), icon: Bookmark, onselect: () => session.setChapter(m, preview(m.content).slice(0, 20).trim() || `#${msgNo.get(id)}`) },
					{ label: $_("chatPanel.startChapterNamed"), icon: Pencil, onselect: rename },
				];
		tocMenuAt = { x: e.clientX, y: e.clientY };
	}
</script>


	{#if panel.tab === "theme"}
		<ThemePanel />
	{:else if panel.tab === "quickreply" && draft}
		<div class="panel-body">
			<QuickReplyList {session} {conv} getDraft={draft.get} setDraft={draft.set} />
		</div>
	{:else if panel.tab === "status" && plugins.on("status")}
		<div class="panel-body">
			<section class="status-pin">
				{#if !layout.narrow && statusView.floating}
					<div class="status-head">
						<span class="faint">{$_("chatPanel.statusFloatingHint")}</span>
						<Button size="sm" variant="ghost" onclick={() => statusView.set({ floating: false })}>{$_("chatPanel.statusToPanel")}</Button>
					</div>
					<Switch checked={statusView.separateWindows} label={$_("chatPanel.statusSeparateMulti")} onchange={(v) => statusView.set({ separateWindows: v })} />
				{:else}
					<div class="status-head">
						{#if layout.narrow}
							<Switch checked={statusView.aboveChat} label={$_("chatPanel.statusAboveChat")} onchange={(v) => statusView.set({ aboveChat: v })} />
						{:else}
							<Switch checked={statusView.separateWindows} label={$_("chatPanel.statusSeparate")} onchange={(v) => statusView.set({ separateWindows: v })} />
							<IconButton label={$_("chatPanel.statusFloatAria")} size="sm" onclick={() => statusView.set({ floating: true, collapsed: false })}><PictureInPicture2 /></IconButton>
						{/if}
					</div>
					<StatusWindow blocks={statusBlocks} />
				{/if}
			</section>
			{#if statusRegexes.length}
				<!-- 상태창 정규식 (5-F-5-3, 사용자 요청): 로어북 고르기처럼 고른 것만 칩으로, '+'로 더 넣기 -->
				<section class="status-regex">
					<h4>{$_("chatPanel.statusRegexHeading")}</h4>
					<ChipPicker
						label={$_("chatPanel.statusRegexHeading")}
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
				<UtilMacroNote presetId={shownPreset} macro="img_inprompt" />
			{:else}
				<p class="faint">{$_("chatPanel.assetsDisabledHint")}</p>
			{/if}
			{#if assetImport.busy}<p class="abusy" role="status"><Spinner />{assetImport.busy}</p>{/if}
			{#if folders.length}
				<label class="row"
					><span>{$_("chatPanel.assetVersionLabel")}</span><Select
						value={version ?? ""}
						options={[{ value: "", label: $_("chatPanel.allOption") }, ...folders.map((f) => ({ value: f, label: f }))]}
						aria-label={$_("chatPanel.assetVersionAria")}
						onchange={(e) => void library.setAssetVersion(conv.characterId, e.currentTarget.value || null)}
					/></label
				>
			{/if}
			<div class="atools">
				<span class="faint">{$_("chatPanel.assetsCountHint", { values: { count: assets.length } })}</span>
				<IconButton label={$_("chatPanel.sortAria")} size="sm" onclick={assetSortMenu}><ArrowUpDown /></IconButton>
				<IconButton label={$_("chatPanel.multiDeleteAria")} size="sm" active={assetPicking} disabled={!assets.length} onclick={() => ((assetPicking = !assetPicking), (assetPicked = new Set()))}><Trash2 /></IconButton>
				<IconButton label={$_("chatPanel.importImagesAria")} size="sm" onclick={addImages}><ImagePlus /></IconButton>
				<IconButton label={$_("chatPanel.importZipAria")} size="sm" onclick={addZip}><FileArchive /></IconButton>
			</div>
			{#if assetPicking}
				<div class="apick">
					<button class="link" onclick={() => (assetPicked = new Set(assets.map((a) => a.id)))}>{$_("chatPanel.selectAll")}</button>
					<span class="sp"></span>
					<Button size="sm" variant="danger" disabled={!assetPicked.size} onclick={removePickedAssets}>{#snippet icon()}<Trash2 />{/snippet}{$_("chatPanel.deleteCountBtn", { values: { count: assetPicked.size } })}</Button>
				</div>
			{/if}
			{#if assets.length}
				<div class="agrid">
					{#each sortedAssets as a (a.id)}
						<button class="tile" class:off={!a.enabled || !inUse(a)} class:picked={assetPicked.has(a.id)} title={a.enabled ? a.key : $_("chatPanel.assetHiddenTitle", { values: { key: a.key } })} onclick={() => (assetPicking ? pickAsset(a) : openAsset(a))} oncontextmenu={(e) => assetMenu(e, a)}>
							<span class="thumb">
								<img src={imageSrc(a, true)} alt="" loading="lazy" />
								{#if !a.enabled}<EyeOff />{/if}
							</span>
							<span class="key">{#if a.folder}<small class="ver">{a.folder}</small> {/if}{a.key}</span>
						</button>
					{/each}
				</div>
			{:else}
				<p class="none">{$_("chatPanel.noAssetsHint")}</p>
			{/if}
			<ContextMenu at={menu} items={menuItems} label={$_("chatPanel.assetMenuAria")} onclose={() => (menu = null)} />
		</div>
	{:else if panel.tab === "branches"}
		<BranchTree {conv} {onopen} />
	{:else if panel.tab === "bookmarks"}
			<ol class="marks">
				{#each [...marks].reverse() as b (b.id)}
					<li class:dim={!onShownSwipe(b)}>
						<button class="mark" onclick={() => onjump(b.messageId, b)} title={onShownSwipe(b) ? $_("chatPanel.jumpToTextTitle") : $_("chatPanel.jumpOtherSwipeTitle")} disabled={!msgNo.has(b.messageId)}>
							<span class="mtext">{b.text}</span>
							<span class="mmeta">{msgNo.has(b.messageId) ? `#${msgNo.get(b.messageId)}` : $_("chatPanel.deletedMsg")}{swipeLabel(b) ? ` · ${swipeLabel(b)}` : ""} · {new Date(b.createdAt).toLocaleDateString()}</span>
						</button>
						{#if oncard}<IconButton label={$_("chatPanel.makeCardAria")} size="sm" onclick={() => oncard(b)}><IdCard /></IconButton>{/if}
						<IconButton label={$_("chatPanel.removeMarkAria")} size="sm" onclick={() => bookmarks.remove(conv, b.id)}><X /></IconButton>
					</li>
				{:else}
					<li class="none"><Bookmark /><br />{$_("chatPanel.noMarksHint")}</li>
				{/each}
			</ol>
	{:else if panel.tab === "toc"}
		<div class="toc-search">
			<Input bind:value={tocQuery} placeholder={$_("chatPanel.tocSearchPlaceholder")} aria-label={$_("chatPanel.tocSearchAria")}>
				{#snippet leading()}<Search />{/snippet}
			</Input>
			{#if hasChapters}
				{#if folded.size}
					<IconButton label={$_("chatPanel.expandAllAria")} size="sm" onclick={() => (folded = new Set())}><ChevronsUpDown /></IconButton>
				{:else}
					<IconButton label={$_("chatPanel.collapseAllAria")} size="sm" onclick={() => (folded = new Set(toc.filter((t) => t.kind === "chapter").map((t) => t.id)))}><ChevronsDownUp /></IconButton>
				{/if}
			{/if}
		</div>
		<ol class="toc" bind:this={tocList}>
			{#each tocShown as t (t.kind + t.id)}
				{@const cur = t.kind === "msg" ? t.id === reading : t.id === readingChapter}
				<li class={t.kind} class:current={cur} aria-current={cur ? "location" : undefined}>
					{#if t.kind === "chapter"}
						<button class="fold" aria-label={folded.has(t.id) ? $_("chatPanel.expandChapterAria") : $_("chatPanel.collapseChapterAria")} aria-expanded={!folded.has(t.id)} onclick={() => toggleChapter(t.id)}><ChevronDown class={folded.has(t.id) ? "rot" : ""} /></button>
						<button class="chapter" onclick={() => onjump(t.id)} oncontextmenu={(e) => tocContext(e, t.id)}>{t.label}</button>
					{:else}
						<button onclick={() => onjump(t.id)} oncontextmenu={(e) => tocContext(e, t.id)}><span class="no">#{t.n}</span>{t.label}</button>
					{/if}
					{#if t.marks?.length}
						<ul class="tbm">
							{#each t.marks as b (b.id)}
								<li class:dim={!onShownSwipe(b)}><button onclick={() => onjump(t.id, b)} title={$_("chatPanel.jumpToTextTitle")}><Bookmark />{b.text}</button></li>
							{/each}
						</ul>
					{/if}
				</li>
			{:else}
				<li class="none">{tocQuery ? $_("chatPanel.tocNoMatch") : $_("chatPanel.tocEmpty")}</li>
			{/each}
		</ol>
		<ContextMenu at={tocMenuAt} items={tocMenuItems} label={$_("chatPanel.tocMenuAria")} onclose={() => (tocMenuAt = null)} />
	{:else if panel.tab === "lint"}
		<div class="panel-body">
			{#if plugins.on("lint")}
				<section>
					<h4>{$_("chatPanel.lintHeading")} <span class="faint">{$_("chatPanel.lintSub")}</span></h4>
					{#if lastLint}
						<p class="faint">{$_("chatPanel.lintLastHits")}</p>
						<ul class="lint-hits">
							{#each lastLint.findings as f, i (i)}
								<li class:warn={f.level === "warn"}>{f.text}{f.fixed ? $_("chatPanel.lintFixed") : ""}</li>
							{/each}
						</ul>
					{:else}
						<p class="faint">{$_("chatPanel.lintNoHits")}</p>
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
					<h4>{$_("chatPanel.memorySearchHeading")}</h4>
					<div class="mem-actions"><MemoryIndex conversationId={conv.id} /></div>
					<MemorySearch conversationId={conv.id} onopen={(h) => onopen(conv.id, h.firstMessageId)} />
				</section>
			{/if}
			{#if !plugins.on("summary") && !plugins.on("memory")}
				<p class="faint">{$_("chatPanel.memoryDisabledHint")}</p>
			{/if}
		</div>
	{:else if panel.tab === "context"}
		<div class="panel-body preset-edit">
			<section>
				<h4>{$_("chatPanel.contextHeading")}</h4>
				<ContextMeter {usage} model={activeModel} price={modelPrice} {draftTokens} />
			</section>
			<section>
				<h4>{$_("chatPanel.presetHeading")} <span class="faint">{conv.promptPresetId ? $_("chatPanel.presetConvPicked") : $_("chatPanel.presetDefaultPicked")}</span></h4>
				<Select value={lockable ? (shownPreset ?? DEFAULT) : (conv.promptPresetId ?? DEFAULT)} options={presetOptions} aria-label={$_("chatPanel.presetConvAria")} onchange={(e) => void pickPreset(e.currentTarget.value)} />
			</section>
			<PresetInline presetId={activePresetId} onsaved={loadUsage} model={conv.model || effectiveApi?.model || null} />
		</div>
	{:else}
		<div class="panel-body">
			<section>
				<h4>{$_("chatPanel.thisConvHeading")}</h4>
				{#if lockable}
					<div class="row">
						<span>API</span>
						<div class="with-btn">
							<Select value={effectiveApi?.id ?? DEFAULT} options={effectiveApi ? apiOptions.slice(1) : apiOptions} aria-label={$_("chatPanel.apiConvAria")} onchange={(e) => pickApi(e.currentTarget.value)} />
							<IconButton label={$_("chatPanel.openApiSettingsAria")} size="sm" onclick={() => nav.go({ page: "settings", tab: "api" })}><Plus /></IconButton>
							<OptionLock locked={lockedApi} from={$_("chatPanel.fromGlobal")} disabled={!lockedApi && !effectiveApi} onclick={toggleApi} />
						</div>
					</div>
				{:else}
					<div class="row">
						<span>API</span>
						<div class="with-btn">
							<Select value={conv.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} />
							<IconButton label={$_("chatPanel.openApiSettingsAria")} size="sm" onclick={() => nav.go({ page: "settings", tab: "api" })}><Plus /></IconButton>
						</div>
					</div>
				{/if}
				<div class="row">
					<span>{$_("chatPanel.modelLabel")}</span>
					<div class="with-btn">
						<ModelPicker profile={effectiveApi} value={lockable ? shownModel : conv.model} onchange={pickModel} {effort} oneffort={pickEffort} label={$_("chatPanel.modelConvAria")} />
						{#if lockable}<OptionLock locked={lockedModel} from={$_("chatPanel.fromGlobal")} disabled={!lockedModel && !shownModel} onclick={toggleModel} />{/if}
					</div>
				</div>
				<div class="row">
					<span>{$_("chatPanel.presetHeading")}</span>
					<div class="with-btn">
						{#if lockable}
							<Select value={shownPreset ?? DEFAULT} options={shownPreset ? presetOptions.slice(1) : presetOptions} aria-label={$_("chatPanel.presetHeading")} onchange={(e) => void pickPreset(e.currentTarget.value)} />
							<IconButton label={`${$_("chatPanel.quickEditPresetAria")}${usage?.presetName ? ` (${usage.presetName})` : ""}`} size="sm" onclick={() => (panel.tab = "context")}><Pencil /></IconButton>
							<OptionLock locked={lockedPreset} from={presetFrom} disabled={!lockedPreset && !shownPreset} onclick={togglePreset} />
						{:else}
							<Select value={conv.promptPresetId ?? DEFAULT} options={presetOptions} aria-label={$_("chatPanel.presetHeading")} onchange={(e) => setOption({ promptPresetId: e.currentTarget.value })} />
							<IconButton label={`${$_("chatPanel.quickEditPresetAria")}${usage?.presetName ? ` (${usage.presetName})` : ""}`} size="sm" onclick={() => (panel.tab = "context")}><Pencil /></IconButton>
						{/if}
					</div>
				</div>
				<div class="row">
					{#if me}
						<span>{me.label}</span>
						<ProfilePicker label={$_("chatPanel.changeMeAria", { values: { label: me.label } })} options={me.options} value={me.value} hint={me.hint} onchange={me.onchange} />
					{:else}
						<span>{$_("chatPanel.personaLabel")}</span>
						<div class="with-btn">
							<ProfilePicker label={$_("chatPanel.changePersonaAria")} options={personaOptions} value={persona?.id} onchange={(id) => void pickPersona(id)} onmanage={() => nav.go({ page: "persona" })} manageLabel={$_("chatPanel.managePersonaLabel")} />
							<IconButton label={$_("chatPanel.managePersonaLabel")} size="sm" onclick={() => nav.go({ page: "persona" })}><UserCog /></IconButton>
							{#if lockable}<OptionLock locked={lockedPersona} from={personaFrom} onclick={togglePersona} />{/if}
						</div>
					{/if}
				</div>
			</section>
			<hr />
			<section>
				<h4>{$_("chatPanel.wiHeading")}</h4>
				<div class="wi">
					<span>{character?.name ?? $_("chatPanel.charFallback")} <small>{$_("chatPanel.wiCharHint")}</small></span>
					<BookPicker label={$_("chatPanel.wiCharAria")} value={character?.worldInfoIds ?? []} onchange={(ids) => library.setCharacterWorldInfo(conv.characterId, ids)} />
				</div>
				<div class="wi">
					<span>{$_("chatPanel.wiConvLabel")} <small>{$_("chatPanel.wiConvHint")}</small></span>
					<BookPicker label={$_("chatPanel.wiConvAria")} value={conv.worldInfoIds} onchange={(ids) => library.setWorldInfo(conv.id, ids)} />
				</div>
			</section>
			<hr />
			<NotesPanel {conv} bare />
			<hr />
			<section>
				<h4>{$_("chatPanel.regexHeading")} <span class="faint">{$_("chatPanel.regexAppliedCount", { values: { count: applied.filter((s) => s.enabled).length } })}</span></h4>
				{#each groups as g (g.scope)}
					<details class="rx-group" open>
						<summary class="rx-scope">
							<ChevronRight class="caret" />
							<b>{scopeName[g.scope]}</b>
							<small>{g.list.filter((s) => s.enabled).length}/{g.list.length}</small>
						</summary>
						<ul class="rx">
							{#each g.list as s, i (s.id)}
								{@const sort = sorters[g.scope]}
								<li {...sort.item(i)} class:dragging={sort.from === i} class:over={sort.over === i && sort.from !== i}>
									<span class="grip" aria-hidden="true" {...sort.handle(i)}><GripVertical /></span>
									<Switch checked={s.enabled} label={s.name} onchange={(v) => toggleScript(s, v)} />
									<div class="rx-acts">
										<IconButton label={$_("chatPanel.editItemAria", { values: { name: s.name } })} size="sm" onclick={() => regexEditor?.startEdit(s)}><Pencil /></IconButton>
										<IconButton label={$_("chatPanel.deleteItemAria", { values: { name: s.name } })} size="sm" variant="danger" onclick={() => removeScript(s)}><Trash2 /></IconButton>
									</div>
								</li>
							{/each}
						</ul>
					</details>
				{:else}
					<p class="none small">{$_("chatPanel.noRegexHint")}</p>
				{/each}
				<Button size="sm" variant="ghost" onclick={() => regexEditor?.startNew({ scope: "character", scopeId: conv.characterId })}>{#snippet icon()}<Plus />{/snippet}{$_("chatPanel.addCharRegexBtn")}</Button>
				<RegexEditDialog bind:this={regexEditor} sortOrder={applied.length * 10} onsaved={onRegexSaved} />
			</section>
			<hr />
			<section>
				<h4>{$_("chatPanel.exportHeading")}</h4>
				<ExportButton {conv} />
			</section>
			<hr />
			<section>
				<Button block variant="danger" onclick={() => convActions.remove(conv)}>
					{#snippet icon()}<Trash2 />{/snippet}
					{$_("chatPanel.deleteConvBtn")}
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
	.rx-group {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		overflow: hidden;
	}
	.rx-scope {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 6px 8px;
		font-size: var(--text-xs);
		color: var(--text);
		background: var(--surface-2);
		cursor: pointer;
		list-style: none;
		user-select: none;
	}
	.rx-scope::-webkit-details-marker {
		display: none;
	}
	.rx-scope:hover {
		background: var(--surface-hover);
	}
	.rx-scope :global(.caret) {
		flex: none;
		width: 14px;
		height: 14px;
		color: var(--text-muted);
		transition: transform var(--duration-fast) var(--ease);
	}
	.rx-group[open] > .rx-scope :global(.caret) {
		transform: rotate(90deg);
	}
	.rx-scope b {
		flex: 1;
		font-size: var(--text-xs);
	}
	.rx-scope small {
		color: var(--text-faint);
		font-size: var(--text-xs);
		font-variant-numeric: tabular-nums;
	}
	.rx {
		list-style: none;
		margin: 0;
		padding: 4px;
		border-top: 1px solid var(--border);
	}
	.rx li {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 4px 2px 2px;
		border-radius: var(--radius-sm);
		cursor: grab;
	}
	.rx li:hover {
		background: var(--surface-hover);
	}
	.rx li :global(.switch-row),
	.rx li > :global(.switch) {
		flex: 1;
		min-width: 0;
	}
	.rx-acts {
		flex: none;
		display: inline-flex;
		align-items: center;
		gap: 1px;
	}
	.rx-acts :global(.icon-btn) {
		width: 24px;
		height: 24px;
	}
	.rx-acts :global(.icon-btn svg) {
		width: 13px;
		height: 13px;
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
