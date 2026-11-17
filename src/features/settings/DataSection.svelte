<script lang="ts">
	import { onMount } from "svelte";
	import { ChevronDown, ChevronRight, RotateCcw, Save, ShieldCheck, Trash2, Upload } from "@lucide/svelte";
	import type { BackupManifest, DataCheck, DataOverview_Serialize as Overview, DataTotals } from "$ipc/bindings";
	import { Badge, Button, Spinner } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo, BACKUP_FILES } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import TrashGroup from "./TrashGroup.svelte";

	const src = { source: "data" };
	let data = $state<Overview | null>(null);
	let pending = $state<BackupManifest | null>(null);
	let busy = $state<"backup" | "restore" | null>(null);
	let openChar = $state<string | null>(null);
	let picked = $state<Set<string>>(new Set());
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
				return call(commands.exportBackup(path), src);
			});
			if (r) toasts.success(`백업함 · ${bytes(r.bytes)}, 파일 ${r.fileCount}개`);
		} finally {
			busy = null;
		}
	}
	async function restore() {
		const [path] = await pickFiles([BACKUP_FILES], false);
		if (!path) return;
		busy = "restore";
		try {
			const p = await call(commands.prepareRestore(path), src);
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
		const removed = await call(commands.deleteConversations([...picked]), src);
		toasts.success(`대화 ${removed}개를 휴지통으로 옮김`);
		trashKey++; // 휴지통 목록 다시 읽기
		await Promise.all([load(), library.load()]);
	}
	const convTitle = (c: { title: string | null; mode: string | null }) => c.title || (c.mode === "phone" ? "폰 대화" : "제목 없음");
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

		{#key trashKey}<TrashGroup onrestored={() => Promise.all([load(), library.load()])} />{/key}

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
				{#if data.orphanConversations.length}
					<li>
						<button class="char" aria-expanded={openChar === "__orphan"} onclick={() => (openChar = openChar === "__orphan" ? null : "__orphan")}>
							{#if openChar === "__orphan"}<ChevronDown />{:else}<ChevronRight />{/if}
							<b>캐릭터 없는 대화</b>
							<span class="meta">{data.orphanConversations.length}개 · 캐릭터가 지워짐</span>
						</button>
						{#if openChar === "__orphan"}
							<ul class="convs">
								{#each data.orphanConversations as v (v.id)}
									<li>
										<label>
											<input type="checkbox" checked={picked.has(v.id)} onchange={(e) => pick(v.id, e.currentTarget.checked)} />
											<span class="t">{convTitle(v)}</span>
											<span class="meta">메시지 {v.messageCount}</span>
										</label>
									</li>
								{/each}
							</ul>
						{/if}
					</li>
				{/if}
			</ul>
		</Group>
	{/if}
</Section>

<style>
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
</style>
