<script lang="ts">
	// 플러그인 기록 (6-D-8): 최근 200개, 메모리에만 있어 앱을 다시 켜면 비어 있다
	import { _ } from "svelte-i18n";
	import type { PluginLogEntry } from "$ipc/bindings";
	import { Badge, Button, Dialog, EmptyState } from "$shared/ui";
	import { admin } from "./admin.svelte";

	let { open = $bindable(false), pluginId, name }: { open?: boolean; pluginId: string; name: string } = $props();
	let entries = $state<PluginLogEntry[]>([]);

	async function load() {
		entries = (await admin.log(pluginId)).slice().reverse();
	}
	$effect(() => {
		if (open) void load();
	});
	const TONE = { info: "neutral", warn: "warning", error: "danger" } as const;
	const time = (ms: number) => new Date(ms).toLocaleTimeString();
</script>

<Dialog bind:open title={$_("communityUi.logTitle", { values: { name } })} description={$_("communityUi.logDesc")} size="md">
	{#if entries.length === 0}
		<EmptyState title={$_("communityUi.logEmpty")} />
	{:else}
		<ul class="log">
			{#each entries as e, i (i)}
				<li>
					<Badge tone={TONE[e.level as keyof typeof TONE] ?? "neutral"}>{e.level}</Badge>
					<time>{time(e.at)}</time>
					<span>{e.message}</span>
				</li>
			{/each}
		</ul>
	{/if}
	{#snippet footer()}
		<Button variant="ghost" onclick={load}>{$_("communityUi.logRefresh")}</Button>
		<Button onclick={() => (open = false)}>{$_("common.close")}</Button>
	{/snippet}
</Dialog>

<style>
	.log {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: var(--space-1);
		font-size: var(--text-sm);
	}
	li {
		display: flex;
		gap: var(--space-2);
		align-items: baseline;
	}
	time {
		color: var(--text-faint);
		font-size: var(--text-xs);
		flex: none;
	}
	span {
		word-break: break-word;
		min-width: 0;
	}
</style>
