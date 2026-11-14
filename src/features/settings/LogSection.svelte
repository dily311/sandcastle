<script lang="ts">
	import { Copy, Trash2 } from "@lucide/svelte";
	import { Badge, Button, EmptyState, Segmented } from "$shared/ui";
	import { logs, type LogEntry, type LogLevel } from "$shared/stores/log.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";

	let level = $state<"all" | LogLevel>("all");
	let open = $state<number | null>(null);
	const shown = $derived(level === "all" ? logs.entries : logs.entries.filter((e) => e.level === level));

	const pad = (n: number) => String(n).padStart(2, "0");
	const clock = (t: number) => {
		const d = new Date(t);
		return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
	};
	const tone = (l: LogLevel) => (l === "error" ? "danger" : l === "warn" ? "warning" : "neutral");
	function detail(e: LogEntry) {
		if (e.details === undefined || e.details === null) return null;
		try {
			return typeof e.details === "string" ? e.details : JSON.stringify(e.details, null, 2);
		} catch {
			return String(e.details);
		}
	}
	async function copyAll() {
		const text = shown
			.map((e) => `[${new Date(e.time).toISOString()}] ${e.level.toUpperCase()} ${e.source}: ${e.message}${detail(e) ? `\n${detail(e)}` : ""}`)
			.join("\n");
		try {
			await navigator.clipboard.writeText(text);
			toasts.success("복사함");
		} catch {
			toasts.error("복사 못 함");
		}
	}
</script>

<Section title="로그" lead="이번 실행 동안 생긴 오류·알림. 앱을 끄면 지워짐">
	{#snippet actions()}
		<Button size="sm" disabled={!shown.length} onclick={copyAll}>{#snippet icon()}<Copy />{/snippet}복사</Button>
		<Button size="sm" variant="ghost" disabled={!logs.entries.length} onclick={() => logs.clear()}>{#snippet icon()}<Trash2 />{/snippet}비우기</Button>
	{/snippet}
	<div class="filter">
		<Segmented
			label="로그 종류"
			bind:value={level}
			options={[
				{ value: "all", label: `전체 ${logs.entries.length}` },
				{ value: "error", label: `오류 ${logs.errorCount}` },
				{ value: "warn", label: "경고" },
				{ value: "info", label: "알림" },
			]}
		/>
	</div>
	<Group flush>
		{#each shown as l (l.id)}
			{@const d = detail(l)}
			<div class="row">
				<button class="line" disabled={!d} aria-expanded={d ? open === l.id : undefined} onclick={() => (open = open === l.id ? null : l.id)}>
					<Badge tone={tone(l.level)}>{l.level}</Badge>
					<span class="t">{clock(l.time)}</span>
					<span class="src">{l.source}</span>
					<span class="m">{l.message}</span>
				</button>
				{#if d && open === l.id}<pre>{d}</pre>{/if}
			</div>
		{:else}
			<EmptyState title="기록 없음" description="오류나 알림이 생기면 여기 쌓임" />
		{/each}
	</Group>
</Section>

<style>
	.filter {
		margin-bottom: var(--space-3);
	}
	.row + .row {
		border-top: 1px solid var(--border);
	}
	.line {
		display: grid;
		grid-template-columns: 56px 64px 72px 1fr;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
	}
	.line:not(:disabled) {
		cursor: pointer;
	}
	.line:not(:disabled):hover {
		background: var(--surface-hover);
	}
	.t,
	.src {
		color: var(--text-faint);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
	}
	.m {
		overflow-wrap: anywhere;
	}
	pre {
		margin: 0 var(--space-2) var(--space-2);
		padding: var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--surface);
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		max-height: 240px;
		overflow: auto;
	}
</style>
