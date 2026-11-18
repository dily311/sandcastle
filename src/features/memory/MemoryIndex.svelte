<script lang="ts">
	// 색인 버튼: 먼저 미리보기(요금 없음)를 보여 주고, 확인했을 때만 색인을 돌린다 (ADR-015). 상한을 넘으면 한 번 더 확인
	import type { IndexPreview } from "$ipc/bindings";
	import { Button, Dialog } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { toasts } from "$shared/stores/toast.svelte";

	let { conversationId = null, onDone }: { conversationId?: string | null; onDone?: () => void } = $props();

	const src = { source: "memory" };
	const scope = $derived({ conversationId });
	let open = $state(false);
	let pv = $state<IndexPreview | null>(null);
	let loading = $state(false);
	let running = $state(false);
	let overOk = $state(false);

	async function showPreview() {
		loading = true;
		try {
			pv = await call(commands.previewMemoryIndex(scope), src);
			overOk = false;
			open = true;
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			loading = false;
		}
	}
	async function run() {
		if (!pv) return;
		running = true;
		try {
			const r = await call(commands.runMemoryIndex(scope, overOk), src);
			toasts.success(`색인함: 새로 ${r.embedded}개, 건너뜀 ${r.cached}개`);
			open = false;
		} catch {
			/* 실패해도 쓴 배치는 남음 — 상태에서 확인 */
		} finally {
			running = false;
			onDone?.();
		}
	}
	const nothing = $derived(!!pv && pv.toEmbed === 0);
</script>

<Button {loading} onclick={showPreview}>{conversationId ? "이 대화 색인" : "전체 색인"}</Button>

<Dialog bind:open title="메모리 색인" description="누르기 전에는 아무것도 전송하지 않음">
	{#if pv}
		<ul class="pv">
			<li>대화 {pv.conversations}개 · chunk {pv.chunks}개</li>
			<li>이미 색인되어 건너뜀 {pv.cached}개</li>
			<li><b>새로 임베딩 {pv.toEmbed}개</b> · 입력 약 {pv.estTokens.toLocaleString()}토큰 <span class="faint">(추정)</span></li>
		</ul>
		<p class="faint">대화 글이 설정한 임베딩 공급자에게 전송되고 요금이 들 수 있음. 숨긴 메시지와 메모리에서 뺀 대화는 제외됨. 중간에 멈춰도 저장한 부분은 남고 다음에 이어짐.</p>
		{#if pv.overCap}
			<label class="over">
				<input type="checkbox" bind:checked={overOk} />
				<span>예상 토큰이 실행당 상한({pv.tokenCap.toLocaleString()})을 넘음 — 그래도 진행</span>
			</label>
		{/if}
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" loading={running} disabled={nothing || (!!pv && pv.overCap && !overOk)} onclick={run}>{nothing ? "새로 할 것 없음" : "색인 시작"}</Button>
	{/snippet}
</Dialog>

<style>
	.pv {
		margin: 0 0 var(--space-3);
		padding-left: var(--space-4);
		font-size: var(--text-sm);
	}
	.faint {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	p.faint {
		margin: 0 0 var(--space-3);
	}
	.over {
		display: flex;
		gap: var(--space-2);
		align-items: flex-start;
		font-size: var(--text-sm);
		color: var(--warning);
	}
</style>
