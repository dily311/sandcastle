<script lang="ts">
	// 화면을 하나씩 새로 만들면서 이 자리 표시를 실제 화면으로 바꾼다
	import { EmptyState } from "$shared/ui";
	import { Hammer } from "@lucide/svelte";
	let { page }: { page: string } = $props();
	const names: Record<string, string> = { phone: "폰", character: "캐릭터", game: "게임", persona: "페르소나", settings: "설정" };
</script>

<EmptyState title="{names[page] ?? page} 화면 준비 중" description="새 디자인으로 다시 만드는 중">
	{#snippet icon()}<Hammer />{/snippet}
</EmptyState>
