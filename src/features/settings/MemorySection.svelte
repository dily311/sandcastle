<script lang="ts">
	// 메모리 검색 (3-C, ADR-015): 임베딩 설정, 색인, 정리, 뜻으로 찾기
	import { onMount } from "svelte";
	import type { MemoryStatus, SearchHit } from "$ipc/bindings";
	import { Button, Input, Select, Switch } from "$shared/ui";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";
	import MemoryIndex from "$features/memory/MemoryIndex.svelte";
	import MemorySearch from "$features/memory/MemorySearch.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { nav } from "$app/nav.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { apiOptions, current, ensureLoaded, orNull, save } from "./pluginSettingForm.svelte";

	onMount(() => {
		ensureLoaded();
		void refresh();
	});
	const cur = $derived(current());
	let status = $state<MemoryStatus | null>(null);
	const src = { source: "memory" };

	async function refresh() {
		status = await call(commands.getMemoryStatus(), { ...src, silent: true }).catch(() => null);
	}
	const SCOPES = [
		{ value: "conversation", label: "이 대화만" },
		{ value: "character", label: "같은 캐릭터의 대화" },
		{ value: "all", label: "모든 대화" },
	];
	const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
	const int = (v: string, min: number, fallback: number) => Math.max(min, Math.floor(Number(v)) || fallback);

	async function purge(kind: "other" | "orphans" | "all") {
		const msg = { other: "다른 모델로 색인된 것을 지움", orphans: "없어진 대화의 색인을 지움", all: "메모리 색인 파일을 전부 지움 (대화는 그대로, 다시 색인하면 됨)" }[kind];
		if (!(await confirmAction({ title: "메모리 정리", message: msg, confirmLabel: "지우기" }))) return;
		const n = await call(commands.purgeMemory(kind), src);
		toasts.success(kind === "all" ? "지움" : `chunk ${n}개 정리`);
		await refresh();
	}
	function open(h: SearchHit) {
		void nav.go({ page: "character", convId: h.conversationId });
	}
</script>

<Section title="메모리 검색" lead="지난 대화를 뜻으로 찾음. 색인과 검색은 버튼을 눌렀을 때만 돌고, 대화 글이 임베딩 공급자에게 전송되어 요금이 들 수 있음">
	{#if cur}
		<Group title="임베딩 연결">
			<Row label="임베딩 API" desc="채팅 API와 따로 고름. 비워 두면 색인·검색을 할 수 없음">
				<Select value={cur.memory.apiProfileId ?? ""} options={apiOptions("고르세요")} onchange={(e) => save("memory", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label="모델 이름" desc="예: text-embedding-3-small, gemini-embedding-001. 직접 입력">
				<Input value={cur.memory.model} placeholder="임베딩 모델" onchange={(e) => save("memory", { model: e.currentTarget.value.trim() })} />
			</Row>
			<Row label="차원" desc="줄이기를 지원하는 모델만. 비우면 모델 기본 (작을수록 저장·검색이 가벼움)">
				<Input type="number" min="1" value={cur.memory.dimensions ? String(cur.memory.dimensions) : ""} onchange={(e) => save("memory", { dimensions: e.currentTarget.value ? int(e.currentTarget.value, 1, 768) : null })} />
			</Row>
			<Row label="실행당 토큰 상한" desc="한 번 색인에서 임베딩할 예상 입력 토큰. 넘으면 한 번 더 확인">
				<Input type="number" min="1000" value={String(cur.memory.tokenCap)} onchange={(e) => save("memory", { tokenCap: int(e.currentTarget.value, 1000, 200000) })} />
			</Row>
			<Row label="접두어" desc="e5·nomic 같은 모델이 요구할 때만. 바꾸면 다시 색인">
				<div class="pair">
					<Input value={cur.memory.docPrefix} placeholder="passage: " onchange={(e) => save("memory", { docPrefix: e.currentTarget.value })} />
					<Input value={cur.memory.queryPrefix} placeholder="query: " onchange={(e) => save("memory", { queryPrefix: e.currentTarget.value })} />
				</div>
			</Row>
		</Group>
		<Group title="색인">
			<Row label="전체 대화 색인" desc="미리보기에서 대화 수·예상 토큰을 확인한 뒤 시작. 대화마다 '이 대화 색인'은 대화 오른쪽 패널에도 있음">
				<MemoryIndex onDone={refresh} />
			</Row>
			{#if status}
				<Row label="상태" desc="벡터 파일은 앱 데이터 폴더의 memory.db (백업·동기화에는 들어가지 않음)">
					<span class="stat">지금 설정 {status.chunksCurrent}개 · 다른 모델 {status.chunksOther}개 · {mb(status.fileBytes)}MB</span>
				</Row>
			{/if}
			<Row label="정리" desc="지운 대화의 색인과 다른 모델의 색인을 지우거나, 파일을 통째로 지움">
				<div class="pair">
					<Button size="sm" onclick={() => purge("other")}>다른 모델 지우기</Button>
					<Button size="sm" onclick={() => purge("orphans")}>없어진 대화 정리</Button>
					<Button size="sm" variant="danger" onclick={() => purge("all")}>전부 지우기</Button>
				</div>
			</Row>
		</Group>
		<Group title="보낼 때 자동으로 떠올리기">
			<Row label="자동 회상" desc="메시지를 보낼 때마다 지난 대화에서 관련된 부분을 찾아 프롬프트에 넣음. 보낼 때마다 질의 임베딩 1회 요금이 자동 실행으로 기록되고, 예산 한도에 걸리면 기억 없이 보냄. 색인이 있어야 함">
				<Switch checked={cur.memory.autoRecall} label="자동 회상" hideLabel onchange={(v) => save("memory", { autoRecall: v })} />
			</Row>
			{#if cur.memory.autoRecall}
				<Row label="어디서 찾을까" desc="'전체'는 다른 캐릭터·페르소나와 나눈 대화도 이 대화의 프롬프트에 들어갈 수 있음. 메모리에서 뺀 대화는 어느 쪽이든 제외">
					<Select value={cur.memory.recallScope} options={SCOPES} onchange={(e) => save("memory", { recallScope: e.currentTarget.value })} />
				</Row>
				<Row label="넣는 글 상한 (토큰)" desc="한 번에 프롬프트에 넣는 최대 분량. chunk 하나가 약 1000자라 너무 작으면 앞부분만 들어감">
					<Input type="number" min="100" value={String(cur.memory.recallTokens)} onchange={(e) => save("memory", { recallTokens: int(e.currentTarget.value, 100, 1500) })} />
				</Row>
				<Row label="최대 chunk 수" desc="튀는 것만 남기므로 보통 이보다 적게 들어감">
					<Input type="number" min="1" max="10" value={String(cur.memory.recallTopK)} onchange={(e) => save("memory", { recallTopK: Math.min(10, int(e.currentTarget.value, 1, 4)) })} />
				</Row>
			{/if}
		</Group>
		<Group title="뜻으로 찾기">
			<MemorySearch onopen={open} />
		</Group>
	{/if}
</Section>

<style>
	.pair {
		display: flex;
		gap: var(--space-2);
		flex-wrap: wrap;
	}
	.stat {
		font-size: var(--text-sm);
		color: var(--text-muted);
	}
</style>
