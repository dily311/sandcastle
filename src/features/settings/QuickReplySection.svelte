<script lang="ts">
	// 빠른 답장 세트 (5-E-20-5-3, ADR-025). 켜짐/꺼짐은 코어 플러그인 slash
	import { onMount } from "svelte";
	import { Download, Plus, Trash2 } from "@lucide/svelte";
	import type { QuickReplySet_Serialize as QuickReplySet } from "$ipc/bindings";
	import { Button, EmptyState, IconButton, Input, Switch, Textarea } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { pickFiles, JSON_FILES } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { pluginSetting } from "$shared/stores/aiTools.svelte";
	import { newId, qrSets, saveSets } from "$features/slash/quickReply";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	// 고치는 동안은 이 사본을 쓰고, 바뀔 때마다 조금 기다렸다 저장
	let sets = $state<QuickReplySet[]>([]);
	let loaded = $state(false);
	onMount(async () => {
		if (!pluginSetting.value) await pluginSetting.load();
		sets = structuredClone($state.snapshot(qrSets())) as QuickReplySet[];
		loaded = true;
	});
	let timer: ReturnType<typeof setTimeout> | undefined;
	function changed() {
		clearTimeout(timer);
		timer = setTimeout(() => void saveSets($state.snapshot(sets) as QuickReplySet[]), 400);
	}

	function addSet() {
		sets.push({ id: newId(), name: "새 세트", enabled: true, autoSend: false, items: [{ id: newId(), label: "", message: "", hidden: false }] });
		changed();
	}
	async function removeSet(i: number) {
		if (!(await confirmAction({ title: "세트 삭제", message: `'${sets[i].name}' 세트와 버튼 ${sets[i].items.length}개를 지움`, confirmLabel: "삭제", danger: true }))) return;
		sets.splice(i, 1);
		changed();
	}
	async function importSt() {
		const [path] = await pickFiles([JSON_FILES], false);
		if (!path) return;
		const [set, warnings] = await call(commands.readQuickReplySet(path), { source: "slash" });
		sets.push(set);
		changed();
		toasts.success(`'${set.name}' 세트를 가져옴 · 버튼 ${set.items.length}개`);
		if (warnings.length) toasts.info(warnings.join(" · "));
	}
</script>

<Section title="빠른 답장" lead="입력칸 위 버튼. /로 시작하는 글은 스크립트로 실행">
	{#snippet actions()}
		<Button size="sm" onclick={importSt}>{#snippet icon()}<Download />{/snippet}SillyTavern에서 가져오기</Button>
		<Button size="sm" variant="primary" onclick={addSet}>{#snippet icon()}<Plus />{/snippet}새 세트</Button>
	{/snippet}
	{#if loaded && !sets.length}
		<Group flush>
			<EmptyState title="세트 없음" description="새로 만들거나 SillyTavern 빠른 답장 세트(.json)를 가져옴"></EmptyState>
		</Group>
	{/if}
	{#each sets as set, i (set.id)}
		<Group>
			<Row label="이름">
				<Input bind:value={set.name} oninput={changed} aria-label="세트 이름" />
				<IconButton label="세트 삭제" size="sm" variant="ghost" onclick={() => removeSet(i)}><Trash2 /></IconButton>
			</Row>
			<Row label="모든 대화에 보이기">
				<Switch checked={set.enabled} label="모든 대화에 보이기" hideLabel onchange={(v) => ((set.enabled = v), changed())} />
			</Row>
			<Row label="글은 바로 보내기" desc="켜면 누를 때 AI 답장 요금이 듦 · 끄면 입력칸에 넣기만">
				<Switch checked={set.autoSend} label="글은 바로 보내기" hideLabel onchange={(v) => ((set.autoSend = v), changed())} />
			</Row>
			<div class="items">
				{#each set.items as q, j (q.id)}
					<div class="item">
						<div class="head">
							<Input bind:value={q.label} oninput={changed} placeholder="버튼 이름" aria-label="버튼 이름" />
							<Switch checked={!q.hidden} label="버튼 줄에 보이기" hideLabel onchange={(v) => ((q.hidden = !v), changed())} />
							<IconButton label="버튼 삭제" size="sm" variant="ghost" onclick={() => (set.items.splice(j, 1), changed())}><Trash2 /></IconButton>
						</div>
						<Textarea bind:value={q.message} oninput={changed} rows={2} autosize maxRows={12} placeholder="보낼 글 또는 /input … | /gen … | /sendas" aria-label="내용" />
					</div>
				{/each}
				<div><Button size="sm" variant="ghost" onclick={() => (set.items.push({ id: newId(), label: "", message: "", hidden: false }), changed())}>{#snippet icon()}<Plus />{/snippet}버튼 추가</Button></div>
			</div>
		</Group>
	{/each}
</Section>

<style>
	.items {
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.item {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding-bottom: var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.head :global(input) {
		flex: 1;
	}
</style>
