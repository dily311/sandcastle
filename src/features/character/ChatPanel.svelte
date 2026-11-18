<script lang="ts">
	import { ArrowUpDown, GripVertical, Bookmark, Check, ChevronDown, Eye, EyeOff, FileArchive, ImagePlus, Pencil, Plus, Search, Trash2, X } from "@lucide/svelte";
	import { Avatar, Button, ContextMenu, IconButton, Input, ProfilePicker, Select, Switch, type MenuItem, type ProfileOption } from "$shared/ui";
	import { bookmarks, bookmarksOf } from "./bookmarks";
	import NotesPanel from "./NotesPanel.svelte";
	import BranchTree from "./BranchTree.svelte";
	import BookPicker from "$features/lorebook/BookPicker.svelte";
	import ContextMeter from "./ContextMeter.svelte";
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
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { convActions } from "./actions";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { preview } from "./format";
	import { library, type CharacterImage, type Conversation } from "./library.svelte";
	import { metaOf, type ChatSession } from "./session.svelte";
	import { lintOf } from "./lint";
	import MemoryIndex from "$features/memory/MemoryIndex.svelte";
	import MemorySearch from "$features/memory/MemorySearch.svelte";

	interface Props {
		conv: Conversation;
		session: ChatSession;
		chapters: Record<string, number>;
		onjump: (messageId: string) => void;
		/** 다른 대화 열기 (주면 그 메시지로) */
		onopen: (id: string, messageId?: string) => void;
		/** "나" 고르기를 바꿔 끼움 (폰: 폰 주인). 없으면 이 대화의 페르소나 */
		me?: { label: string; options: ProfileOption[]; value: string | null; hint?: string; onchange: (id: string) => void };
	}
	let { conv, session, chapters, onjump, onopen, me }: Props = $props();

	// ---- 메시지 linter: 이 대화의 덮어쓰기와 마지막 응답에서 걸린 규칙 (3-B-5) ----
	const memoryOff = $derived((conv.meta as { memory?: string } | null)?.memory === "off");
	const lintOff = $derived((conv.meta as { lint?: string } | null)?.lint === "off");
	const lastLint = $derived.by(() => {
		const last = [...session.messages].reverse().find((m) => m.role === "assistant");
		return last ? lintOf(last) : null;
	});

	// ---- 대화 설정: API → 프리셋 → 페르소나 ----
	const DEFAULT = "";
	const apiOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.apis.map((a) => ({ value: a.id, label: `${a.name} · ${a.model}` }))]);
	const presetOptions = $derived([{ value: DEFAULT, label: "앱 기본값" }, ...library.presets.map((p) => ({ value: p.id, label: p.name }))]);
	function setOption(p: { apiProfileId?: string; promptPresetId?: string; personaId?: string }) {
		void library.setOptions(conv.id, {
			personaId: p.personaId ?? conv.personaId,
			promptPresetId: (p.promptPresetId ?? conv.promptPresetId ?? "") || null,
			apiProfileId: (p.apiProfileId ?? conv.apiProfileId ?? "") || null,
		});
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
	function assetMenu(e: MouseEvent, a: CharacterImage) {
		e.preventDefault();
		menuItems = [
			{ label: a.enabled ? "AI에게 보내지 않기" : "AI에게 보내기", icon: a.enabled ? EyeOff : Eye, onselect: () => library.updateImage(a, { enabled: !a.enabled }) },
			{
				label: "키 이름 바꾸기",
				icon: Pencil,
				onselect: async () => {
					const key = await promptText({ title: "키 이름 바꾸기", message: "AI가 <cg>키</cg>로 부르는 이름", input: { value: a.key }, confirmLabel: "바꾸기" });
					if (key && key !== a.key) await library.updateImage(a, { key });
				},
			},
			{
				label: "삭제",
				icon: Trash2,
				danger: true,
				separator: true,
				onselect: async () => {
					if (await confirmAction({ title: "에셋 삭제", message: `'${a.key}' 이미지를 지움`, confirmLabel: "삭제", danger: true })) await library.removeImage(a);
				},
			},
		];
		menu = { x: e.clientX, y: e.clientY };
	}
	async function addImages() {
		await library.addImages(conv.characterId, await pickFiles([IMAGE_FILES]));
	}
	async function addZip() {
		const [zip] = await pickFiles([{ name: "ZIP", extensions: ["zip"] }], false);
		if (!zip) return;
		const n = await library.addImagesZip(conv.characterId, zip);
		toasts.success(n ? `에셋 ${n}개 넣음 (같은 이름은 덮어씀)` : "ZIP에 그림이 없음");
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
	let assetView = $state<CharacterImage | null>(null);
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
	// 에셋 탭이 열려 있으면 끌어다 놓은 그림은 캐릭터 에셋으로 (메시지 첨부보다 우선)
	$effect(() => {
		if (panel.tab !== "assets" || !plugins.on("chat.images")) return;
		const charId = conv.characterId;
		return drops.register({
			priority: 2,
			label: "캐릭터 에셋에 넣기",
			hint: "파일 이름이 이미지 키가 됨 (<cg>키</cg>). 그림이 든 ZIP도 가능",
			accept: (f) => isImage(f) || extOf(f) === "zip",
			ondrop: async (files) => {
				const paths = await Promise.all(files.filter(isImage).map((f) => storeFile(f)));
				if (paths.length) await library.addImages(charId, paths);
					// ZIP은 에셋 추가 메뉴의 "ZIP"과 같은 처리
					for (const z of files.filter((f) => extOf(f) === "zip")) {
						const n = await library.addImagesZip(charId, await storeFile(z));
						toasts.success(n ? `에셋 ${n}개 넣음 (같은 이름은 덮어씀)` : "ZIP에 그림이 없음");
					}
			},
		});
	});

	// ---- 북마크 ----
	const marks = $derived(bookmarksOf(conv));
	const msgNo = $derived(new Map(session.messages.map((m, i) => [m.id, i + 1])));

	// ---- 목차 ----
	let tocQuery = $state("");
	const toc = $derived.by(() => {
		// under: 이 메시지가 속한 챕터(그 챕터를 시작한 메시지 id)
		const out: { kind: "chapter" | "msg"; id: string; n: number; label: string; under?: string }[] = [];
		let cur: string | undefined;
		session.messages.forEach((m, i) => {
			const meta = metaOf(m);
			if (meta.chapter) {
				cur = m.id;
				out.push({ kind: "chapter", id: m.id, n: chapters[m.id], label: `${chapters[m.id]}장 · ${meta.chapter}` });
			}
			out.push({ kind: "msg", id: m.id, n: i + 1, label: meta.hidden ? "(숨긴 메시지)" : preview(m.content) || "(빈 메시지)", under: cur });
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
		return toc.filter((t) => t.label.toLowerCase().includes(q) || (t.kind === "msg" && t.n === num));
	});
</script>


	{#if panel.tab === "assets"}
		<div class="panel-body">
			{#if plugins.on("chat.images")}
				<Switch checked={conv.imagesEnabled} label="이 대화에서 캐릭터 에셋 사용" onchange={(v) => library.setImagesEnabled(conv.id, v)} />
			{:else}
				<p class="faint">캐릭터 에셋 기능이 꺼져 있음 (설정 > 코어 플러그인)</p>
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
						<button class="tile" class:off={!a.enabled} class:picked={assetPicked.has(a.id)} title={a.enabled ? a.key : `${a.key} (AI에게 안 보냄)`} onclick={() => (assetPicking ? pickAsset(a) : (assetView = a))} oncontextmenu={(e) => assetMenu(e, a)}>
							<span class="thumb">
								<img src={fileSrc(a.thumbnailPath ?? a.path)} alt="" loading="lazy" />
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
		{#if assetView}
			{@const v = assetView}
			<div class="aview" role="dialog" aria-modal="true" aria-label="에셋 크게 보기">
				<button class="ascrim" aria-label="닫기" onclick={() => (assetView = null)}></button>
				<figure>
					<img src={fileSrc(v.path)} alt={v.key} data-no-zoom />
					<figcaption><code>&lt;cg&gt;{v.key}&lt;/cg&gt;</code>{v.enabled ? "" : " · AI에게 안 보냄"}</figcaption>
				</figure>
			</div>
		{/if}
	{:else if panel.tab === "branches"}
		<BranchTree {conv} {onopen} />
	{:else if panel.tab === "bookmarks"}
			<ol class="marks">
				{#each [...marks].reverse() as b (b.id)}
					<li>
						<button class="mark" onclick={() => onjump(b.messageId)}>
							<span class="mtext">{b.text}</span>
							<span class="mmeta">{msgNo.has(b.messageId) ? `#${msgNo.get(b.messageId)}` : "지운 메시지"} · {new Date(b.createdAt).toLocaleDateString()}</span>
						</button>
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
					{/if}
					<button onclick={() => onjump(t.id)}><span class="no">{t.kind === "msg" ? `#${t.n}` : ""}</span>{t.label}</button>
				</li>
			{:else}
				<li class="none">{tocQuery ? "찾는 항목 없음" : "메시지 없음"}</li>
			{/each}
		</ol>
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
			<PresetInline presetId={activePresetId} onsaved={loadUsage} />
		</div>
	{:else}
		<div class="panel-body">
			<section>
				<h4>이 대화</h4>
				<label class="row"><span>API</span><Select value={conv.apiProfileId ?? DEFAULT} options={apiOptions} onchange={(e) => setOption({ apiProfileId: e.currentTarget.value })} /></label>
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
			{#if plugins.on("memory")}
				<hr />
				<section>
					<h4>메모리 검색 <span class="faint">지난 대화를 뜻으로 찾기</span></h4>
					<Switch checked={!memoryOff} label="이 대화를 메모리에 포함" description="끄면 색인과 전체 검색에서 이 대화를 뺌" onchange={(v) => library.patchMeta(conv.id, { memory: v ? null : "off" })} />
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
								<Switch checked={s.enabled} label={s.name} onchange={(v) => toggleScript(s, v)} />
								<IconButton label="{s.name} 편집" size="sm" onclick={() => regexEditor?.startEdit(s)}><Pencil /></IconButton>
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
	.aview {
		position: fixed;
		inset: 0;
		z-index: var(--z-viewer);
		display: grid;
		place-items: center;
		padding: var(--space-5);
	}
	.ascrim {
		position: absolute;
		inset: 0;
		border: none;
		background: rgb(0 0 0 / 0.6);
	}
	.aview figure {
		position: relative;
		margin: 0;
		padding: var(--space-3);
		border-radius: var(--radius-lg);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.aview img {
		display: block;
		max-width: 80vw;
		max-height: 80vh;
		object-fit: contain;
		border-radius: var(--radius-md);
	}
	.aview figcaption {
		padding-top: var(--space-2);
		font-size: var(--text-sm);
		color: var(--text-muted);
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
