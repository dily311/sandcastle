<script lang="ts">
	// 메뉴에서 '내보내기…'를 누르면 뜨는 형식 목록 (AppShell에 하나)
	import { Check } from "@lucide/svelte";
	import { ContextMenu, type MenuItem } from "$shared/ui";
	import { exportPicker, rememberedFormat, rememberFormat } from "./exportFormats.svelte";

	const items = $derived.by((): MenuItem[] => {
		const p = exportPicker.current;
		if (!p) return [];
		const last = rememberedFormat(p.key, p.formats);
		return p.formats.map((f) => ({
			label: f.desc ? `${f.label} — ${f.desc}` : f.label,
			icon: f.id === last ? Check : undefined,
			onselect: () => {
				rememberFormat(p.key, f.id);
				p.run(f.id);
			},
		}));
	});
</script>

{#if exportPicker.current}
	<ContextMenu at={exportPicker.current.at} label={exportPicker.current.title} {items} onclose={() => exportPicker.close()} />
{/if}
