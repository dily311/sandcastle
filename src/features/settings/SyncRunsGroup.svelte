<script lang="ts">
	// 동기화 실행 기록 (성공·실패). 예전에는 설정 > 동기화 안에 있었고, 이제 설정 > 로그에서 본다.
	// 동기화를 쓰는 기기에서만 읽는다 (Lazy: 이 화면을 열 때)
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import type { SyncRun } from "$ipc/bindings";
	import { Badge } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { absolute, relative } from "$shared/time";
	import Group from "./Group.svelte";

	let runs = $state<SyncRun[]>([]);
	onMount(async () => {
		if (!plugins.loaded) await plugins.load();
		if (plugins.on("sync")) runs = await call(commands.listSyncRuns(30), { source: "sync", silent: true }).catch(() => []);
	});

	const n = (v: number) => v.toLocaleString();
	function runText(r: SyncRun) {
		if (!r.ok) return r.error ?? $_("settings.more2.syncRunFail");
		const s = r.summary;
		if (!s) return "";
		const merged = s.merged.reduce((a, d) => a + d.report.added + d.report.updated + d.report.deleted + d.report.restored, 0);
		const conflicts = s.merged.reduce((a, d) => a + d.report.conflicts + d.report.branches, 0);
		return [
			s.merged.length ? $_("settings.more2.syncRunMerged", { values: { devices: s.merged.length, count: n(merged) } }) : $_("settings.more2.syncRunNothing"),
			conflicts && $_("settings.more2.syncRunConflicts", { values: { count: n(conflicts) } }),
			s.filesDown && $_("settings.more2.syncRunFilesDown", { values: { count: n(s.filesDown) } }),
			s.filesUp && $_("settings.more2.syncRunFilesUp", { values: { count: n(s.filesUp) } }),
			s.filesMissing && $_("settings.more2.syncRunFilesMissing", { values: { count: n(s.filesMissing) } }),
		]
			.filter(Boolean)
			.join(" · ");
	}
</script>

{#if runs.length}
	<Group title={$_("settings.more2.syncRunsTitle")} flush>
		<ul class="runs" aria-label={$_("settings.more2.syncRunsTitle")}>
			{#each runs as r (r.id)}
				<li class:fail={!r.ok}>
					<span title={absolute(r.startedAt)}>{relative(r.startedAt)}</span>
					<Badge tone={r.ok ? undefined : "danger"}>{r.runBy === "auto" ? $_("settings.more2.syncRunAuto") : $_("settings.more2.syncRunManual")}</Badge>
					<span class="what">{runText(r)}</span>
				</li>
			{/each}
		</ul>
	</Group>
{/if}

<style>
	.runs {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.runs li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-2) var(--space-2);
		font-size: var(--text-sm);
		flex-wrap: wrap;
	}
	.runs li + li {
		border-top: 1px solid var(--border);
	}
	.runs .what {
		color: var(--text-muted);
		flex: 1;
		min-width: 12rem;
	}
	.runs .fail .what {
		color: var(--danger, var(--text-muted));
	}
</style>
