import type { Attachment } from "svelte/attachments";

type Options = {
	items: any;
	onReorder: (items: any) => void;
};

export function sortableList({ items, onReorder }: Options): Attachment<HTMLElement> {
	return (node) => {
		let activeEl: HTMLLIElement | null = null;
		let startIndex: number | null = null;
		let startY = 0;

		const handlePointerDown = (e: PointerEvent) => {
			const targetEl = e.target as HTMLElement;

			// 1. 'order-change-btn' 클래스를 가진 요소나 그 자식을 클릭했을 때만 드래그 시작 허용
			const handleBtn = targetEl.closest(".order-change-btn");
			if (!handleBtn) return;

			// 2. 가장 가까운 리스트 아이템([data-index]) 탐색
			const target = targetEl.closest("[data-index]") as HTMLLIElement | null;
			if (!target) return;

			startIndex = Number(target.dataset.index);
			activeEl = target;

			try {
				target.setPointerCapture(e.pointerId);
			} catch (err) {}

			startY = e.clientY;
			activeEl.classList.add("onDrag");
		};

		const handlePointerMove = (e: PointerEvent) => {
			if (!activeEl) return;
			e.preventDefault(); // 스크롤 방지 및 부드러운 드래그 유도

			const deltaY = e.clientY - startY;
			activeEl.style.transform = `translateY(${deltaY}px)`;
		};

		const handlePointerUp = (e: PointerEvent) => {
			if (!activeEl) return;

			activeEl.style.transform = "";
			activeEl.classList.remove("onDrag");

			const elemBelow = document.elementFromPoint(e.clientX, e.clientY);
			const targetBelow = elemBelow?.closest("[data-index]") as HTMLLIElement | null;

			if (targetBelow) {
				const targetIndex = Number(targetBelow.dataset.index);
				if (startIndex !== null && startIndex !== targetIndex) {
					const updated = [...items];
					const [moved] = updated.splice(startIndex, 1);
					updated.splice(targetIndex, 0, moved);

					onReorder(updated);
				}
			}

			try {
				activeEl.releasePointerCapture(e.pointerId);
			} catch (err) {}

			activeEl = null;
			startIndex = null;
		};

		// 이벤트 리스너 등록
		node.addEventListener("pointerdown", handlePointerDown as EventListener);
		window.addEventListener("pointermove", handlePointerMove as EventListener, { passive: false });
		window.addEventListener("pointerup", handlePointerUp as EventListener);

		// 클린업
		return () => {
			node.removeEventListener("pointerdown", handlePointerDown as EventListener);
			window.removeEventListener("pointermove", handlePointerMove as EventListener);
			window.removeEventListener("pointerup", handlePointerUp as EventListener);
		};
	};
}
