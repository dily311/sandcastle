<script lang="ts">
	import { onMount } from "svelte";
	import { KeyRound, Plus, Zap } from "@lucide/svelte";
	import { apiLists } from "$/data/aiModelList";
	import { Badge, Button, Dialog, EmptyState, Field, Input, Select } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { library, type ApiProfiles } from "$features/character/library.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	const src = { source: "api" };
	let keys = $state<Record<string, boolean>>({});
	let testing = $state<string | null>(null);

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? null : library.load()]);
		await loadKeys();
	});
	async function loadKeys() {
		const pairs = await Promise.all(library.apis.map(async (a) => [a.id, await call(commands.hasApiKey(a.id), { ...src, silent: true }).catch(() => false)] as const));
		keys = Object.fromEntries(pairs);
	}

	// ---- 편집 ----
	interface Draft {
		id: string | null;
		provider: string;
		name: string;
		baseUrl: string;
		model: string;
		key: string;
	}
	let draft = $state<Draft | null>(null);
	let open = $state(false);
	let models = $state<string[]>([]);
	let loadingModels = $state(false);
	let saving = $state(false);

	const provider = $derived(apiLists.find((p) => p.provider === draft?.provider));
	const knownModels = $derived([...new Set([...(provider?.modelList.flatMap((g) => g.model) ?? []), ...models])]);

	function startNew() {
		const p = apiLists[0];
		draft = { id: null, provider: p.provider, name: p.label, baseUrl: p.baseUrl, model: p.modelList[0]?.model[0] ?? "", key: "" };
		models = [];
		open = true;
	}
	function startEdit(a: ApiProfiles) {
		draft = { id: a.id, provider: a.provider, name: a.name, baseUrl: a.baseUrl, model: a.model, key: "" };
		models = [];
		open = true;
	}
	function changeProvider(id: string) {
		if (!draft) return;
		const p = apiLists.find((x) => x.provider === id);
		if (!p) return;
		const prev = apiLists.find((x) => x.provider === draft!.provider);
		// 이름을 손대지 않았으면 공급자 이름을 따라감
		if (!draft.name || draft.name === prev?.label) draft.name = p.label;
		draft.provider = p.provider;
		draft.baseUrl = p.baseUrl;
		draft.model = p.modelList[0]?.model[0] ?? "";
		models = [];
	}
	async function fetchModels() {
		if (!draft?.id) return;
		loadingModels = true;
		try {
			models = await call(commands.getModels(draft.id), src);
			toasts.success(`모델 ${models.length}개 불러옴`);
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			loadingModels = false;
		}
	}
	async function save() {
		if (!draft || !provider || !draft.model.trim()) return;
		saving = true;
		const input = { name: draft.name.trim() || provider.label, provider: provider.provider, formatType: provider.formatType, baseUrl: draft.baseUrl.trim(), model: draft.model.trim() };
		try {
			if (draft.id) {
				await call(commands.updateApiProfile(draft.id, input, draft.key), src);
				const a = library.apis.find((x) => x.id === draft!.id);
				if (a) Object.assign(a, input, { updatedAt: Date.now() });
				if (draft.key) keys[draft.id] = true;
			} else {
				const created = await call(commands.createApiProfile(input, draft.key), src);
				library.apis = [...library.apis, created];
				keys[created.id] = !!draft.key;
				// 첫 연결이면 기본값으로
				if (!appSetting.value?.selectedApiProfile) await appSetting.update({ selectedApiProfile: created.id });
			}
			open = false;
			toasts.success("저장함");
		} finally {
			saving = false;
		}
	}
	async function removeKey() {
		if (!draft?.id) return;
		await call(commands.deleteApiKey(draft.id), src);
		keys[draft.id] = false;
		toasts.success("키 지움");
	}

	async function test(a: ApiProfiles) {
		testing = a.id;
		try {
			const reply = await call(commands.testMessage(a.id), src);
			toasts.success(`연결됨 · ${reply.trim().slice(0, 40) || "(빈 응답)"}`);
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			testing = null;
		}
	}
	async function remove(a: ApiProfiles) {
		if (!(await confirmAction({ title: "연결 삭제", message: `'${a.name}' 연결과 저장된 키를 지움`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteApiProfile(a.id), src);
		library.apis = library.apis.filter((x) => x.id !== a.id);
		if (appSetting.value?.selectedApiProfile === a.id) await appSetting.update({ selectedApiProfile: library.apis[0]?.id ?? null });
	}
</script>

<Section title="API 키" lead="AI 공급자 연결. 키는 운영체제 보안 저장소에 따로 보관함">
	{#snippet actions()}<Button size="sm" variant="primary" onclick={startNew}>{#snippet icon()}<Plus />{/snippet}연결 추가</Button>{/snippet}
	{#if library.apis.length}
		<Group>
			{#each library.apis as a (a.id)}
				<Row label={a.name} desc="{a.provider} · {a.model}">
					{#if appSetting.value?.selectedApiProfile === a.id}<Badge tone="accent">기본</Badge>{/if}
					{#if keys[a.id] === false}<Badge tone="warning"><KeyRound /> 키 없음</Badge>{/if}
					<Button size="sm" variant="ghost" onclick={() => remove(a)}>삭제</Button>
					<Button size="sm" onclick={() => startEdit(a)}>편집</Button>
					<Button size="sm" variant="soft" loading={testing === a.id} disabled={testing !== null} onclick={() => test(a)}>{#snippet icon()}<Zap />{/snippet}테스트</Button>
				</Row>
			{/each}
		</Group>
	{:else}
		<Group flush>
			<EmptyState title="연결 없음" description="AI와 대화하려면 연결을 하나 추가해야 함">
				{#snippet action()}<Button variant="primary" onclick={startNew}>연결 추가</Button>{/snippet}
			</EmptyState>
		</Group>
	{/if}
</Section>

<Dialog bind:open title={draft?.id ? "연결 편집" : "연결 추가"}>
	{#if draft}
		<div class="form">
			<Field label="공급자">
				<Select value={draft.provider} options={apiLists.map((p) => ({ value: p.provider, label: p.label }))} disabled={!!draft.id} onchange={(e) => changeProvider(e.currentTarget.value)} />
			</Field>
			<Field label="이름"><Input bind:value={draft.name} placeholder={provider?.label} /></Field>
			<Field label="모델" hint={draft.id ? "목록에 없으면 직접 입력" : "저장한 뒤 편집에서 공급자의 모델 목록을 불러올 수 있음"}>
				<div class="inline">
					<Input bind:value={draft.model} list="api-models" placeholder="모델 이름" />
					{#if draft.id}<Button loading={loadingModels} onclick={fetchModels}>목록 불러오기</Button>{/if}
				</div>
				<datalist id="api-models">
					{#each knownModels as m (m)}<option value={m}></option>{/each}
				</datalist>
			</Field>
			<Field label="API 키" hint={draft.id ? (keys[draft.id] ? "저장된 키 있음. 비워 두면 그대로 둠" : "저장된 키 없음") : undefined}>
				<div class="inline">
					<Input type="password" bind:value={draft.key} placeholder={draft.id && keys[draft.id] ? "••••••••" : "키 붙여넣기"} autocomplete="off" />
					{#if draft.id && keys[draft.id]}<Button variant="ghost" onclick={removeKey}>키 지우기</Button>{/if}
				</div>
			</Field>
			<Field label="주소 (Base URL)" hint={provider?.formatType === "vertexai" ? "Vertex AI는 프로젝트 주소를 직접 입력" : undefined}>
				<Input bind:value={draft.baseUrl} placeholder="https://" />
			</Field>
		</div>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={() => (open = false)}>취소</Button>
		<Button variant="primary" loading={saving} disabled={!draft?.model.trim()} onclick={save}>저장</Button>
	{/snippet}
</Dialog>

<style>
	.form {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
	}
	.inline {
		display: flex;
		gap: var(--space-2);
	}
	.inline > :global(:first-child) {
		flex: 1;
	}
</style>
