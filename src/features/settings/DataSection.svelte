<script lang="ts">
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
	import { ChevronDown, ChevronRight, Link2, RotateCcw, Save, ShieldCheck, Trash2, Upload } from "@lucide/svelte";
	import type { BackupManifest, DataCheck, DataOverview_Serialize as Overview, DataTotals, SillyTavernDataImport } from "$ipc/bindings";
	import { Badge, Button, Select, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo, BACKUP_FILES, ST_DATA_FILES } from "$shared/ipc/dialog";
	import { dropPaths, extOf } from "$app/drop.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import PluginVersionsGroup from "./PluginVersionsGroup.svelte";
	import TrashGroup from "./TrashGroup.svelte";

	const src = { source: "data" };
	let data = $state<Overview | null>(null);
	let pending = $state<BackupManifest | null>(null);
	let busy = $state<"backup" | "restore" | "st" | "st-out" | null>(null);
	/** SillyTavern 데이터 가져오기 결과 (5-E-20-4-1) */
	let stResult = $state<SillyTavernDataImport | null>(null);
	async function exportSt() {
		try {
			const r = await saveTo("sillytavern-default-user.zip", [ST_DATA_FILES], (path) => {
				busy = "st-out";
				return jobs.run("SillyTavern 데이터", () => call(commands.exportSillytavernData(path), src), { group: "내보내기" });
			});
			if (r) toasts.success(`내보냄 · 캐릭터 ${r.characters}, 채팅 ${r.chats}, 로어북 ${r.books}, 프리셋 ${r.presets}, 페르소나 ${r.personas}, 정규식 ${r.regex}`);
			if (r?.warnings.length) toasts.error(`넣지 못한 것 ${r.warnings.length}개`, r.warnings.join("\n"));
		} finally {
			busy = null;
		}
	}
	async function importSt() {
		const [path] = await pickFiles([ST_DATA_FILES], false);
		if (!path) return;
		busy = "st";
		try {
			const r = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.importSillytavernData(path), src), { group: "SillyTavern 가져오기" });
			if (!r) return;
			stResult = r;
			toasts.success(`가져옴 · 캐릭터 ${r.characters}, 채팅 ${r.chats}, 로어북 ${r.books}, 프리셋 ${r.presets}, 정규식 ${r.regex}, 페르소나 ${r.personas}`);
			await Promise.all([load(), library.load()]);
		} finally {
			busy = null;
		}
	}
	let openChar = $state<string | null>(null);
	let picked = $state<Set<string>>(new Set());
	let orphanPicked = $state<Set<string>>(new Set());
	let orphanTargetChar = $state("");
	let trashKey = $state(0);

	// ---- 점검 (1-B-3): 보여주기만, 지우지 않음 ----
	let check = $state<DataCheck | null>(null);
	let checking = $state(false);
	let showOrphans = $state(false);
	async function runCheck() {
		checking = true;
		try {
			check = await call(commands.checkData(), src);
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			checking = false;
		}
	}
	const kb = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`);

	onMount(load);
	async function load() {
		const [o, p] = await Promise.all([
			call(commands.getDataOverview(), src).catch(() => null),
			call(commands.getPendingRestore(), { ...src, silent: true }).catch(() => null),
		]);
		data = o;
		pending = p;
		picked = new Set();
		orphanPicked = new Set();
		if (!orphanTargetChar && o?.characters.length) orphanTargetChar = o.characters[0].id;
	}

	function bytes(n: number) {
		if (n < 1024) return `${n} B`;
		const u = ["KB", "MB", "GB"];
		let v = n / 1024;
		let i = 0;
		while (v >= 1024 && i < u.length - 1) {
			v /= 1024;
			i++;
		}
		return `${v.toFixed(v < 10 ? 1 : 0)} ${u[i]}`;
	}
	const COUNTS: [keyof DataTotals, string][] = [
		["characters", "캐릭터"],
		["conversations", "대화"],
		["messages", "메시지"],
		["personas", "페르소나"],
		["characterImages", "이미지"],
		["worldInfoBooks", "월드인포"],
		["promptPresets", "프리셋"],
		["regexScripts", "정규식"],
		["games", "게임"],
	];
	const parts = $derived(
		data
			? [
					{ label: "DB", value: data.storage.databaseBytes, color: "var(--accent)" },
					{ label: "캐릭터 파일", value: data.storage.charactersBytes, color: "var(--info, #3f74c9)" },
					{ label: "게임", value: data.storage.gamesBytes, color: "var(--warning, #a87a2f)" },
					{ label: "테마", value: data.storage.themesBytes, color: "var(--text-faint)" },
				]
			: [],
	);

	// ---- 백업 / 복원 ----
	async function backup() {
		const d = new Date();
		const stamp = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}`;
		try {
			const r = await saveTo(`sandcastle-${stamp}.scbackup`, [BACKUP_FILES], (path) => {
				busy = "backup";
				return jobs.run("백업", () => call(commands.exportBackup(path), src), { group: "백업" });
			});
			if (r) toasts.success(`백업함 · ${bytes(r.bytes)}, 파일 ${r.fileCount}개`);
		} finally {
			busy = null;
		}
	}
	async function restore() {
		const [path] = await pickFiles([BACKUP_FILES], false);
		if (path) await restoreFrom(path);
	}
	// 끌어다 놓아도 같은 확인을 거침 (놓은 파일은 통째로 읽혀 옮겨지므로 아주 큰 백업은 버튼으로)
	$effect(() =>
		dropPaths({
			label: "백업에서 복원",
			hint: "샌드캐슬 백업(.scbackup), 300MB까지. 더 큰 백업은 복원 버튼으로",
			accept: (f) => extOf(f) === "scbackup",
			multiple: false,
			maxBytes: 300 * 1024 * 1024,
			onpaths: ([path]) => restoreFrom(path),
		}),
	);
	async function restoreFrom(path: string) {
		busy = "restore";
		try {
			const p = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.prepareRestore(path), src), { group: "복원 준비" });
			if (!p) return;
			const t = p.totals;
			const ok = await confirmAction({
				title: "백업으로 복원",
				message: `${absolute(p.createdAt)}에 만든 백업 (v${p.appVersion}) · 캐릭터 ${t.characters}, 대화 ${t.conversations}, 메시지 ${t.messages}. 지금 데이터는 지우지 않고 따로 옮겨 둠. 앱을 다시 시작하면 바뀜${p.willMigrate ? ". 예전 버전 백업이라 시작할 때 변환함" : ""}`,
				confirmLabel: "다시 시작해서 복원",
			});
			if (ok) await call(commands.restartApp(), src);
			else await call(commands.cancelRestore(), src);
			pending = ok ? null : await call(commands.getPendingRestore(), { ...src, silent: true }).catch(() => null);
		} finally {
			busy = null;
		}
	}
	async function cancelPending() {
		await call(commands.cancelRestore(), src);
		pending = null;
		toasts.success("복원 취소함");
	}

	// ---- 대화 정리 ----
	function pick(id: string, on: boolean) {
		const next = new Set(picked);
		if (on) next.add(id);
		else next.delete(id);
		picked = next;
	}
	async function removePicked() {
		const n = picked.size;
		if (!(await confirmAction({ title: "대화 삭제", message: `고른 대화 ${n}개와 메시지를 휴지통으로 옮김. 아래 휴지통에서 되살릴 수 있음`, confirmLabel: `${n}개 삭제`, danger: true }))) return;
		const ids = [...picked];
		const removed = await jobs.run(`대화 ${n}개`, () => call(commands.deleteConversations(ids), src), { group: "대화 지우기" });
		if (removed !== undefined) toasts.success(`대화 ${removed}개를 휴지통으로 옮김`);
		trashKey++; // 휴지통 목록 다시 읽기
		await Promise.all([load(), library.load()]);
	}
	const convTitle = (c: { title: string | null; mode: string | null }) => c.title || (c.mode === "phone" ? "폰 대화" : "제목 없음");

	// ---- 고아 채팅 관리 ----
	const charOptions = $derived(
		(data?.characters ?? []).map((c) => ({ value: c.id, label: c.name })),
	);
	function pickOrphan(id: string, on: boolean) {
		const next = new Set(orphanPicked);
		if (on) next.add(id);
		else next.delete(id);
		orphanPicked = next;
	}
	function toggleAllOrphans() {
		if (!data) return;
		if (orphanPicked.size === data.orphanConversations.length) orphanPicked = new Set();
		else orphanPicked = new Set(data.orphanConversations.map((c) => c.id));
	}
	async function reassignOrphans(ids: string[]) {
		if (!ids.length || !orphanTargetChar) return;
		const charName = data?.characters.find((c) => c.id === orphanTargetChar)?.name ?? "캐릭터";
		const n = await call(commands.reassignConversationsCharacter(ids, orphanTargetChar), src);
		toasts.success(`고아 채팅 ${n}개를 '${charName}'에 연결함`);
		await Promise.all([load(), library.load()]);
	}
	async function removeOrphans(ids: string[]) {
		if (!ids.length) return;
		if (!(await confirmAction({ title: "고아 채팅 삭제", message: `캐릭터 없는 대화 ${ids.length}개를 휴지통으로 옮김. 아래 휴지통에서 되살릴 수 있음`, confirmLabel: `${ids.length}개 삭제`, danger: true }))) return;
		const removed = await jobs.run(`고아 채팅 ${ids.length}개`, () => call(commands.deleteConversations(ids), src), { group: "대화 지우기" });
		if (removed !== undefined) toasts.success(`고아 채팅 ${removed}개를 휴지통으로 옮김`);
		trashKey++;
		await Promise.all([load(), library.load()]);
	}
