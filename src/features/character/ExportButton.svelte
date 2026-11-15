<script lang="ts">
	// 대화 내보내기: 한 버튼 + 형식 고르기
	import { ExportSplit } from "$shared/ui";
	import { CHAT_FORMATS, convActions, type ChatFormat } from "./actions";
	import type { Conversation } from "./library.svelte";

	let { conv }: { conv: Conversation } = $props();
</script>

<ExportSplit formats={CHAT_FORMATS} storageKey="sc.export.chat" onexport={(f) => convActions.exportFormat(conv, f as ChatFormat)} />
