<script lang="ts">
	// 꺼진 코어 플러그인의 화면에 들어왔을 때 (주소로 바로 오거나 이전 화면 기록으로)
	import { _ } from "svelte-i18n";
	import { Blocks } from "@lucide/svelte";
	import { Button, EmptyState } from "$shared/ui";
	import { nav } from "./nav.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";

	let { id, name }: { id: PluginId; name: string } = $props();
</script>

<div class="off">
	<EmptyState title={$_("settings.mascot.pluginOffTitle", { values: { name } })} description={$_("settings.mascot.pluginOffDesc")}>
		{#snippet action()}
		<div class="acts">
			<Button variant="primary" onclick={() => plugins.set(id, true)}>{#snippet icon()}<Blocks />{/snippet}{$_("settings.mascot.enableNow")}</Button>
			<Button variant="ghost" onclick={() => nav.go({ page: "settings", tab: "plugins" })}>{$_("settings.mascot.corePluginSettings")}</Button>
		</div>
		{/snippet}
	</EmptyState>
</div>

<style>
	.off {
		display: grid;
		place-items: center;
		height: 100%;
		padding: var(--space-6);
	}
	.acts {
		display: flex;
		gap: var(--space-2);
		justify-content: center;
	}
</style>
