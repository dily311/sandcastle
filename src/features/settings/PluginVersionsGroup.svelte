<script lang="ts">
	// 플러그인 판 폴더 (CLAUDE.md 1-F-1, ADR-012 갱신): 쓰지 않는 판의 폴더를 지운다. 쓰는 판은 지울 수 없다
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Trash2 } from "@lucide/svelte";
	import type { UserPlugin_Serialize as UserPlugin } from "$ipc/bindings";
	import { Badge, Button } from "$shared/ui";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { admin } from "$features/community/admin.svelte";
	import { relative } from "$shared/time";
	import Group from "./Group.svelte";

	let plugins = $state<UserPlugin[]>([]);
	let busy = $state<string | null>(null);

	onMount(load);
	async function load() {
		plugins = (await admin.folders()).filter((p) => p.versions.length > 1 || p.versions.some((v) => !v.active));
	}
	const size = (b: number) => (b < 1024 * 1024 ? `${Math.max(1, Math.round(b / 1024))}KB` : `${(b / 1024 / 1024).toFixed(1)}MB`);

	async function remove(p: UserPlugin, dir: string, version: string) {
		const ok = await confirmAction({
			title: $_("settings.pluginVersions.deleteTitle", { values: { name: p.manifest?.name ?? p.id, version } }),
			message: $_("settings.pluginVersions.deleteMsg"),
			danger: true,
			confirmLabel: $_("settings.pluginVersions.deleteBtn"),
		});
		if (!ok) return;
		busy = `${p.id}/${dir}`;
		try {
			await admin.deleteVersion(p.id, dir);
			await load();
		} finally {
			busy = null;
		}
	}
</script>

{#if plugins.length}
	<Group title={$_("settings.pluginVersions.title")}>
		<ul class="items">
			{#each plugins as p (p.id)}
				{#each p.versions as v (v.dir)}
					<li>
						<div class="txt">
							<b>{p.manifest?.name ?? p.id} · v{v.version}</b>
							<span class="meta">{size(v.size)} · {relative(v.modifiedAt)}{v.error ? ` · ${v.error}` : ""}</span>
						</div>
						{#if v.active}
							<Badge tone="accent">{$_("settings.pluginVersions.active")}</Badge>
						{:else}
							<Button size="sm" variant="ghost" loading={busy === `${p.id}/${v.dir}`} disabled={busy !== null} onclick={() => remove(p, v.dir, v.version)}>{#snippet icon()}<Trash2 />{/snippet}{$_("settings.pluginVersions.deleteBtn")}</Button>
						{/if}
					</li>
				{/each}
			{/each}
		</ul>
	</Group>
{/if}

<style>
	.items {
		list-style: none;
		margin: 0;
		padding: 0;
		border-top: 1px solid var(--border);
	}
	.items li {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) 0;
		border-bottom: 1px solid var(--border);
	}
	.txt {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.meta {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
</style>
