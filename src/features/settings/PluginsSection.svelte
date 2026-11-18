<script lang="ts">
	// 코어 플러그인: 부가 기능을 껐다 켬 (Obsidian의 코어 플러그인처럼). 목록과 기본값은 백엔드 sys/plugins.rs.
	// 기능별 세부 설정은 각자의 설정 탭 (켜져 있을 때만 보임)
	import { onMount } from "svelte";
	import { Settings2 } from "@lucide/svelte";
	import { Badge, IconButton, Switch } from "$shared/ui";
	import { layout } from "$app/layout.svelte";
	import { nav, type SettingsTab } from "$app/nav.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	onMount(() => void plugins.load());

	const GROUPS: { title: string; ids: PluginId[] }[] = [
		{ title: "화면", ids: ["phone", "game", "widget"] },
		{ title: "대화 표시", ids: ["chat.images", "html", "regex", "lint"] },
		{ title: "AI 도구", ids: ["translate", "tts", "imagegen", "memory"] },
		{ title: "데이터", ids: ["sync"] },
		{ title: "확장", ids: ["community"] },
	];
	/** 세부 설정 탭이 있는 플러그인 */
	const TAB: Partial<Record<PluginId, SettingsTab>> = { phone: "phone", widget: "widget", regex: "regex", translate: "translate", tts: "tts", imagegen: "imagegen", sync: "sync", community: "community", lint: "lint", memory: "memory" };
	const byId = (id: string) => plugins.list.find((p) => p.id === id);
</script>

<Section title="코어 플러그인" lead="필요 없는 부가 기능은 꺼 두면 메뉴와 설정에서 사라지고 AI에게도 쓰이지 않음. 끈 기능의 데이터는 지우지 않음">
	{#each GROUPS as g (g.title)}
		<Group title={g.title}>
			{#each g.ids as id (id)}
				{@const p = byId(id)}
				{@const tab = TAB[id]}
				{#if p}
					<Row label={p.name} desc={p.description}>
						{#if p.desktopOnly && layout.mobile}<Badge>데스크톱 전용</Badge>{/if}
						{#if tab && p.enabled}<IconButton label="{p.name} 설정" size="sm" onclick={() => nav.replace({ page: "settings", tab })}><Settings2 /></IconButton>{/if}
						<Switch checked={p.enabled} label="{p.name} 사용" hideLabel disabled={p.desktopOnly && layout.mobile} onchange={(v) => plugins.set(id, v)} />
					</Row>
				{/if}
			{/each}
		</Group>
	{/each}
</Section>
