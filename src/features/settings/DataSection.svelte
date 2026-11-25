<script lang="ts">
	import { jobs } from "$shared/stores/jobs.svelte";
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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
				return jobs.run($_("settings.dataSec.stExportJob"), () => call(commands.exportSillytavernData(path), src), { group: $_("settings.dataSec.stExportGroup") });
			});
			if (r) toasts.success($_("settings.dataSec.stExported", { values: { characters: r.characters, chats: r.chats, books: r.books, presets: r.presets, personas: r.personas, regex: r.regex } }));
			if (r?.warnings.length) toasts.error($_("settings.dataSec.stExportWarnings", { values: { count: r.warnings.length } }), r.warnings.join("\n"));
		} finally {
			busy = null;
		}
	}
	async function importSt() {
		const [path] = await pickFiles([ST_DATA_FILES], false);
		if (!path) return;
		busy = "st";
		try {
			const r = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.importSillytavernData(path), src), { group: $_("settings.dataSec.stImportGroup") });
			if (!r) return;
			stResult = r;
			toasts.success($_("settings.dataSec.stImported", { values: { characters: r.characters, chats: r.chats, books: r.books, presets: r.presets, regex: r.regex, personas: r.personas } }));
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
	const COUNTS = $derived<[keyof DataTotals, string][]>([
		["characters", $_("settings.dataSec.counts.characters")],
		["conversations", $_("settings.dataSec.counts.conversations")],
		["messages", $_("settings.dataSec.counts.messages")],
		["personas", $_("settings.dataSec.counts.personas")],
		["characterImages", $_("settings.dataSec.counts.characterImages")],
		["worldInfoBooks", $_("settings.dataSec.counts.worldInfoBooks")],
		["promptPresets", $_("settings.dataSec.counts.promptPresets")],
		["regexScripts", $_("settings.dataSec.counts.regexScripts")],
		["games", $_("settings.dataSec.counts.games")],
	]);
	const parts = $derived(
		data
			? [
					{ key: "db", label: $_("settings.dataSec.storage.db"), value: data.storage.databaseBytes, color: "var(--accent)" },
					{ key: "characters", label: $_("settings.dataSec.storage.characters"), value: data.storage.charactersBytes, color: "var(--info, #3f74c9)" },
					{ key: "games", label: $_("settings.dataSec.storage.games"), value: data.storage.gamesBytes, color: "var(--warning, #a87a2f)" },
					{ key: "themes", label: $_("settings.dataSec.storage.themes"), value: data.storage.themesBytes, color: "var(--text-faint)" },
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
				return jobs.run($_("settings.dataSec.backupBtn"), () => call(commands.exportBackup(path), src), { group: $_("settings.dataSec.backupGroup") });
			});
			if (r) toasts.success($_("settings.dataSec.backupDone", { values: { size: bytes(r.bytes), count: r.fileCount } }));
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
			label: $_("settings.dataSec.dropRestoreLabel"),
			hint: $_("settings.dataSec.dropRestoreHint"),
			accept: (f) => extOf(f) === "scbackup",
			multiple: false,
			maxBytes: 300 * 1024 * 1024,
			onpaths: ([path]) => restoreFrom(path),
		}),
	);
	async function restoreFrom(path: string) {
		busy = "restore";
		try {
			const p = await jobs.run(path.split(/[\\/]/).pop() ?? path, () => call(commands.prepareRestore(path), src), { group: $_("settings.dataSec.prepareRestoreGroup") });
			if (!p) return;
			const t = p.totals;
			const ok = await confirmAction({
				title: $_("settings.dataSec.restoreConfirmTitle"),
				message: $_("settings.dataSec.restoreConfirmMsg", {
					values: {
						date: absolute(p.createdAt),
						version: p.appVersion,
						characters: t.characters,
						conversations: t.conversations,
						messages: t.messages,
						migrate: p.willMigrate ? $_("settings.dataSec.restoreWillMigrate") : "",
					},
				}),
				confirmLabel: $_("settings.dataSec.restoreConfirmBtn"),
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
		toasts.success($_("settings.dataSec.restoreCancelled"));
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
		if (!(await confirmAction({ title: $_("settings.dataSec.deleteConvTitle"), message: $_("settings.dataSec.deleteConvMsg", { values: { count: n } }), confirmLabel: $_("settings.dataSec.deleteCountBtn", { values: { count: n } }), danger: true }))) return;
		const ids = [...picked];
		const removed = await jobs.run($_("settings.dataSec.deleteConvJob", { values: { count: n } }), () => call(commands.deleteConversations(ids), src), { group: $_("settings.dataSec.deleteConvGroup") });
		if (removed !== undefined) toasts.success($_("settings.dataSec.deleteConvDone", { values: { count: removed } }));
		trashKey++; // 휴지통 목록 다시 읽기
		await Promise.all([load(), library.load()]);
	}
	const convTitle = (c: { title: string | null; mode: string | null }) => c.title || (c.mode === "phone" ? $_("settings.dataSec.phoneChat") : $_("settings.dataSec.untitled"));

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
		const charName = data?.characters.find((c) => c.id === orphanTargetChar)?.name ?? $_("settings.dataSec.counts.characters");
		const n = await call(commands.reassignConversationsCharacter(ids, orphanTargetChar), src);
		toasts.success($_("settings.dataSec.orphanReassigned", { values: { count: n, name: charName } }));
		await Promise.all([load(), library.load()]);
	}
	async function removeOrphans(ids: string[]) {
		if (!ids.length) return;
		if (!(await confirmAction({ title: $_("settings.dataSec.orphanDeleteTitle"), message: $_("settings.dataSec.orphanDeleteMsg", { values: { count: ids.length } }), confirmLabel: $_("settings.dataSec.deleteCountBtn", { values: { count: ids.length } }), danger: true }))) return;
		const removed = await jobs.run($_("settings.dataSec.orphanDeleteJob", { values: { count: ids.length } }), () => call(commands.deleteConversations(ids), src), { group: $_("settings.dataSec.deleteConvGroup") });
		if (removed !== undefined) toasts.success($_("settings.dataSec.orphanDeleteDone", { values: { count: removed } }));
		trashKey++;
		await Promise.all([load(), library.load()]);
	}
</script>

<Section title={$_("settings.dataSec.title")} lead={$_("settings.dataSec.lead")}>
	{#if !data}
		<div class="loading"><Spinner /></div>
	{:else}
		{#if pending}
			<div class="banner">
				<RotateCcw />
				<div>
					<b>{$_("settings.dataSec.restoreBtn")}</b>
					<span>{absolute(pending.createdAt)}</span>
				</div>
				<Button size="sm" variant="ghost" onclick={cancelPending}>{$_("common.cancel")}</Button>
				<Button size="sm" variant="primary" onclick={() => call(commands.restartApp(), src)}>{$_("settings.dataSec.restoreBtn")}</Button>
			</div>
		{/if}

		<Group>
			<div class="usage">
				<div class="total"><b>{bytes(data.storage.totalBytes)}</b><span>{$_("settings.dataSec.inUse")}</span></div>
				<div class="bar" role="img" aria-label={$_("settings.dataSec.title")}>
					{#each parts as p (p.key)}
						{#if p.value > 0}<span style:flex-grow={p.value} style:background={p.color} title="{p.label} {bytes(p.value)}"></span>{/if}
					{/each}
				</div>
				<ul class="legend">
					{#each parts as p (p.key)}
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

		<Group title={$_("settings.dataSec.backupGroup")}>
			<Row label={$_("settings.dataSec.createBackup")} desc={$_("settings.dataSec.createBackupDesc")}>
				<Button size="sm" loading={busy === "backup"} disabled={busy !== null} onclick={backup}>{#snippet icon()}<Save />{/snippet}{$_("settings.dataSec.backupBtn")}</Button>
			</Row>
			<Row label={$_("settings.dataSec.restoreBackup")} desc={$_("settings.dataSec.restoreBackupDesc")}>
				<Button size="sm" loading={busy === "restore"} disabled={busy !== null} onclick={restore}>{#snippet icon()}<Upload />{/snippet}{$_("settings.dataSec.restoreBtn")}</Button>
			</Row>
		</Group>

		<Group title="SillyTavern">
			<Row label={$_("settings.dataSec.importSt")} desc={$_("settings.dataSec.importStDesc")}>
				<Button size="sm" loading={busy === "st"} disabled={busy !== null} onclick={importSt}>{#snippet icon()}<Upload />{/snippet}{$_("common.import")}</Button>
			</Row>
			<Row label={$_("settings.dataSec.exportSt")} desc={$_("settings.dataSec.exportStDesc")}>
				<Button size="sm" loading={busy === "st-out"} disabled={busy !== null} onclick={exportSt}>{#snippet icon()}<Save />{/snippet}{$_("common.export")}</Button>
			</Row>
			{#if stResult?.warnings.length}
				<details class="st-warn">
					<summary>({stResult.warnings.length})</summary>
					<ul>{#each stResult.warnings as w, i (i)}<li>{w}</li>{/each}</ul>
				</details>
			{/if}
		</Group>

		<Group title={$_("settings.dataSec.checkGroup")}>
			<Row label={$_("settings.dataSec.dbCheck")} desc={$_("settings.dataSec.dbCheckDesc")}>
				<Button size="sm" loading={checking} disabled={checking} onclick={runCheck}>{#snippet icon()}<ShieldCheck />{/snippet}{$_("settings.dataSec.checkBtn")}</Button>
			</Row>
			{#if check}
				<div class="check" role="status">
					{#if check.ok}
						<p><Badge tone="success">{$_("settings.dataSec.dbOk")}</Badge></p>
					{:else}
						<p><Badge tone="danger">{$_("settings.dataSec.dbProblem")}</Badge> {$_("settings.dataSec.dbProblemHint")}</p>
						<ul class="problems">
							{#each check.problems as p, i (i)}<li><code>{p}</code></li>{/each}
						</ul>
					{/if}
					<p>
						{$_("settings.dataSec.checkedFiles", { values: { count: check.checkedFiles.toLocaleString() } })}
						{#if check.orphanCount}
							{$_("settings.dataSec.orphanFiles", { values: { count: check.orphanCount.toLocaleString(), size: kb(check.orphanBytes) } })}
							<Button size="sm" variant="ghost" aria-expanded={showOrphans} onclick={() => (showOrphans = !showOrphans)}>{showOrphans ? $_("settings.dataSec.hideList") : $_("settings.dataSec.showList")}</Button>
						{:else}
							{$_("settings.dataSec.noOrphanFiles")}
						{/if}
					</p>
					{#if showOrphans && check.orphans.length}
						<ul class="orphans">
							{#each check.orphans as o (o.path)}<li><code>{o.path}</code><span class="meta">{kb(o.bytes)}</span></li>{/each}
						</ul>
						{#if check.orphanCount > check.orphans.length}<p class="meta">{$_("settings.dataSec.showingLargest", { values: { count: check.orphans.length } })}</p>{/if}
					{/if}
				</div>
			{/if}
		</Group>

		<PluginVersionsGroup />

		{#key trashKey}<TrashGroup onrestored={() => Promise.all([load(), library.load()])} />{/key}

		<Group title={$_("settings.dataSec.orphanGroup", { values: { count: data.orphanConversations.length } })}>
			{#snippet aside()}
				{#if data?.orphanConversations.length}
					<div class="orphan-bar">
						<Button size="sm" variant="ghost" onclick={toggleAllOrphans}>
							{orphanPicked.size === (data?.orphanConversations.length ?? 0) ? $_("settings.dataSec.deselectAll") : $_("settings.dataSec.selectAll")}
						</Button>
						{#if orphanPicked.size && charOptions.length}
							<Select bind:value={orphanTargetChar} options={charOptions} aria-label={$_("settings.dataSec.targetChar")} />
							<Button size="sm" variant="secondary" onclick={() => reassignOrphans([...orphanPicked])}>{#snippet icon()}<Link2 />{/snippet}{$_("settings.dataSec.linkCount", { values: { count: orphanPicked.size } })}</Button>
						{/if}
						{#if orphanPicked.size}
							<Button size="sm" variant="danger" onclick={() => removeOrphans([...orphanPicked])}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.dataSec.deleteCount", { values: { count: orphanPicked.size } })}</Button>
						{/if}
					</div>
				{/if}
			{/snippet}
			{#if data.orphanConversations.length}
				<p class="orphan-desc">{$_("settings.dataSec.orphanDesc")}</p>
				<ul class="convs orphan-list">
					{#each data.orphanConversations as v (v.id)}
						<li class="orphan-row">
							<label>
								<input type="checkbox" checked={orphanPicked.has(v.id)} onchange={(e) => pickOrphan(v.id, e.currentTarget.checked)} />
								<span class="t">{convTitle(v)}</span>
								{#if v.mode === "phone"}<Badge>{$_("settings.dataSec.phoneBadge")}</Badge>{/if}
								<span class="meta">{$_("settings.dataSec.msgCount", { values: { count: v.messageCount } })}</span>
								<span class="when">{v.lastMessageAt ? relative(v.lastMessageAt) : ""}</span>
							</label>
							<div class="orphan-acts">
								{#if charOptions.length}
									<Button size="sm" variant="ghost" title={$_("settings.dataSec.linkToCharTitle")} onclick={() => reassignOrphans([v.id])}>{#snippet icon()}<Link2 />{/snippet}{$_("settings.dataSec.linkBtn")}</Button>
								{/if}
								<Button size="sm" variant="ghost" aria-label={$_("common.delete")} onclick={() => removeOrphans([v.id])}>{#snippet icon()}<Trash2 />{/snippet}</Button>
							</div>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="orphan-empty">{$_("settings.dataSec.orphanEmpty")}</p>
			{/if}
		</Group>

		<Group title={$_("settings.dataSec.cleanupGroup")}>
			{#snippet aside()}
				{#if picked.size}<Button size="sm" variant="danger" onclick={removePicked}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.dataSec.deleteSelectedConvs", { values: { count: picked.size } })}</Button>{/if}
			{/snippet}
			<ul class="chars">
				{#each data.characters as c (c.id)}
					<li>
						<button class="char" aria-expanded={openChar === c.id} disabled={!c.conversations.length} onclick={() => (openChar = openChar === c.id ? null : c.id)}>
							{#if openChar === c.id}<ChevronDown />{:else}<ChevronRight />{/if}
							<b>{c.name}</b>
							<span class="meta">{c.imageCount ? $_("settings.dataSec.charSummaryWithImg", { values: { convs: c.conversationCount, msgs: c.messageCount.toLocaleString(), imgs: c.imageCount } }) : $_("settings.dataSec.charSummary", { values: { convs: c.conversationCount, msgs: c.messageCount.toLocaleString() } })}</span>
							<span class="when">{c.lastActivity ? relative(c.lastActivity) : ""}</span>
						</button>
						{#if openChar === c.id}
							<ul class="convs">
								{#each c.conversations as v (v.id)}
									<li>
										<label>
											<input type="checkbox" checked={picked.has(v.id)} onchange={(e) => pick(v.id, e.currentTarget.checked)} />
											<span class="t">{convTitle(v)}</span>
											{#if v.mode === "phone"}<Badge>{$_("settings.dataSec.phoneBadge")}</Badge>{/if}
											<span class="meta">{v.hiddenCount ? $_("settings.dataSec.msgCountHidden", { values: { count: v.messageCount, hidden: v.hiddenCount } }) : $_("settings.dataSec.msgCount", { values: { count: v.messageCount } })}</span>
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
