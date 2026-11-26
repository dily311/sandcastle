<script lang="ts">
	// 플러그인 명령 메뉴 (6-D-6). 명령은 사용자가 여기서 누를 때만 플러그인에 간다 (Explicit Execution)
	import { _ } from "svelte-i18n";
	import { Puzzle } from "@lucide/svelte";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { pluginHost } from "./host.svelte";

	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);

	const items = $derived<MenuItem[]>(
		pluginHost.active.flatMap((p, i) =>
			p.commands.map((c, j) => ({
				label: `${c.title} · ${p.name}`,
				separator: i > 0 && j === 0,
				onselect: () => void pluginHost.runCommand(p.id, c.id),
			})),
		),
	);

	function open(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
</script>

{#if items.length}
	<IconButton label={$_("communityUi.commandsMenu")} size="sm" onclick={open}><Puzzle /></IconButton>
	<ContextMenu at={menu} {items} label={$_("communityUi.commandsMenu")} onclose={() => (menu = null)} />
{/if}