</script>

<Section title="데이터" lead="저장 공간, 백업과 복원, 휴지통, 대화 정리">
	{#if !data}
		<div class="loading"><Spinner /></div>
	{:else}
		{#if pending}
			<div class="banner">
				<RotateCcw />
				<div>
					<b>복원 준비됨</b>
					<span>{absolute(pending.createdAt)} 백업. 앱을 다시 시작하면 복원함</span>
				</div>
				<Button size="sm" variant="ghost" onclick={cancelPending}>취소</Button>
				<Button size="sm" variant="primary" onclick={() => call(commands.restartApp(), src)}>다시 시작</Button>
			</div>
		{/if}

		<Group>
			<div class="usage">
				<div class="total"><b>{bytes(data.storage.totalBytes)}</b><span>사용 중</span></div>
				<div class="bar" role="img" aria-label="저장 공간 나눔">
					{#each parts as p (p.label)}
						{#if p.value > 0}<span style:flex-grow={p.value} style:background={p.color} title="{p.label} {bytes(p.value)}"></span>{/if}
					{/each}
				</div>
				<ul class="legend">
					{#each parts as p (p.label)}
						<li><i style:background={p.color}></i>{p.label} <span>{bytes(p.value)}</span></li>
					{/each}
				</ul>
			</div>
			<dl class="counts">
				{#each COUNTS as [k, label] (k)}
					<div><dt>{label}</dt><dd>{data.totals[k].toLocaleString()}</dd></div>
				{/each}
			</dl>
		</Group>

		<Group title="백업">
			<Row label="백업 파일 만들기" desc="캐릭터·대화·설정·이미지·갤러리·배경·게임을 파일 하나로. API 키는 빠짐">
				<Button size="sm" loading={busy === "backup"} disabled={busy !== null} onclick={backup}>{#snippet icon()}<Save />{/snippet}백업</Button>
			</Row>
			<Row label="백업에서 복원" desc="지금 데이터는 지우지 않고 따로 옮겨 둔 뒤 바꿈. 앱이 다시 시작됨">
				<Button size="sm" loading={busy === "restore"} disabled={busy !== null} onclick={restore}>{#snippet icon()}<Upload />{/snippet}복원</Button>
			</Row>
		</Group>

		<Group title="SillyTavern">
			<Row label="SillyTavern 데이터 가져오기" desc="data/default-user 폴더를 압축한 zip(10GB까지). 새로 만들기만 하므로 다시 가져오면 한 벌 더 생김">
				<Button size="sm" loading={busy === "st"} disabled={busy !== null} onclick={importSt}>{#snippet icon()}<Upload />{/snippet}가져오기</Button>
			</Row>
			<Row label="SillyTavern 모양으로 내보내기" desc="캐릭터 카드·채팅·로어북·프리셋·페르소나·정규식. 게임·폰 기능은 빠짐">
				<Button size="sm" loading={busy === "st-out"} disabled={busy !== null} onclick={exportSt}>{#snippet icon()}<Save />{/snippet}내보내기</Button>
			</Row>
			{#if stResult?.warnings.length}
				<details class="st-warn">
					<summary>가져오지 못한 것 {stResult.warnings.length}개</summary>
					<ul>{#each stResult.warnings as w, i (i)}<li>{w}</li>{/each}</ul>
				</details>
			{/if}
		</Group>

		<Group title="점검">
			<Row label="DB 점검" desc="DB가 손상되지 않았는지, 캐릭터 폴더에 아무 데도 쓰이지 않는 파일이 있는지 봄. 아무것도 지우지 않음">
				<Button size="sm" loading={checking} disabled={checking} onclick={runCheck}>{#snippet icon()}<ShieldCheck />{/snippet}점검</Button>
			</Row>
			{#if check}
				<div class="check" role="status">
					{#if check.ok}
						<p><Badge tone="success">DB 이상 없음</Badge></p>
					{:else}
						<p><Badge tone="danger">DB 문제 발견</Badge> 먼저 백업 파일을 만들어 두고, 문제가 계속되면 백업에서 복원해 볼 것</p>
						<ul class="problems">
							{#each check.problems as p, i (i)}<li><code>{p}</code></li>{/each}
						</ul>
					{/if}
					<p>
						파일 {check.checkedFiles.toLocaleString()}개를 살펴봄 ·
						{#if check.orphanCount}
							쓰이지 않는 파일 {check.orphanCount.toLocaleString()}개 ({kb(check.orphanBytes)})
							<Button size="sm" variant="ghost" aria-expanded={showOrphans} onclick={() => (showOrphans = !showOrphans)}>{showOrphans ? "목록 접기" : "목록 보기"}</Button>
						{:else}
							쓰이지 않는 파일 없음
						{/if}
					</p>
					{#if showOrphans && check.orphans.length}
						<ul class="orphans">
							{#each check.orphans as o (o.path)}<li><code>{o.path}</code><span class="meta">{kb(o.bytes)}</span></li>{/each}
						</ul>
						{#if check.orphanCount > check.orphans.length}<p class="meta">큰 것부터 {check.orphans.length}개만 보임</p>{/if}
					{/if}
				</div>
			{/if}
		</Group>

		<PluginVersionsGroup />

		{#key trashKey}<TrashGroup onrestored={() => Promise.all([load(), library.load()])} />{/key}

		<Group title="고아 채팅 ({data.orphanConversations.length})">
			{#snippet aside()}
				{#if data?.orphanConversations.length}
					<div class="orphan-bar">
						<Button size="sm" variant="ghost" onclick={toggleAllOrphans}>
							{orphanPicked.size === (data?.orphanConversations.length ?? 0) ? "전체 해제" : "전체 선택"}
						</Button>
						{#if orphanPicked.size && charOptions.length}
							<Select bind:value={orphanTargetChar} options={charOptions} aria-label="연결할 캐릭터" />
							<Button size="sm" variant="secondary" onclick={() => reassignOrphans([...orphanPicked])}>{#snippet icon()}<Link2 />{/snippet}연결 ({orphanPicked.size})</Button>
						{/if}
						{#if orphanPicked.size}
							<Button size="sm" variant="danger" onclick={() => removeOrphans([...orphanPicked])}>{#snippet icon()}<Trash2 />{/snippet}삭제 ({orphanPicked.size})</Button>
						{/if}
					</div>
				{/if}
			{/snippet}
			{#if data.orphanConversations.length}
				<p class="orphan-desc">캐릭터가 삭제되어 연결이 끊어진 대화. 다른 캐릭터에 다시 연결하거나 휴지통으로 옮길 수 있음</p>
				<ul class="convs orphan-list">
					{#each data.orphanConversations as v (v.id)}
						<li class="orphan-row">
							<label>
								<input type="checkbox" checked={orphanPicked.has(v.id)} onchange={(e) => pickOrphan(v.id, e.currentTarget.checked)} />
								<span class="t">{convTitle(v)}</span>
								{#if v.mode === "phone"}<Badge>폰</Badge>{/if}
								<span class="meta">메시지 {v.messageCount}</span>
								<span class="when">{v.lastMessageAt ? relative(v.lastMessageAt) : ""}</span>
							</label>
							<div class="orphan-acts">
								{#if charOptions.length}
									<Button size="sm" variant="ghost" title="고른 캐릭터에 연결" onclick={() => reassignOrphans([v.id])}>{#snippet icon()}<Link2 />{/snippet}연결</Button>
								{/if}
								<Button size="sm" variant="ghost" aria-label="삭제" onclick={() => removeOrphans([v.id])}>{#snippet icon()}<Trash2 />{/snippet}</Button>
							</div>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="orphan-empty">캐릭터가 없는 고아 채팅이 없음</p>
			{/if}
		</Group>

		<Group title="대화 정리">
			{#snippet aside()}
				{#if picked.size}<Button size="sm" variant="danger" onclick={removePicked}>{#snippet icon()}<Trash2 />{/snippet}고른 대화 {picked.size}개 삭제</Button>{/if}
			{/snippet}
			<ul class="chars">
				{#each data.characters as c (c.id)}
					<li>
						<button class="char" aria-expanded={openChar === c.id} disabled={!c.conversations.length} onclick={() => (openChar = openChar === c.id ? null : c.id)}>
							{#if openChar === c.id}<ChevronDown />{:else}<ChevronRight />{/if}
							<b>{c.name}</b>
							<span class="meta">대화 {c.conversationCount} · 메시지 {c.messageCount.toLocaleString()}{c.imageCount ? ` · 이미지 ${c.imageCount}` : ""}</span>
							<span class="when">{c.lastActivity ? relative(c.lastActivity) : ""}</span>
						</button>
						{#if openChar === c.id}
							<ul class="convs">
								{#each c.conversations as v (v.id)}
									<li>
										<label>
											<input type="checkbox" checked={picked.has(v.id)} onchange={(e) => pick(v.id, e.currentTarget.checked)} />
											<span class="t">{convTitle(v)}</span>
											{#if v.mode === "phone"}<Badge>폰</Badge>{/if}
											<span class="meta">메시지 {v.messageCount}{v.hiddenCount ? ` · 숨김 ${v.hiddenCount}` : ""}</span>
											<span class="when">{v.lastMessageAt ? relative(v.lastMessageAt) : ""}</span>
										</label>
									</li>
								{/each}
							</ul>
						{/if}
					</li>
				{/each}
			</ul>
		</Group>
	{/if}
</Section>

<style>
	.st-warn {
		padding: var(--space-2) var(--space-4) var(--space-3);
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.st-warn summary {
		cursor: pointer;
	}
	.st-warn ul {
		margin: var(--space-2) 0 0;
		padding-left: var(--space-5);
	}
	.check {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-3) 0;
		font-size: var(--text-sm);
	}
	.check p {
		margin: 0;
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.problems,
	.orphans {
		margin: 0;
		padding: 0;
		list-style: none;
		max-height: 240px;
		overflow: auto;
	}
	.orphans li {
		display: flex;
		justify-content: space-between;
		gap: var(--space-3);
		padding: 2px 0;
	}
	.orphans code {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.loading {
		display: grid;
		place-items: center;
		padding: var(--space-8);
	}
	.banner {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		margin-bottom: var(--space-4);
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--accent);
		border-radius: var(--radius-lg);
		background: var(--accent-soft);
	}
	.banner > :global(svg) {
		width: 18px;
		height: 18px;
		color: var(--accent);
	}
	.banner div {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.banner span {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
	.usage {
		padding: var(--space-4) 0;
	}
	.total {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
		margin-bottom: var(--space-2);
	}
	.total b {
		font-size: var(--text-xl);
	}
	.total span {
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.bar {
		display: flex;
		gap: 2px;
		height: 10px;
		border-radius: 999px;
		overflow: hidden;
		background: var(--surface-active);
	}
	.bar span {
		flex-basis: 0;
		min-width: 3px;
	}
	.legend {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-4);
		margin: var(--space-3) 0 0;
		padding: 0;
		list-style: none;
		font-size: var(--text-sm);
	}
	.legend li {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.legend i {
		width: 8px;
		height: 8px;
		border-radius: 50%;
	}
	.legend span {
		color: var(--text-muted);
	}
	.counts {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(110px, 1fr));
		gap: var(--space-3);
		margin: 0;
		padding: var(--space-4) 0;
		border-top: 1px solid var(--border);
	}
	.counts div {
		display: flex;
		flex-direction: column;
	}
	.counts dt {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.counts dd {
		margin: 0;
		font-size: var(--text-lg, 1.1rem);
		font-weight: 600;
		font-variant-numeric: tabular-nums;
	}
	.chars,
	.convs {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.chars {
		padding: var(--space-2) 0;
	}
	.chars > li + li {
		border-top: 1px solid var(--border);
	}
	.char,
	.convs label {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		min-height: 40px;
		padding: var(--space-1) var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.char:disabled {
		cursor: default;
	}
	.char :global(svg) {
		width: 16px;
		height: 16px;
		color: var(--text-faint);
		flex-shrink: 0;
	}
	.char:not(:disabled):hover,
	.convs label:hover {
		background: var(--surface-hover);
	}
	.convs {
		padding: 0 0 var(--space-2) var(--space-6);
	}
	.convs label {
		min-height: 34px;
		font-size: var(--text-sm);
	}
	.t {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-sm);
		white-space: nowrap;
	}
	.when {
		margin-left: auto;
		color: var(--text-faint);
		font-size: var(--text-xs);
		white-space: nowrap;
	}
	input[type="checkbox"] {
		accent-color: var(--accent);
	}
	.orphan-bar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.orphan-desc {
		margin: var(--space-3) 0 var(--space-1);
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.orphan-empty {
		padding: var(--space-4) 0;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
	.orphan-list {
		padding: var(--space-2) 0;
	}
	.orphan-row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.orphan-row + .orphan-row {
		border-top: 1px solid var(--border);
	}
	.orphan-row label {
		flex: 1;
		min-width: 0;
	}
	.orphan-acts {
		display: flex;
		align-items: center;
		gap: 2px;
		flex: none;
	}
</style>
