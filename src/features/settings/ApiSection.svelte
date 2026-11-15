<script lang="ts">
	import { onMount } from "svelte";
	import { KeyRound, Plus, Zap } from "@lucide/svelte";
	import { Badge, Button, EmptyState } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
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

	// 편집은 설정 안 페이지로 (ApiEditor)
	const open = (id: string) => nav.go({ page: "settings", tab: "api", edit: id });

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
	{#snippet actions()}<Button size="sm" variant="primary" onclick={() => open("new")}>{#snippet icon()}<Plus />{/snippet}연결 추가</Button>{/snippet}
	{#if library.apis.length}
		<Group>
			{#each library.apis as a (a.id)}
				<Row label={a.name} desc="{a.provider} · {a.model}">
					{#if appSetting.value?.selectedApiProfile === a.id}<Badge tone="accent">기본</Badge>{/if}
					{#if keys[a.id] === false}<Badge tone="warning"><KeyRound /> 키 없음</Badge>{/if}
					<Button size="sm" variant="ghost" onclick={() => remove(a)}>삭제</Button>
					<Button size="sm" onclick={() => open(a.id)}>편집</Button>
					<Button size="sm" variant="soft" loading={testing === a.id} disabled={testing !== null} onclick={() => test(a)}>{#snippet icon()}<Zap />{/snippet}테스트</Button>
				</Row>
			{/each}
		</Group>
	{:else}
		<Group flush>
			<EmptyState title="연결 없음" description="AI와 대화하려면 연결을 하나 추가해야 함"></EmptyState>
		</Group>
	{/if}
</Section>
