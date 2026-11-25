<script lang="ts">
	// 메모리 검색 (3-C, ADR-015): 임베딩 설정, 색인, 정리, 뜻으로 찾기
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
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
	import { library } from "$features/character/library.svelte";
	import { embeddingModelInfo, embeddingModelsFor } from "./providers";
	import Slider from "$/shared/ui/Slider.svelte";

	onMount(() => {
		ensureLoaded();
		void refresh();
	});
	const cur = $derived(current());
	// 임베딩 모델 추천: 고른 임베딩 API의 공급자에 맞는 내장 목록 (직접 입력도 됨)
	const embedProfile = $derived(library.apis.find((a) => a.id === cur?.memory.apiProfileId));
	const embedChoices = $derived(embeddingModelsFor(embedProfile?.provider));
	const embedInfo = $derived(embeddingModelInfo(cur?.memory.model ?? ""));
	const nf = (n: number) => n.toLocaleString();
	/** 고른 모델에 대해 앱이 아는 것 (기본 차원 · 최대 입력 · 가격) */
	const embedFacts = $derived(
		embedInfo
			? [
					embedInfo.dimensions ? $_("settings.more.embedDefaultDim", { values: { dim: nf(embedInfo.dimensions) } }) : "",
					embedInfo.maxInputTokens ? $_("settings.more.embedMaxInput", { values: { tokens: nf(embedInfo.maxInputTokens) } }) : "",
					embedInfo.price != null ? $_("settings.more.embedPrice", { values: { price: embedInfo.price } }) : "",
				]
					.filter(Boolean)
					.join(" · ")
			: "",
	);
	/** 상한 슬라이더를 끄는 동안의 값 (놓으면 저장) */
	let capDraft = $state<number | null>(null);
	let status = $state<MemoryStatus | null>(null);
	const src = { source: "memory" };

	async function refresh() {
		status = await call(commands.getMemoryStatus(), { ...src, silent: true }).catch(() => null);
	}
	const SCOPES = $derived([
		{ value: "conversation", label: $_("settings.more.memScopeConv") },
		{ value: "character", label: $_("settings.more.memScopeChar") },
		{ value: "all", label: $_("settings.more.memScopeAll") },
	]);
	const mb = (n: number) => (n / 1024 / 1024).toFixed(1);
	const int = (v: string, min: number, fallback: number) => Math.max(min, Math.floor(Number(v)) || fallback);

	async function purge(kind: "other" | "orphans" | "all") {
		const msg = {
			other: $_("settings.more.memPurgeOther"),
			orphans: $_("settings.more.memPurgeOrphans"),
			all: $_("settings.more.memPurgeAll"),
		}[kind];
		if (!(await confirmAction({ title: $_("settings.more.memPurgeTitle"), message: msg, confirmLabel: $_("settings.more.memPurgeBtn") }))) return;
		const n = await call(commands.purgeMemory(kind), src);
		toasts.success(kind === "all" ? $_("settings.more.deletedToast") : $_("settings.more.memPurgedChunks", { values: { count: n } }));
		await refresh();
	}
	function open(h: SearchHit) {
		void nav.go({ page: "character", convId: h.conversationId });
	}
</script>

