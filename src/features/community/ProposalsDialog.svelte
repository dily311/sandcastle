<script lang="ts">
	// 고치기 제안 고르기 (6-D-3): 바뀌는 칸만 전·후로 보여 주고 적용/거절, 적용한 것은 되돌리기
	import type { ChangeProposal } from "$ipc/bindings";
	import { Badge, Button, Dialog, EmptyState } from "$shared/ui";
	import { diffLines, fold, show } from "./diff";
	import { pluginHost } from "./host.svelte";
	import { proposals, proposalsUi } from "./proposals.svelte";

	const KIND: Record<ChangeProposal["kind"], string> = {
		character: "캐릭터",
		persona: "페르소나",
		lorebook: "로어북",
		lorebook_entry: "로어북 항목",
		message: "메시지",
	};
	const STATUS: Record<string, string> = { applied: "적용함", rejected: "거절함", reverted: "되돌림" };

	let busy = $state<string | null>(null);

	function sourceName(source: string) {
		const id = source.replace(/^plugin:/, "");
		return pluginHost.plugins.find((p) => p.id === id)?.name ?? id;
	}

	async function run(id: string, f: (id: string) => Promise<void>) {
		busy = id;
		try {
			await f(id);
		} finally {
			busy = null;
		}
	}

	const when = (ms: number) => new Date(ms).toLocaleString();
</script>

<Dialog bind:open={proposalsUi.open} title="플러그인의 고치기 제안" description="적용하기 전에는 원본이 바뀌지 않습니다. 적용한 것은 그 뒤 다시 고치지 않았다면 되돌릴 수 있습니다." size="lg">
	{#if proposals.pending.length === 0}
		<EmptyState title="기다리는 제안이 없습니다" />
	{/if}
	{#each proposals.pending as p (p.id)}
		<section class="card">
			<header>
				<Badge tone="info">{KIND[p.kind]}{p.op === "create" ? " 새로 만들기" : ""}</Badge>
				<strong>{p.targetLabel || "(이름 없음)"}</strong>
				<span class="meta">{sourceName(p.source)} · {when(p.createdAt)}</span>
			</header>
			{#if p.reason}<p class="reason">{p.reason}</p>{/if}
			{#each p.fields as f (f.field)}
				<div class="field">
					<div class="label">{f.label}</div>
					{#if typeof f.after === "string" && (f.after.includes("\n") || String(f.before ?? "").includes("\n"))}
						<pre class="diff">{#each fold(diffLines(show(f.before), show(f.after))) as l, i (i)}{#if l.kind === "skip"}<span class="skip">  … 같은 줄 {l.count}개
</span>{:else}<span class={l.kind}>{l.kind === "del" ? "- " : l.kind === "add" ? "+ " : "  "}{l.text}
</span>{/if}{/each}</pre>
					{:else}
						{#if p.op === "update"}<div class="before">{show(f.before) || "(비어 있음)"}</div>{/if}
						<div class="after">{show(f.after) || "(비어 있음)"}</div>
					{/if}
				</div>
			{/each}
			{#if p.outdated}<p class="warn">제안한 뒤 원본이 바뀌어 적용할 수 없습니다. 거절하세요.</p>{/if}
			<footer>
				<Button variant="ghost" size="sm" disabled={busy === p.id} onclick={() => run(p.id, (id) => proposals.reject(id))}>거절</Button>
				<Button variant="primary" size="sm" disabled={p.outdated} loading={busy === p.id} onclick={() => run(p.id, (id) => proposals.apply(id))}>적용</Button>
			</footer>
		</section>
	{/each}

	{#if proposals.decided.length > 0}
		<h3 class="sub">최근에 고른 것</h3>
		{#each proposals.decided as p (p.id)}
			<div class="row">
				<Badge tone={p.status === "applied" ? "success" : "neutral"}>{STATUS[p.status] ?? p.status}</Badge>
				<span class="grow">{KIND[p.kind]} · {p.targetLabel || "(이름 없음)"} · {p.fields.map((f) => f.label).join(", ")}</span>
				<span class="meta">{sourceName(p.source)}</span>
				{#if p.status === "applied"}
					<Button size="sm" variant="ghost" disabled={!p.canRevert || busy === p.id} title={p.canRevert ? "" : "적용한 뒤 다시 고쳐서 되돌릴 수 없음"} onclick={() => run(p.id, (id) => proposals.revert(id))}>되돌리기</Button>
				{/if}
			</div>
		{/each}
	{/if}
</Dialog>

<style>
	.card {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		padding: var(--space-3);
		margin-bottom: var(--space-3);
		display: grid;
		gap: var(--space-2);
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.meta {
		color: var(--text-faint);
		font-size: var(--text-xs);
		margin-left: auto;
	}
	.reason {
		margin: 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.field {
		display: grid;
		gap: var(--space-1);
	}
	.label {
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.before,
	.after {
		padding: var(--space-1) var(--space-2);
		border-radius: var(--radius-xs);
		white-space: pre-wrap;
		word-break: break-word;
		font-size: var(--text-sm);
	}
	.before {
		background: var(--danger-soft);
		text-decoration: line-through;
	}
	.after {
		background: var(--success-soft);
	}
	.diff {
		margin: 0;
		max-height: 320px;
		overflow: auto;
		font-family: var(--font-mono);
		font-size: var(--text-xs);
		white-space: pre-wrap;
		word-break: break-word;
		background: var(--surface-2);
		border-radius: var(--radius-xs);
		padding: var(--space-2);
	}
	.diff .del {
		background: var(--danger-soft);
	}
	.diff .add {
		background: var(--success-soft);
	}
	.diff .same {
		color: var(--text-faint);
	}
	.diff .skip {
		color: var(--text-faint);
		font-style: italic;
	}
	.warn {
		margin: 0;
		color: var(--warning);
		font-size: var(--text-sm);
	}
	footer {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-2);
	}
	.sub {
		font-size: var(--text-sm);
		margin: var(--space-4) 0 var(--space-2);
	}
	.row {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-1) 0;
		font-size: var(--text-sm);
	}
	.grow {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
