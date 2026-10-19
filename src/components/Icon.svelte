<script lang="ts">
	import * as Icons from "@lucide/svelte";

	// 부모로부터 문자열로 아이콘 이름을 받음 (예: "ArrowRight", "Search" 등)
	let { name = "Activity", size = 24, color = "currentColor" } = $props();
	
	/** 첫번째 문자열을 대문자로 만드는 처리 */
	function capitalize(str: string) {
		if (!str) return ""; // 빈 문자열 처리
		return str.charAt(0).toUpperCase() + str.slice(1);
	}

	// 문자열 이름에 맞는 아이콘 컴포넌트 매칭 (없으면 기본값 지정)
	// @ts-ignore
	let SelectedIcon = $derived(Icons[capitalize(name)] || Icons.HelpCircle);
</script>

<!-- 일반 컴포넌트처럼 사용하고 props 전달 -->
<SelectedIcon {size} {color} />
