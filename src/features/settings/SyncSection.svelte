<script lang="ts">
	// 동기화 (CLAUDE.md 2-B-3, ADR-011): 파일로 주고받기. 누를 때만 만들고, 합치기 전에 미리보기 → 자동 사본 → 적용
	import { onMount } from "svelte";
	import { Download, FolderOpen, Info } from "@lucide/svelte";
	import type { SyncPreview, SyncStatus } from "$ipc/bindings";
	import { Badge, Button, Spinner, Switch } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, saveTo, SYNC_FILES } from "$shared/ipc/dialog";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "sync" };
	let status = $state<SyncStatus | null>(null);
	let includeFiles = $state(true);
	let busy = $state<"export" | "open" | "apply" | null>(null);
	/** 열어 본 묶음 (적용 전) */
	let pending = $state<{ path: string; preview: SyncPreview } | null>(null);

	onMount(load);
	async function load() {
		status = await call(commands.getSyncStatus(), src).catch(() => null);
	}
	const short = (id: string) => id.slice(0, 8);
	const n = (v: number) => v.toLocaleString();
	function size(b: number) {
		return b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`;
	}

	async function exportBundle() {
		const d = new Date();
		const stamp = `${d.getFullYear()}${String(d.getMonth() + 1).padStart(2, "0")}${String(d.getDate()).padStart(2, "0")}-${String(d.getHours()).padStart(2, "0")}${String(d.getMinutes()).padStart(2, "0")}`;
		try {
			const r = await saveTo(`sandcastle-sync-${stamp}.scsync`, [SYNC_FILES], (path) => {
				busy = "export";
				return call(commands.exportSyncBundle(path, includeFiles), src);
			});
			if (r) toasts.success(`동기화 파일을 만듦 · 항목 ${n(r.rows)}개, 파일 ${n(r.blobs)}/${n(r.files)}개, ${size(r.bytes)}`);
		} finally {
			busy = null;
		}
	}

	async function openBundle() {
		const [path] = await pickFiles([SYNC_FILES], false);
		if (!path) return;
		busy = "open";
		try {
			pending = { path, preview: await call(commands.previewSyncBundle(path), src) };
		} catch {
			pending = null; // 이유는 call이 토스트로
		} finally {
			busy = null;
		}
	}

	async function apply() {
		if (!pending) return;
		busy = "apply";
		try {
			const r = await call(commands.applySyncBundle(pending.path), src);
			const m = r.report;
			const parts = [
				m.added && `추가 ${n(m.added)}`,
				m.updated && `변경 ${n(m.updated)}`,
				m.deleted && `지움 ${n(m.deleted)}`,
				m.restored && `되살림 ${n(m.restored)}`,
				r.files.added && `파일 ${n(r.files.added)}`,
				m.conflicts + m.branches && `둘 다 남김 ${n(m.conflicts + m.branches)}`,
			].filter(Boolean);
			toasts.success(parts.length ? `합침: ${parts.join(" · ")}` : "합침: 바뀐 것 없음");
			pending = null;
			await Promise.all([load(), library.load()]);
		} finally {
			busy = null;
		}
	}

	const rows = $derived.by(() => {
		if (!pending) return [];
		const m = pending.preview.report;
		const f = pending.preview.files;
		return [
			{ label: "추가", value: m.added, hint: "이 기기에 없던 항목" },
			{ label: "변경", value: m.updated, hint: "다른 기기에서만 고친 항목" },
			{ label: "지움", value: m.deleted, hint: "다른 기기에서 지웠고 이 기기는 그 뒤로 안 고친 항목" },
			{ label: "되살림", value: m.restored, hint: "이 기기에서 지웠지만 다른 기기에서 고친 항목 (고친 쪽을 살림)" },
			{ label: "이 기기 고침 유지", value: m.keptLocalEdits, hint: "다른 기기에서 지웠지만 이 기기에서 고친 항목" },
			{ label: "둘 다 남김", value: m.conflicts, hint: "양쪽에서 고친 항목. 이 기기 것을 두고 받은 것은 따로 보관" },
			{ label: "분기 대화", value: m.branches, hint: "같은 대화에 양쪽이 이어 씀. 받은 쪽을 새 분기로" },
			{ label: "파일 추가", value: f.added, hint: "이 기기에 없는 그림·파일" },
			{ label: "빠진 파일", value: f.missing, hint: "묶음에 내용이 없음 — 보낸 기기에서 '파일 포함'으로 다시 만들면 들어옴" },
			{ label: "이 기기 파일 유지", value: f.keptLocal, hint: "같은 자리에 내용이 다른 파일이 있어 이 기기 것을 둠" },
		].filter((r) => r.value > 0);
	});
</script>

<Section title="동기화" lead="다른 기기(PC·폰)와 파일로 주고받아 합침. API 키는 넘어가지 않음 — 받는 기기에서 다시 넣기">
	{#if !status}
		<div class="loading"><Spinner /></div>
	{:else}
		<Group title="보내기">
			<Row label="그림·파일 포함" desc="끄면 항목과 설정만 (파일 목록은 들어가서, 받는 쪽이 빠진 파일 수를 알 수 있음)">
				<Switch checked={includeFiles} label="그림·파일 포함" hideLabel onchange={(v) => (includeFiles = v)} />
			</Row>
			<Row label="동기화 파일 만들기" desc="캐릭터·대화·로어북·프리셋·게임·같이 쓰는 설정. 실행 기록·휴지통·이 기기 전용 설정은 빠짐">
				<Button size="sm" loading={busy === "export"} disabled={busy !== null} onclick={exportBundle}>{#snippet icon()}<Download />{/snippet}만들기</Button>
			</Row>
		</Group>

		<Group title="받기">
			<Row label="동기화 파일 열기" desc="합치기 전에 무엇이 바뀌는지 먼저 보여줌. 적용하면 이 기기 DB·설정 사본을 먼저 만듦">
				<Button size="sm" loading={busy === "open"} disabled={busy !== null} onclick={openBundle}>{#snippet icon()}<FolderOpen />{/snippet}열기</Button>
			</Row>
			{#if pending}
				{@const b = pending.preview.bundle}
				<div class="preview" role="region" aria-label="합치기 미리보기">
					<p class="from">
						<Badge>기기 {short(b.deviceId)}</Badge>
						<span title={absolute(b.createdAt)}>{relative(b.createdAt)}에 만듦</span>
						<span class="meta">앱 {b.appVersion} · {size(b.bytes)}</span>
					</p>
					{#if rows.length}
						<dl class="counts">
							{#each rows as r (r.label)}
								<div title={r.hint}><dt>{r.label}</dt><dd>{n(r.value)}</dd></div>
							{/each}
						</dl>
					{:else}
						<p class="meta">바뀔 것이 없음 (이미 합친 내용)</p>
					{/if}
					{#if pending.preview.report.settingsApplied}
						<p class="note"><Info /> 같이 쓰는 설정(테마·언어·프롬프트 등)은 받은 것으로 바뀜. 이 기기 전용(API 선택·기능 켜기·글자 크기 등)은 그대로</p>
					{/if}
					{#if pending.preview.report.conflicts + pending.preview.report.branches > 0}
						<p class="note"><Info /> 둘 다 남긴 항목은 지금은 이 기기 것이 보이고, 받은 것은 따로 보관됨 (고르는 화면은 준비 중)</p>
					{/if}
					<div class="acts">
						<Button size="sm" variant="ghost" disabled={busy !== null} onclick={() => (pending = null)}>취소</Button>
						<Button size="sm" variant="primary" loading={busy === "apply"} disabled={busy !== null} onclick={apply}>합치기</Button>
					</div>
				</div>
			{/if}
		</Group>

		<Group title="기기">
			<Row label="이 기기" desc="동기화 파일에 적히는 이 설치의 표시. 백업을 다른 기기에 복원해도 바뀌지 않음">
				<code>{short(status.deviceId)}</code>
			</Row>
			{#if status.openConflicts > 0}
				<Row label="둘 다 남긴 항목" desc="양쪽에서 고쳐 받은 쪽을 따로 보관한 것 (고르는 화면은 준비 중)">
					<Badge tone="warning">{n(status.openConflicts)}</Badge>
				</Row>
			{/if}
			{#if status.peers.length}
				<ul class="peers">
					{#each status.peers as p (p.deviceId)}
						<li>
							<code>{short(p.deviceId)}</code>
							<span title={absolute(p.lastMergeAt)}>마지막으로 합침 {relative(p.lastMergeAt)}</span>
							<span class="meta">{n(p.merges)}번</span>
						</li>
					{/each}
				</ul>
			{:else}
				<p class="meta empty">아직 합친 기기 없음</p>
			{/if}
		</Group>
	{/if}
</Section>

<style>
	.loading {
		display: flex;
		justify-content: center;
		padding: var(--space-6);
	}
	.preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		margin: var(--space-2) 0 var(--space-3);
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface-2, var(--surface));
	}
	.from {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
		margin: 0;
	}
	.counts {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
		gap: var(--space-2);
		margin: 0;
	}
	.counts div {
		display: flex;
		justify-content: space-between;
		gap: var(--space-2);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-size: var(--text-sm);
	}
	.counts dt {
		color: var(--text-muted);
	}
	.counts dd {
		margin: 0;
		font-weight: 600;
	}
	.note {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.note :global(svg) {
		flex-shrink: 0;
		width: 1em;
		height: 1em;
		margin-top: 0.2em;
	}
	.acts {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.empty {
		margin: 0;
		padding: var(--space-3) 0;
	}
	.peers {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.peers li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) 0;
		border-top: 1px solid var(--border);
		font-size: var(--text-sm);
		flex-wrap: wrap;
	}
</style>
