import type { Attachment } from "svelte/attachments";

type Pos = { x: number; y: number };
type Option = {
	pos?: Pos;
	onchange?: (pos: Pos) => void;
	onEditEnd?: () => void;
};

export function draggablePosition(options: Option): Attachment<HTMLElement> {
	return (node) => {
		let isDragging = false;
		let startX = 0;
		let startY = 0;

		const fallback = $state({ x: options.pos?.x ?? 50, y: options.pos?.y ?? 50 });
		const pos = fallback;

		let stopDrag: (() => void) | null = null;

		$effect(() => {
			node.style.setProperty("--pos-x", `${pos.x}%`);
			node.style.setProperty("--pos-y", `${pos.y}%`);
			node.style.cursor = isDragging ? "grabbing" : "grab";
		});

		function handleMouseDown(e: MouseEvent) {
			e.stopPropagation();
			isDragging = true;
			startX = e.clientX;
			startY = e.clientY;

			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", handleMouseUp);

			function cleanup() {
				isDragging = false;
				node.style.cursor = "default";
				window.removeEventListener("mousemove", handleMouseMove);
				window.removeEventListener("mouseup", handleMouseUp);
				window.removeEventListener("scroll", cleanup);
				stopDrag = null;
			}

			function handleMouseMove(e: MouseEvent) {
				if (!isDragging) return;
				const deltaX = e.clientX - startX;
				const deltaY = e.clientY - startY;

				startX = e.clientX;
				startY = e.clientY;

				const sensitivity = 0.3;
				pos.x = Math.round(Math.max(0, Math.min(100, pos.x - deltaX * sensitivity)));
				pos.y = Math.round(Math.max(0, Math.min(100, pos.y - deltaY * sensitivity)));
			}

			function handleMouseUp(e: MouseEvent) {
				cleanup(); // 리스너와 드래그 미리 정리
				isDragging = false;
				options?.onEditEnd?.();
				options.onchange?.(pos);
			}

			stopDrag = cleanup;
			window.addEventListener("scroll", cleanup, { passive: true });
		}

		node.addEventListener("mousedown", handleMouseDown);

		return () => {
			stopDrag?.();
			node.removeEventListener("mousedown", handleMouseDown);
		};
	};
}
