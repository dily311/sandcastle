<script lang="ts">
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { KeyRound, Plus, Zap } from "@lucide/svelte";
	import { Badge, Button, EmptyState } from "$shared/ui";
	import { localizeApiTestReply } from "$lib/utils/backendI18n";
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
	let vertexProjects = $state<Record<string, string>>({});
	let testing = $state<string | null>(null);

	onMount(async () => {
		await Promise.all([appSetting.load(), library.loaded ? library.refreshApis() : library.load()]);
		await loadKeys();
	});
	async function loadKeys() {
		const pairs = await Promise.all(library.apis.map(async (a) => [a.id, await call(commands.hasApiKey(a.id), { ...src, silent: true }).catch(() => false)] as const));
		keys = Object.fromEntries(pairs);
		const vPairs = await Promise.all(
			library.apis
				.filter((a) => a.formatType === "vertexai")
				.map(async (a) => {
					const info = await call(commands.getVertexKeyInfo(a.id), { ...src, silent: true }).catch(() => null);
					return [a.id, info?.projectId ?? ""] as const;
				}),
		);
		vertexProjects = Object.fromEntries(vPairs.filter(([, pid]) => !!pid));
	}

	// 편집은 설정 안 페이지로 (ApiEditor)
	const open = (id: string) => nav.go({ page: "settings", tab: "api", edit: id });

	async function test(a: ApiProfiles) {
		testing = a.id;
		try {
			const reply = localizeApiTestReply(await call(commands.testMessage(a.id, null), src));
			a.verified = true;
			toasts.success($_("settings.more.apiConnected", { values: { reply: reply.slice(0, 40) || $_("settings.more.emptyReply") } }));
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			testing = null;
		}
	}
	async function remove(a: ApiProfiles) {
		if (!(await confirmAction({ title: $_("settings.more.deleteApiTitle"), message: $_("settings.more.deleteApiMsg", { values: { name: a.name } }), confirmLabel: $_("common.delete"), danger: true }))) return;
		await call(commands.deleteApiProfile(a.id), src);
		library.apis = library.apis.filter((x) => x.id !== a.id);
		if (appSetting.value?.selectedApiProfile === a.id) await appSetting.update({ selectedApiProfile: library.apis[0]?.id ?? null });
	}
</script>

<Section title={$_("settings.apiSec.title")} lead={$_("settings.apiSec.lead")}>
	{#snippet actions()}<Button size="sm" variant="primary" onclick={() => open("new")}>{#snippet icon()}<Plus />{/snippet}{$_("settings.apiSec.add")}</Button>{/snippet}
	{#if library.apis.length}
		<Group>
			{#each library.apis as a (a.id)}
				<Row
					class={a.verified ? undefined : "unverified"}
					label={a.name}
					desc={vertexProjects[a.id] ? `${a.provider} · ${vertexProjects[a.id]}` : a.provider}
				>
					{#if appSetting.value?.selectedApiProfile === a.id}<Badge tone="accent">{$_("common.default")}</Badge>{/if}
					{#if keys[a.id] === false}
						<Badge tone="warning"><KeyRound /> {$_("settings.apiSec.noKey")}</Badge>
					{:else if !a.verified}
						<Badge tone="neutral">{$_("settings.apiSec.unverified")}</Badge>
					{/if}
					<Button size="sm" variant="ghost" onclick={() => remove(a)}>{$_("common.delete")}</Button>
					<Button size="sm" onclick={() => open(a.id)}>{$_("common.edit")}</Button>
					<Button size="sm" variant="soft" loading={testing === a.id} disabled={testing !== null} onclick={() => test(a)}>{#snippet icon()}<Zap />{/snippet}{$_("settings.apiSec.test")}</Button>
				</Row>
			{/each}
		</Group>
	{:else}
		<Group flush>
			<EmptyState title={$_("settings.apiSec.emptyTitle")} description={$_("settings.apiSec.emptyDesc")}></EmptyState>
		</Group>
	{/if}
</Section>

<style>
	:global(.row.unverified .txt) {
		opacity: 0.5;
		transition: opacity 0.15s ease;
	}
	:global(.row.unverified:hover .txt) {
		opacity: 0.78;
	}
</style>
