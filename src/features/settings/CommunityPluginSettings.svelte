<script lang="ts">
	// 설치된 커뮤니티 플러그인 하나의 설정 (설정 > 커뮤니티 플러그인 그룹의 탭). 설치·권한·켜기는 "커뮤니티 플러그인" 관리 화면에서
	import { onMount } from "svelte";
	import { _ } from "svelte-i18n";
	import { admin } from "$features/community/admin.svelte";
	import SettingsForm from "$features/community/SettingsForm.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";

	let { pluginId }: { pluginId: string } = $props();
	onMount(() => {
		if (!admin.loaded) void admin.load();
	});
	const plugin = $derived(admin.list.find((p) => p.id === pluginId));
	const m = $derived(plugin?.manifest);
</script>

<Section title={m?.name ?? pluginId} lead={plugin ? (plugin.enabled ? m?.description || "" : $_("settings.extra.communityDisabledLead")) : $_("settings.extra.communityNotInstalled")}>
	{#if m?.settings.length}
		<Group>
			<SettingsForm {pluginId} fields={m.settings} />
		</Group>
	{/if}
</Section>
