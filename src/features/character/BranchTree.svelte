<script lang="ts">
	// 오른쪽 패널 '분기' 탭: 갈래를 타임라인으로 (git 그래프처럼)
	// 세로축 = 메시지 번호, 대화마다 한 줄(레인). 갈라진 메시지에서 옆 레인으로 휘어져 나감.
	// 아무 일도 없는 메시지 구간은 접어서, 갈라진 지점과 끝만 줄로 보여줌.
	import { GitBranch } from "@lucide/svelte";
	import { relative } from "$shared/time";
	import { familyOf, pathOf, type BranchNode } from "./branches";
	import type { Conversation } from "./library.svelte";

	let { conv, onopen }: { conv: Conversation; onopen: (id: string, jumpTo?: string) => void } = $props();

	const ROW = 34;
	const LANE = 16;

	interface Lane {
		node: BranchNode;
		lane: number;
		parentLane: number | null;
		start: number;
		end: number;
		/** 끝이 놓일 줄. 새 메시지 없이 갈라지기만 했으면 갈라진 줄 바로 아래 (start + 0.5) */
		endRow: number;
	}

	const tree = $derived(familyOf(conv));
	const onPath = $derived(new Set(pathOf(conv).flatMap((s) => (s.conv ? [s.conv.id] : []))));
	const orphan = $derived(!pathOf(conv)[0].conv);

	// 레인 배치: 깊이 우선 순서대로 왼쪽부터.
	// 갈래의 갈래가 부모가 갈라지기 전 메시지에서 나왔으면 그 메시지는 위쪽 대화와 같은 내용이므로,
	// 그 메시지를 실제로 가진 가장 가까운 조상 레인에서 갈라져 나오게 그린다.
	const lanes = $derived.by(() => {
		const out: Lane[] = [];
		const walk = (n: BranchNode, chain: number[]) => {
			const lane = out.length;
			const start = n.at?.seq ?? 1;
			let parentLane: number | null = null;
			for (let i = chain.length - 1; i >= 0; i--) {
				parentLane = chain[i];
				if (out[chain[i]].start < start) break;
			}
			const end = Math.max(n.conv.messageCount, start);
			out.push({ node: n, lane, parentLane, start, end, endRow: end === start && parentLane !== null ? start + 0.5 : end });
			for (const c of n.children) walk(c, [...chain, lane]);
		};
		walk(tree, []);
		return out;
	});
	// 보여줄 줄: 1번, 갈라진 메시지, 각 대화의 끝
	const rows = $derived([...new Set([1, ...lanes.flatMap((l) => [l.start, l.endRow])])].sort((a, b) => a - b));
	const rowOf = $derived(new Map(rows.map((n, i) => [n, i])));
	const y = (n: number) => (rowOf.get(n) ?? 0) * ROW + ROW / 2;
	const x = (lane: number) => lane * LANE + LANE / 2;
	const width = $derived(lanes.length * LANE);
	const height = $derived(rows.length * ROW);

	/** 갈래 선: 부모 레인의 갈라진 지점에서 휘어져 자기 레인으로, 끝까지 */
	function lanePath(l: Lane) {
		const ys = y(l.start);
		const ye = y(l.endRow);
		if (l.parentLane === null) return `M ${x(l.lane)} ${ys} V ${ye}`;
		const xp = x(l.parentLane);
		const xc = x(l.lane);
		const bend = Math.min(ROW * 0.6, ye - ys);
		return `M ${xp} ${ys} C ${xp} ${ys + bend * 0.75}, ${xc} ${ys + bend * 0.25}, ${xc} ${ys + bend} V ${ye}`;
	}
	const endY = (l: Lane) => y(l.endRow);
	const tone = (l: Lane) => (l.node.conv.id === conv.id ? "current" : onPath.has(l.node.conv.id) ? "path" : "other");

	/** 줄마다: 여기서 끝나는 대화, 여기서 갈라진 갈래 */
	const rowInfo = $derived(
		rows.map((n) => ({
			n,
			ends: lanes.filter((l) => l.endRow === n),
			forks: lanes.filter((l) => l.parentLane !== null && l.start === n),
		})),
	);
	const parentOf = (l: Lane) => (l.parentLane === null ? null : lanes[l.parentLane]);
</script>

