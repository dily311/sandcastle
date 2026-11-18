<script lang="ts">
	// 컨텍스트 사용량 막대: 시스템(규칙·캐릭터·월드인포·노트) / 대화 기록 / 응답 몫 / 남은 칸
	import type { ContextUsage } from "$ipc/bindings";

	let { usage }: { usage: ContextUsage | null } = $props();

	const n = (v: number) => v.toLocaleString();
	const pct = (v: number) => (usage && usage.limit ? `${Math.min(100, (v / usage.limit) * 100)}%` : "0%");
	const total = $derived(usage?.used != null ? usage.used + usage.reserved : 0);
	const ratio = $derived(usage && usage.limit ? total / usage.limit : 0);
	const tone = $derived(ratio >= 0.95 ? "danger" : ratio >= 0.8 ? "warn" : "ok");
</script>

<div class="meter" aria-label="컨텍스트 사용량">
	{#if !usage}
		<p class="faint">불러오는 중…</p>
	{:else if usage.used == null}
		<div class="bar"><i class="reserved" style:width={pct(usage.reserved)}></i></div>
		<p class="faint">아직 보낸 적 없음 · 한도 {usage.limit ? `${n(usage.limit)} 토큰` : "제한 없음"}</p>
	{:else}
		<div class="nums">
			<b class={tone}>{n(total)}</b><span>/ {usage.limit ? `${n(usage.limit)} 토큰` : "제한 없음"}</span><em>{Math.round(ratio * 100)}%</em>
		</div>
		<div class="bar" role="img" aria-label="시스템 {usage.system}, 대화 {usage.history}, 응답 몫 {usage.reserved}, 한도 {usage.limit}">
			<i class="system" style:width={pct(usage.system)}></i>
			<i class="history" style:width={pct(usage.history)}></i>
			<i class="reserved" style:width={pct(usage.reserved)}></i>
		</div>
		<ul class="legend">
			<li><span class="dot system"></span>설정·캐릭터 {n(usage.system)}</li>
			<li><span class="dot history"></span>대화 {n(usage.history)} <small>(메시지 {usage.historyCount}개)</small></li>
			{#if usage.reserved}<li><span class="dot reserved"></span>응답 몫 {n(usage.reserved)}</li>{/if}
		</ul>
		{#if usage.plugins?.length}
			<!-- 커뮤니티 플러그인이 더한 블록 (6-D-5). 설정·캐릭터 몫 안에 들어 있다 -->
			<p class="faint small plugins">플러그인이 더함: {usage.plugins.map((p) => `${p.name} ${n(p.tokens)}`).join(" · ")} <small>(설정·캐릭터에 포함)</small></p>
		{/if}
		{#if ratio >= 0.8}<p class="note">한도에 가까우면 오래된 메시지부터 빠짐. 메모에 꼭 남길 내용을 적어 두면 좋음</p>{/if}
	{/if}
	<p class="faint small">마지막으로 보낸 프롬프트 기준{usage?.presetName ? ` · 프리셋 '${usage.presetName}'` : ""}</p>
</div>

<style>
	.meter {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.nums {
		display: flex;
		align-items: baseline;
		gap: var(--space-1);
	}
	.nums b {
		font-size: var(--text-lg);
		font-variant-numeric: tabular-nums;
	}
	.nums span {
		color: var(--text-muted);
	}
	.nums em {
		margin-left: auto;
		font-style: normal;
		color: var(--text-muted);
		font-variant-numeric: tabular-nums;
	}
	.warn {
		color: var(--warning, #c98a1b);
	}
	.danger {
		color: var(--danger, #d9534f);
	}
	.bar {
		display: flex;
		height: 8px;
		border-radius: 4px;
		background: var(--bg-sunken);
		overflow: hidden;
	}
	.bar i {
		display: block;
		height: 100%;
	}
	.system {
		background: color-mix(in oklab, var(--accent) 55%, var(--text-faint));
	}
	.history {
		background: var(--accent);
	}
	.reserved {
		background: repeating-linear-gradient(135deg, var(--border-strong) 0 3px, transparent 3px 6px);
	}
	.legend {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 2px var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.legend li {
		display: flex;
		align-items: center;
		gap: 5px;
	}
	.dot {
		width: 8px;
		height: 8px;
		border-radius: 2px;
	}
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	p {
		margin: 0;
	}
	.small {
		font-size: var(--text-xs);
	}
</style>
