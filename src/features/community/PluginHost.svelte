<script lang="ts">
	// 커뮤니티 플러그인의 숨은 iframe들 (6-D-2). 코어 플러그인 `community`가 켜져 있을 때만 호스트를 시작한다.
	// sandbox="allow-scripts"만 준다(allow-same-origin 없음 → 불투명 origin: 앱 화면·저장소·IPC에 닿지 못함)
	import { plugins } from "$shared/stores/plugins.svelte";
	import { pluginHost } from "./host.svelte";

	let { enabled = plugins.on("community") }: { enabled?: boolean } = $props();

	$effect(() => {
		if (enabled) void pluginHost.start();
		else void pluginHost.stop();
	});
	$effect(() => () => void pluginHost.stop());

	function frame(el: HTMLIFrameElement, id: string) {
		pluginHost.attach(id, el);
		return { destroy: () => pluginHost.attach(id, null) };
	}
</script>

{#if enabled}
	<div class="plugin-host" aria-hidden="true">
		{#key pluginHost.gen}
			{#each pluginHost.plugins as p (p.id)}
				<iframe use:frame={p.id} title={`플러그인 ${p.name}`} src={p.hostUrl} sandbox="allow-scripts" referrerpolicy="no-referrer" tabindex="-1"></iframe>
			{/each}
		{/key}
	</div>
{/if}

<style>
	.plugin-host {
		position: fixed;
		width: 0;
		height: 0;
		overflow: hidden;
		pointer-events: none;
		visibility: hidden;
	}
	.plugin-host iframe {
		width: 0;
		height: 0;
		border: 0;
	}
</style>
