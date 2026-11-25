<script lang="ts">
	import { _ } from "svelte-i18n";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import Section from "./Section.svelte";
	import Group from "./Group.svelte";
	import Row from "./Row.svelte";

	// 지금 앱에서 실제로 동작하는 키만 보여줌
	const groups = $derived([
		{
			title: $_("settings.shortcutSec.navGroup"),
			items: [
				{ label: $_("settings.shortcutSec.backForward"), keys: [["Alt", "←"], ["Alt", "→"]] },
				{ label: $_("settings.shortcutSec.backForwardMouse"), keys: [["Mouse 4"], ["Mouse 5"]] },
			],
		},
		{
			title: $_("settings.shortcutSec.chatGroup"),
			items: [
				{ label: $_("settings.shortcutSec.send"), keys: [appSetting.sendOnEnter ? ["Enter"] : ["Ctrl", "Enter"]] },
				{ label: $_("settings.shortcutSec.newline"), keys: [["Shift", "Enter"]] },
				{ label: $_("settings.shortcutSec.cancelClose"), keys: [["Esc"]] },
			],
		},
	]);
</script>

<Section title={$_("settings.shortcutSec.title")} lead={$_("settings.shortcutSec.lead")}>
	{#each groups as g (g.title)}
		<Group title={g.title}>
			{#each g.items as it (it.label)}
				<Row label={it.label}>
					<span class="keys">
						{#each it.keys as combo, i (i)}
							{#if i > 0}<span class="or">/</span>{/if}
							{#each combo as k (k)}<kbd>{k}</kbd>{/each}
						{/each}
					</span>
				</Row>
			{/each}
		</Group>
	{/each}
</Section>

<style>
	.keys {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.or {
		margin: 0 4px;
		color: var(--text-faint);
	}
	kbd {
		min-width: 24px;
		padding: 2px 6px;
		border: 1px solid var(--border-strong);
		border-bottom-width: 2px;
		border-radius: 5px;
		background: var(--surface);
		font-family: var(--font-sans);
		font-size: var(--text-xs);
		font-weight: 600;
		text-align: center;
	}
</style>