<Section title={$_("settings.memorySec.title")} lead={$_("settings.memorySec.lead")}>
	{#if cur}
		<Group title={$_("settings.memorySec.embedGroup")}>
			<Row label={$_("settings.memorySec.embedApi")} desc={$_("settings.memorySec.embedApiDesc")}>
				<Select value={cur.memory.apiProfileId ?? ""} options={apiOptions($_("settings.chatSec.notSelected"))} onchange={(e) => save("memory", { apiProfileId: orNull(e.currentTarget.value) })} />
			</Row>
			<Row label={$_("settings.memorySec.modelName")} desc={embedFacts || $_("settings.memorySec.modelPlaceholder")}>
				<Input value={cur.memory.model} list="embed-models" placeholder={$_("settings.memorySec.modelPlaceholder")} onchange={(e) => save("memory", { model: e.currentTarget.value.trim() })} />
				<datalist id="embed-models">
					{#each embedChoices as m (m.modelId)}<option value={m.modelId} label={m.label}></option>{/each}
				</datalist>
			</Row>
			<Row
				label={$_("settings.memorySec.dimensions")}
				desc={embedInfo
					? embedInfo.adjustable
						? `(${embedInfo.supportedDimensions ? embedInfo.supportedDimensions.map(nf).join(" · ") : ""})`
						: ""
					: ""}
			>
				<Input type="number" min="1" placeholder={embedInfo?.dimensions ? String(embedInfo.dimensions) : ""} value={cur.memory.dimensions ? String(cur.memory.dimensions) : ""} onchange={(e) => save("memory", { dimensions: e.currentTarget.value ? int(e.currentTarget.value, 1, 768) : null })} />
			</Row>
			<Row label={$_("settings.memorySec.tokenCap")} desc={$_("settings.memorySec.tokenCapDesc")}>
				<div class="cap">
					<input
						type="range"
						min="1000"
						max="200000"
						step="1000"
						aria-label={$_("settings.memorySec.tokenCap")}
						value={capDraft ?? cur.memory.tokenCap}
						style:--pct="{(((capDraft ?? cur.memory.tokenCap) - 1000) / 199000) * 100}%"
						oninput={(e) => (capDraft = Number(e.currentTarget.value))}
						onchange={(e) => {
							void save("memory", { tokenCap: int(e.currentTarget.value, 1000, 200000) });
							capDraft = null;
						}}
					/>
					<b>{(capDraft ?? cur.memory.tokenCap).toLocaleString()}</b>
				</div>
			</Row>
			<Row label={$_("settings.memorySec.prefix")} desc={$_("settings.memorySec.prefixDesc")}>
				<div class="pair">
					<Input value={cur.memory.docPrefix} placeholder="passage: " onchange={(e) => save("memory", { docPrefix: e.currentTarget.value })} />
					<Input value={cur.memory.queryPrefix} placeholder="query: " onchange={(e) => save("memory", { queryPrefix: e.currentTarget.value })} />
				</div>
			</Row>
		</Group>
		<Group title={$_("settings.memorySec.indexGroup")}>
			<Row label={$_("settings.memorySec.indexAll")} desc={$_("settings.memorySec.indexAllDesc")}>
				<MemoryIndex onDone={refresh} />
			</Row>
			{#if status}
				<Row label={$_("settings.memorySec.status")} desc={$_("settings.memorySec.statusDesc")}>
					<span class="stat">{$_("settings.more.memStat", { values: { current: status.chunksCurrent, other: status.chunksOther, mb: mb(status.fileBytes) } })}</span>
				</Row>
			{/if}
			<Row label={$_("settings.more.memCleanup")} desc={$_("settings.more.memCleanupDesc")}>
				<div class="pair">
					<Button size="sm" onclick={() => purge("other")}>{$_("settings.more.memPurgeOtherBtn")}</Button>
					<Button size="sm" onclick={() => purge("orphans")}>{$_("settings.more.memPurgeOrphansBtn")}</Button>
					<Button size="sm" variant="danger" onclick={() => purge("all")}>{$_("settings.more.memPurgeAllBtn")}</Button>
				</div>
			</Row>
		</Group>
		<Group title={$_("settings.more.memAutoRecallGroup")}>
			<Row label={$_("settings.more.memAutoRecall")} desc={$_("settings.more.memAutoRecallDesc")} nowrap>
				<Switch checked={cur.memory.autoRecall} label={$_("settings.more.memAutoRecall")} hideLabel onchange={(v) => save("memory", { autoRecall: v })} />
			</Row>
			{#if cur.memory.autoRecall}
				<Row label={$_("settings.more.memRecallScope")} desc={$_("settings.more.memRecallScopeDesc")}>
					<Select value={cur.memory.recallScope} options={SCOPES} onchange={(e) => save("memory", { recallScope: e.currentTarget.value })} />
				</Row>
				<Row label={$_("settings.more.memRecallTokens")} desc={$_("settings.more.memRecallTokensDesc")}>
					<Input type="number" min="100" value={String(cur.memory.recallTokens)} onchange={(e) => save("memory", { recallTokens: int(e.currentTarget.value, 100, 1500) })} />
				</Row>
				<Row label={$_("settings.more.memRecallTopK")} desc={$_("settings.more.memRecallTopKDesc")}>
					<Input type="number" min="1" max="10" value={String(cur.memory.recallTopK)} onchange={(e) => save("memory", { recallTopK: Math.min(10, int(e.currentTarget.value, 1, 4)) })} />
				</Row>
			{/if}
		</Group>
		<Group title={$_("settings.more.memSearchGroup")}>
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
	.cap {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		min-width: min(260px, 100%);
		width: 100%;
	}
	.cap input[type="range"] {
		flex: 1;
		min-width: 0;
		accent-color: var(--accent);
		cursor: pointer;
	}
	.cap b {
		min-width: 6ch;
		text-align: right;
		font-variant-numeric: tabular-nums;
		font-weight: 600;
	}
</style>
