<script lang="ts">
	import { _ } from "svelte-i18n";
	// 오른쪽 패널 '분기' 탭: 갈래를 타임라인으로 (git 그래프처럼)
	// 세로축 = 메시지 번호, 대화마다 한 줄(레인). 갈라진 메시지에서 옆 레인으로 휘어져 나감.
	// 아무 일도 없는 메시지 구간은 접어서, 갈라진 지점과 끝만 줄로 보여줌.
	import { ArrowUpRight, GitBranch } from "@lucide/svelte";
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
	const rows = $derived([...new Set([1, ...lanes.flatMap((l) => [l.start, l.endRow])])].sort((a, b) => a - b));
	const rowOf = $derived(new Map(rows.map((n, i) => [n, i])));
	const y = (n: number) => (rowOf.get(n) ?? 0) * ROW + ROW / 2;
	const x = (lane: number) => lane * LANE + LANE / 2;
	const width = $derived(lanes.length * LANE);
	const height = $derived(rows.length * ROW);

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

	const rowInfo = $derived(
		rows.map((n) => ({
			n,
			ends: lanes.filter((l) => l.endRow === n),
			forks: lanes.filter((l) => l.parentLane !== null && l.start === n),
		})),
	);
	const parentOf = (l: Lane) => (l.parentLane === null ? null : lanes[l.parentLane]);

	// ---- 원 아이콘 호버 시 파일(대화)별 이동 팝오버 ----
	interface HoverCard {
		id: string;
		x: number;
		y: number;
		heading: string;
		items: { conv: Conversation; jumpTo?: string; current: boolean }[];
	}
	let hovered = $state<HoverCard | null>(null);
	let hoverLaneId = $state<string | null>(null);
	let hideTimer: ReturnType<typeof setTimeout> | undefined;

	function keepHover() {
		clearTimeout(hideTimer);
	}
	function scheduleHide() {
		clearTimeout(hideTimer);
		hideTimer = setTimeout(() => {
			hovered = null;
			hoverLaneId = null;
		}, 180);
	}
	function showEndHover(l: Lane) {
		clearTimeout(hideTimer);
		hoverLaneId = l.node.conv.id;
		hovered = {
			id: `end:${l.node.conv.id}`,
			x: 40 + x(l.lane),
			y: endY(l),
			heading: $_("branchTree.msgHeading", { values: { num: l.end } }),
			items: [{ conv: l.node.conv, current: l.node.conv.id === conv.id }],
		};
	}
	function showForkHover(parentLaneIdx: number, startRow: number) {
		clearTimeout(hideTimer);
		const parent = lanes[parentLaneIdx];
		const children = lanes.filter((l) => l.parentLane === parentLaneIdx && l.start === startRow);
		hoverLaneId = parent?.node.conv.id ?? null;
		const items: HoverCard["items"] = [
			...(parent ? [{ conv: parent.node.conv, jumpTo: children[0]?.node.at?.messageId, current: parent.node.conv.id === conv.id }] : []),
			...children.map((c) => ({ conv: c.node.conv, current: c.node.conv.id === conv.id })),
		];
		hovered = {
			id: `fork:${parentLaneIdx}:${startRow}`,
			x: 40 + x(parentLaneIdx),
			y: y(startRow),
			heading: $_("branchTree.forkHeading", { values: { row: startRow, count: children.length } }),
			items,
		};
	}
</script>