<div class="panel-body">
	<section>
		<h4>분기 <span class="faint">대화 {lanes.length}개</span></h4>
		{#if orphan}<p class="note">갈라져 나온 원래 대화는 지워졌음</p>{/if}
		{#if lanes.length > 1}
			<div class="graph" style:--lanes-w="{width}px">
				<svg class="lines" {width} {height} viewBox="0 0 {width} {height}" aria-hidden="true">
					{#each lanes as l (l.node.conv.id)}
						<path d={lanePath(l)} class="lane {tone(l)}" />
					{/each}
					{#each lanes as l (l.node.conv.id)}
						{#if l.parentLane !== null}
							<circle cx={x(l.parentLane)} cy={y(l.start)} r="3.5" class="fork {tone(lanes[l.parentLane])}" />
						{/if}
						<circle cx={x(l.lane)} cy={endY(l)} r={l.node.conv.id === conv.id ? 5.5 : 4.5} class="end {tone(l)}" />
					{/each}
				</svg>
				<ol class="rows">
					{#each rowInfo as r (r.n)}
						<li style:height="{ROW}px" title={Number.isInteger(r.n) ? undefined : "새 메시지 없이 갈라지기만 한 대화"}>
							<span class="no" class:fork={r.forks.length > 0}>{Number.isInteger(r.n) ? `#${r.n}` : ""}</span>
							<span class="lanes-gap"></span>
							<span class="what">
								{#each r.ends as l (l.node.conv.id)}
									<button
										class="end-btn {tone(l)}"
										aria-current={l.node.conv.id === conv.id ? "page" : undefined}
										onclick={() => onopen(l.node.conv.id)}
										title="{l.node.conv.title} · 메시지 {l.node.conv.messageCount}개 · {relative(l.node.conv.updatedAt)}"
									>
										<b>{l.node.conv.title}</b>{#if l.node.conv.id === conv.id}<em>지금</em>{/if}
									</button>
								{/each}
								{#if r.forks.length && !r.ends.length}
									{@const p = parentOf(r.forks[0])}
									<button class="fork-btn" onclick={() => p && onopen(p.node.conv.id, r.forks[0].node.at?.messageId)} title="갈라진 메시지로 가기">
										<GitBranch />갈래 {r.forks.length}개
									</button>
								{/if}
							</span>
						</li>
					{/each}
				</ol>
			</div>
			<p class="hint">왼쪽 번호는 메시지 번호. 큰 점은 대화의 끝, 작은 점은 갈라진 메시지. 사이 구간은 접어 둠</p>
		{:else}
			<div class="panel-none">
				<GitBranch />
				<p>아직 갈래 없음. 메시지 위 도구의 '여기서 분기 나누기'로 다른 전개를 따로 이어 갈 수 있음</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.graph {
		position: relative;
	}
	.lines {
		position: absolute;
		top: 0;
		left: 40px;
		overflow: visible;
		pointer-events: none;
	}
	.lane {
		fill: none;
		stroke-width: 2;
		stroke: var(--border-strong);
	}
	.lane.path {
		stroke: color-mix(in oklab, var(--accent) 60%, var(--border-strong));
	}
	.lane.current {
		stroke: var(--accent);
		stroke-width: 2.5;
	}
	.end,
	.fork {
		stroke-width: 2;
		fill: var(--bg);
		stroke: var(--border-strong);
	}
	.end.path,
	.fork.path,
	.fork.current {
		stroke: var(--accent);
	}
	.end.current {
		fill: var(--accent);
		stroke: var(--accent);
	}
	.rows {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.rows li {
		display: flex;
		align-items: center;
		min-width: 0;
	}
	.no {
		flex: none;
		width: 40px;
		padding-right: 6px;
		text-align: right;
		font-size: var(--text-xs);
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.no.fork {
		color: var(--text-muted);
		font-weight: 600;
	}
	.lanes-gap {
		flex: none;
		width: calc(var(--lanes-w) + 8px);
	}
	.what {
		display: flex;
		flex-direction: column;
		gap: 2px;
		flex: 1;
		min-width: 0;
	}
	.end-btn,
	.fork-btn {
		display: flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		padding: 2px 6px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	.end-btn b {
		font-weight: 500;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.end-btn:hover,
	.fork-btn:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.end-btn.path {
		color: var(--text);
	}
	.end-btn.current {
		background: var(--accent-soft);
		color: var(--text);
	}
	.end-btn.current b {
		font-weight: 600;
	}
	.end-btn em {
		flex: none;
		font-style: normal;
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--accent-text);
	}
	.fork-btn {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.fork-btn :global(svg) {
		width: 12px;
		height: 12px;
	}
	.note,
	.hint {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.hint {
		color: var(--text-faint);
	}
	.panel-none :global(svg) {
		width: 22px;
		height: 22px;
	}
	.panel-none p {
		margin: var(--space-2) 0 0;
	}
</style>
