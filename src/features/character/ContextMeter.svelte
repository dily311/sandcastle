<script lang="ts">
	// 컨텍스트 사용량 막대: 시스템(규칙·캐릭터·월드인포·노트) / 대화 기록 / 응답 몫 / 남은 칸
	import type { ContextUsage } from "$ipc/bindings";

	let { usage }: { usage: ContextUsage | null } = $props();

	const n = (v: number) => v.toLocaleString();
	/** 메모리 회상을 건너뛴 이유 (백엔드 service/memory/recall.rs의 코드) */
	const RECALL_REASON: Record<string, string> = {
		short: "입력이 너무 짧음",
		no_index: "색인이 없음",
		other_model: "다른 모델로 색인되어 있음 (다시 색인 필요)",
		no_candidates: "찾을 만한 지난 내용이 없음 (최근 대화는 이미 프롬프트에 있음)",
		no_relevant: "관련 있어 보이는 것이 없음",
		timeout: "질의 임베딩이 오래 걸려 기억 없이 보냄",
		budget: "예산 한도로 막힘 (자동 실행은 한도를 넘지 않음)",
		cancelled: "취소됨",
		error: "질의 임베딩 실패 (기억 없이 보냄, 다시 시도하지 않음)",
		regenerate: "다시 만들기: 이전에 떠올린 기억이 없었음",
	};
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
		{#if usage.memoryRecall}
			<!-- 자동 회상 (3-C-5): 지난 대화에서 가져온 글. 대화 몫 안에 들어 있다 -->
			{#if usage.memoryRecall.status === "used"}
				<p class="faint small plugins">메모리에서 가져옴: {usage.memoryRecall.hits.map((h) => `${h.conversationTitle || "(제목 없음)"} ${n(h.tokens)}`).join(" · ")} <small>(대화에 포함 · 합계 {n(usage.memoryRecall.tokens)})</small></p>
			{:else}
				<p class="faint small plugins">메모리 회상 건너뜀: {RECALL_REASON[usage.memoryRecall.reason ?? ""] ?? usage.memoryRecall.reason}</p>
			{/if}
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
