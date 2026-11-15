<script lang="ts">
	// 꺼진 코어 플러그인의 화면에 들어왔을 때 (주소로 바로 오거나 이전 화면 기록으로)
	import { Blocks } from "@lucide/svelte";
	import { Button, EmptyState } from "$shared/ui";
	import { nav } from "./nav.svelte";
	import { plugins, type PluginId } from "$shared/stores/plugins.svelte";

	let { id, name }: { id: PluginId; name: string } = $props();
</script>

<div class="off">
	<EmptyState title="{name} 기능이 꺼져 있음" description="설정 > 코어 플러그인에서 켤 수 있음">
		{#snippet action()}
		<div class="acts">
			<Button variant="primary" onclick={() => plugins.set(id, true)}>{#snippet icon()}<Blocks />{/snippet}지금 켜기</Button>
			<Button variant="ghost" onclick={() => nav.go({ page: "settings", tab: "plugins" })}>코어 플러그인 설정</Button>
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
