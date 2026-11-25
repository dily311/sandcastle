<script lang="ts">
	// 코어 플러그인: 부가 기능을 껐다 켬 (Obsidian의 코어 플러그인처럼). 목록과 기본값은 백엔드 sys/plugins.rs.
	// 기능별 세부 설정은 각자의 설정 탭 (켜져 있을 때만 보임)
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { Settings2 } from "@lucide/svelte";
	import { Badge, IconButton, Switch } from "$shared/ui";
	import { layout } from "$app/layout.svelte";
	import { nav, type SettingsTab } from "$app/nav.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";
	import { localizeCorePlugin } from "$lib/utils/backendI18n";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => void plugins.load());

	const GROUPS = $derived<{ title: string; ids: PluginId[] }[]>([
		{ title: $_("corePlugins.groups.screen"), ids: ["phone", "game", "widget"] },
		{ title: $_("corePlugins.groups.chat"), ids: ["chat.images", "html", "regex", "lint", "rewrite", "status"] },
		{ title: $_("corePlugins.groups.ai"), ids: ["translate", "tts", "imagegen", "memory", "summary", "ghostwrite", "slash"] },
		{ title: $_("corePlugins.groups.data"), ids: ["sync"] },
		{ title: $_("corePlugins.groups.ext"), ids: ["community"] },
	]);
	/** 세부 설정 탭이 있는 플러그인 */
	const TAB: Partial<Record<PluginId, SettingsTab>> = { phone: "phone", widget: "widget", regex: "regex", translate: "translate", tts: "tts", imagegen: "imagegen", sync: "sync", community: "community", lint: "lint", memory: "memory", summary: "summary", rewrite: "rewrite", ghostwrite: "ghostwrite", slash: "quickreply" };
	const byId = (id: string) => plugins.list.find((p) => p.id === id);
</script>

<Section title={$_("corePlugins.title")}>
	{#each GROUPS as g (g.title)}
		<Group title={g.title}>
			{#each g.ids as id (id)}
				{@const p = byId(id)}
				{@const tab = TAB[id]}
				{#if p}
					{@const loc = localizeCorePlugin(p)}
					<Row label={loc.name} desc={loc.description} nowrap>
						{#if p.desktopOnly && layout.mobile}<Badge>{$_("corePlugins.desktopOnly")}</Badge>{/if}
						{#if tab && p.enabled}<IconButton label="{loc.name} {$_('nav.settings')}" size="sm" onclick={() => nav.replace({ page: "settings", tab })}><Settings2 /></IconButton>{/if}
						<Switch checked={p.enabled} label={loc.name} hideLabel disabled={p.desktopOnly && layout.mobile} onchange={(v) => plugins.set(id, v)} />
					</Row>
				{/if}
			{/each}
		</Group>
	{/each}
</Section>
