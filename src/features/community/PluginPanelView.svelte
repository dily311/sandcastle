<script lang="ts">
	// 플러그인 패널 탭의 내용 (6-D-6): 플러그인 폴더의 html을 sandbox iframe으로.
	// 앱과 같은 색을 쓰도록 테마 CSS 변수를 보내 준다 (SDK가 <style id="sc-theme">에 넣음)
	import { _ } from "svelte-i18n";
	import { theme } from "$app/theme.svelte";
	import { currentThemeCss } from "$shared/html/theme";
	import { EmptyState } from "$shared/ui";
	import { pluginHost } from "./host.svelte";
	import { parsePanelTab } from "./panels";

	let { tab }: { tab: string } = $props();

	const target = $derived(parsePanelTab(tab));
	const plugin = $derived(target ? pluginHost.active.find((p) => p.id === target.pluginId) : undefined);
	const panelDecl = $derived(plugin?.panels.find((p) => p.id === target?.panelId));
	let frame = $state<HTMLIFrameElement | null>(null);

	function sendTheme() {
		const css = currentThemeCss(theme.dark);
		frame?.contentWindow?.postMessage({ type: "sc-plugin-theme", css }, "*");
	}

	$effect(() => {
		if (!frame || !plugin) return;
		const el = frame;
		const id = plugin.id;
		pluginHost.attachPanel(id, el);
		return () => pluginHost.detachPanel(el);
	});
	$effect(() => {
		void theme.dark;
		void JSON.stringify(theme.setting);
		sendTheme();
	});
</script>

{#if plugin && panelDecl}
	{#key panelDecl.url + pluginHost.gen}
		<iframe bind:this={frame} class="plugin-panel" title={`${plugin.name} · ${panelDecl.title}`} src={panelDecl.url} sandbox="allow-scripts" referrerpolicy="no-referrer" onload={sendTheme}></iframe>
	{/key}
{:else}
	<EmptyState title={$_("communityUi.panelUnavailableTitle")} description={$_("communityUi.panelUnavailableDesc")} />
{/if}

<style>
	.plugin-panel {
		display: block;
		width: 100%;
		height: 100%;
		min-height: 320px;
		border: 0;
		background: transparent;
	}
</style>
