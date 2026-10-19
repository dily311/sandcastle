<script lang="ts">
	import type { FolderMap } from "$/lib/type/ui";

	// 부모로부터 Map 객체를 전달받음
	let { data }: { data: FolderMap } = $props();

	// 데이터가 Map 타입인지(폴더인지) 확인하는 함수
	function isFolder(value: string | FolderMap) {
		return value instanceof Map;
	}
</script>

<ul>
	{#each [...data.entries()] as [name, content]}
		<li>
			{#if isFolder(content)}
				<!-- 폴더 아이콘과 이름 -->
				<span class="folder">📁 {name}</span>

				<!-- 하위 폴더 구조를 위해 자기 자신을 재귀 호출 -->
				<!-- svelte-ignore svelte_self_deprecated -->
				<svelte:self data={content} />
			{:else}
				<!-- 파일 아이콘과 이름 -->
				<span class="file">📄 {name}</span>
			{/if}
		</li>
	{/each}
</ul>

<style>
	ul {
		list-style-type: none;
		padding-left: 20px;
		margin: 4px 0;
	}
	li {
		margin: 2px 0;
	}
	.folder {
		font-weight: bold;
		color: #333;
		cursor: pointer;
	}
	.file {
		color: #666;
	}
</style>
