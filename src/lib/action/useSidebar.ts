import type { Attachment } from "svelte/attachments";

interface sidebarSidlerItem {
	axis?: "left" | "right";
	isSidebarOpen: boolean;
	toggleSidebar: () => void;
}

export function sidebarSidler(options: sidebarSidlerItem): Attachment<HTMLElement> {
	return (node) => {
		const ele = node.parentElement;
		let isResizing = false;

		function startResize(e: MouseEvent) {
			e.preventDefault();
			isResizing = true;
			window.addEventListener("mousemove", handleMouseMove);
			window.addEventListener("mouseup", stopResize);
		}

		function handleMouseMove(e: MouseEvent) {
			if (!isResizing || ele === null) return;

			document.body.classList.add("is-resizing");
			let newWidth = e.clientX - 16;
			if (options.axis === "right") {
				// 전체 화면 너비에서 마우스 X 위치 및 마진(16px)을 차감
				newWidth = window.innerWidth - e.clientX - 16;
			}

			if (newWidth < 200) {
				if (options.isSidebarOpen) {
					options.toggleSidebar();
					stopResize();
				}
			} else if (newWidth <= 800) {
				ele.style.width = `${newWidth}px`;
			}
		}

		function stopResize() {
			isResizing = false;
			document.body.classList.remove("is-resizing");
			window.removeEventListener("mousemove", handleMouseMove);
			window.removeEventListener("mouseup", stopResize);
		}

		node.addEventListener("mousedown", startResize);

		return () => {
			node.removeEventListener("mousedown", startResize);
		};
	};
}
