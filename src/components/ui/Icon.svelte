<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import * as Icons from "@lucide/svelte";
	import type { SVGAttributes } from "svelte/elements";
	type LucideIconName = keyof typeof Icons;
	type Props = SVGAttributes<SVGSVGElement> & {
		name: LucideIconName | (string & {});
		size?: number;
	};

	// 부모로부터 문자열로 아이콘 이름을 받음 (예: "ArrowRight", "Search" 등)
	let { name = "Activity", size = 16, color = "currentColor", class: className, children, ...restProps }: Props = $props();

	const getPascalCase = (str: string) => {
		if (!str) return "";

		return (
			str
				// 1. 대시(-)나 언더바(_) 뒤에 오는 문자를 대문자로 변경
				.replace(/[-_]([a-z0-9])/gi, (match, letter) => letter.toUpperCase())
				// 2. 문자열의 첫 글자를 대문자로 변경
				.replace(/^[a-z]/, (letter) => letter.toUpperCase())
		);
	};
	// 문자열 이름에 맞는 아이콘 컴포넌트 매칭 (없으면 기본값 지정)
	// @ts-ignore
	let SelectedIcon = $derived(Icons[getPascalCase(name)] || Icons.HelpCircle);
</script>

<!-- 일반 컴포넌트처럼 사용하고 props 전달 -->
<SelectedIcon class={cn("svg-icon", className)} {size} {color} nonScalingStroke {...restProps}>
	{@render children?.()}
</SelectedIcon>

<style>
	:global(.svg-icon.fill) {
		fill: currentColor;
	}
</style>
