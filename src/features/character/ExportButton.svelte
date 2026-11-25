<script lang="ts">
	// 대화 내보내기: 한 버튼 + 형식 고르기
	import { _ } from "svelte-i18n";
	import { ExportSplit } from "$shared/ui";
	import { getChatFormats, convActions, type ChatFormat } from "./actions";
	import type { Conversation } from "./library.svelte";

	let { conv }: { conv: Conversation } = $props();
	const formats = $derived(getChatFormats((k) => $_(k)));
</script>

<ExportSplit {formats} storageKey="sc.export.chat" onexport={(f) => convActions.exportFormat(conv, f as ChatFormat)} />