<div class="panel-body">
	<section>
		<h4>{$_("branchTree.title")} <span class="faint">{$_("branchTree.convCount", { values: { count: lanes.length } })}</span></h4>
		{#if orphan}<p class="note">{$_("branchTree.orphanNote")}</p>{/if}
		{#if lanes.length > 1}
			<p class="hint">{$_("branchTree.hoverHint")}</p>
			<div class="graph-scroll">
				<div class="graph" style:--lanes-w="{width}px">
					<svg class="lines" {width} {height} viewBox="0 0 {width} {height}">
						{#each lanes as l (l.node.conv.id)}
							<path d={lanePath(l)} class="lane {tone(l)}" class:hi={hoverLaneId === l.node.conv.id} />
						{/each}
						{#each lanes as l (l.node.conv.id)}
							{#if l.parentLane !== null}
								<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
								<circle
									cx={x(l.parentLane)}
									cy={y(l.start)}
									r="4"
									class="fork {tone(lanes[l.parentLane])} dot-hit"
									onmouseenter={() => showForkHover(l.parentLane!, l.start)}
									onmouseleave={scheduleHide}
									onclick={() => {
										const p = parentOf(l);
										if (p) onopen(p.node.conv.id, l.node.at?.messageId);
									}}
								/>
							{/if}
							<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
							<circle
								cx={x(l.lane)}
								cy={endY(l)}
								r={l.node.conv.id === conv.id ? 5.5 : 4.5}
								class="end {tone(l)} dot-hit"
								class:hi={hoverLaneId === l.node.conv.id}
								onmouseenter={() => showEndHover(l)}
								onmouseleave={scheduleHide}
								onclick={() => onopen(l.node.conv.id)}
							/>
						{/each}
					</svg>
					<ol class="rows">
						{#each rowInfo as r (r.n)}
							<li style:height="{ROW}px" title={Number.isInteger(r.n) ? undefined : $_("branchTree.emptyForkTitle")}>
								<span class="no" class:fork={r.forks.length > 0}>{Number.isInteger(r.n) ? `#${r.n}` : ""}</span>
								<span class="lanes-gap"></span>
								<span class="what">
									{#each r.ends as l (l.node.conv.id)}
										<button
											class="end-btn {tone(l)}"
											class:hi={hoverLaneId === l.node.conv.id}
											aria-current={l.node.conv.id === conv.id ? "page" : undefined}
											onmouseenter={() => (hoverLaneId = l.node.conv.id)}
											onmouseleave={() => (hoverLaneId = null)}
											onclick={() => onopen(l.node.conv.id)}
											title={$_("branchTree.convTitle", { values: { title: l.node.conv.title, count: l.node.conv.messageCount, when: relative(l.node.conv.updatedAt) } })}
										>
											<b>{l.node.conv.title}</b>{#if l.node.conv.id === conv.id}<em>{$_("branchTree.currentBadge")}</em>{/if}
										</button>
									{/each}
									{#if r.forks.length && !r.ends.length}
										{@const p = parentOf(r.forks[0])}
										<button class="fork-btn" onclick={() => p && onopen(p.node.conv.id, r.forks[0].node.at?.messageId)} title={$_("branchTree.jumpForkTitle")}>
											<GitBranch />{$_("branchTree.forkCount", { values: { count: r.forks.length } })}
										</button>
									{/if}
								</span>
							</li>
						{/each}
					</ol>

					{#if hovered}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="dot-pop"
							style:left="{Math.max(8, hovered.x)}px"
							style:top="{hovered.y + 12}px"
							onmouseenter={keepHover}
							onmouseleave={scheduleHide}
						>
							<p class="pop-head">{hovered.heading}</p>
							<ul>
								{#each hovered.items as it (it.conv.id + (it.jumpTo ?? ""))}
									<li>
										<button type="button" class:cur={it.current} onclick={() => ((hovered = null), onopen(it.conv.id, it.jumpTo))}>
											<span class="pt">
												<b>{it.conv.title}</b>
												<small>{$_("branchTree.msgMeta", { values: { count: it.conv.messageCount, when: relative(it.conv.updatedAt) } })}</small>
											</span>
											{#if it.current}<em>{$_("branchTree.currentBadge")}</em>{:else}<ArrowUpRight />{/if}
										</button>
									</li>
								{/each}
							</ul>
						</div>
					{/if}
				</div>
			</div>
		{:else}
			<div class="panel-none">
				<GitBranch />
				<p>{$_("branchTree.emptyState")}</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.graph-scroll {
		overflow-x: auto;
		padding-bottom: var(--space-3);
	}
	.graph {
		position: relative;
		width: max-content;
		min-width: 100%;
	}
	.lines {
		position: absolute;
		top: 0;
		left: 40px;
		overflow: visible;
		pointer-events: none;
		z-index: 2;
	}
	.dot-hit {
		pointer-events: all;
		cursor: pointer;
		transition: r 120ms var(--ease), stroke-width 120ms var(--ease);
	}
	.dot-hit:hover,
	.dot-hit.hi {
		stroke: var(--accent);
		stroke-width: 3;
	}
	.lane {
		fill: none;
		stroke-width: 2;
		stroke: var(--border-strong);
		transition: stroke 120ms var(--ease), stroke-width 120ms var(--ease);
	}
	.lane.path {
		stroke: color-mix(in oklab, var(--accent) 60%, var(--border-strong));
	}
	.lane.current,
	.lane.hi {
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
		min-width: 148px;
		max-width: 240px;
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
	.end-btn.hi,
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
		white-space: nowrap;
	}
	.fork-btn :global(svg) {
		width: 12px;
		height: 12px;
		flex: none;
	}
	.dot-pop {
		position: absolute;
		z-index: 10;
		min-width: 180px;
		max-width: 240px;
		padding: 6px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
	}
	.pop-head {
		margin: 0 0 4px;
		padding: 0 6px;
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.dot-pop ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.dot-pop button {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		padding: 5px 6px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-xs);
		text-align: left;
		cursor: pointer;
	}
	.dot-pop button:hover {
		background: var(--surface-hover);
	}
	.dot-pop button.cur {
		background: var(--accent-soft);
	}
	.dot-pop .pt {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
	}
	.dot-pop b {
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.dot-pop small {
		color: var(--text-faint);
		font-size: 11px;
	}
	.dot-pop em {
		flex: none;
		font-style: normal;
		font-weight: 600;
		color: var(--accent-text);
	}
	.dot-pop :global(svg) {
		flex: none;
		width: 13px;
		height: 13px;
		color: var(--text-muted);
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
