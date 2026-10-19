import { tick } from "svelte";

interface TooltipData {
	label: string;
	delayDuration: number;
}

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// 툴팁 작동 트리거: aria-label이 붙은 요소
export function tooltipTrigger(node: HTMLElement, data: TooltipData) {
	let timerId: number | null = null;

	node.setAttribute("aria-label", data.label);
	node.setAttribute("aria-tooltip-delay", data.delayDuration.toString());

	function handleOpenTooltip(e: MouseEvent) {
		e.preventDefault();
		timerId = setTimeout(() => {
			tooltipStore.openTooltip(e.clientX, e.clientY, data.label);
		}, data.delayDuration);
	}

	function handleCloseTooltip(e: MouseEvent) {
		tooltipStore.closeTooltip();
		if (timerId) {
			clearTimeout(timerId);
			timerId = null;
		}
	}

	node.addEventListener("mouseenter", handleOpenTooltip);
	node.addEventListener("mouseleave", handleCloseTooltip);

	return {
		destroy() {
			// 이벤트 리스너 제거
			node.addEventListener("mouseenter", handleOpenTooltip);
			node.addEventListener("mouseleave", handleCloseTooltip);
		},
	};
}

class TooltipStore {
	isOpen = $state(false);
	pos = $state({ x: 0, y: 0 });
	tooltipEl = $state<HTMLElement | undefined>(undefined);
	position = "";
	label = $state("");

	openTooltip = async (x: number, y: number, label: string) => {
		this.label = label;
		this.isOpen = true;

		await tick();

		if (this.tooltipEl) {
			const { offsetWidth, offsetHeight } = this.tooltipEl;
			const { innerWidth, innerHeight } = window;

			if (offsetWidth - innerWidth < 0) {
				// x가 툴팁을 띄워야 하는 대상의 오른쪽에 위치
				// 기본으로 오른쪽에 툴팁을 띄울거야
				this.position = "right";
			}

			this.pos = { x, y };
		}
	};
	closeTooltip = () => {
		this.pos = { x: 0, y: 0 };
		this.isOpen = false;
		this.label = "";
	};
}
export const tooltipStore = new TooltipStore();
