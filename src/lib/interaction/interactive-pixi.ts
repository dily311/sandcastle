// pixi-interactive.ts
import type { Application, Container } from "pixi.js";
import { interactiveManager } from "./interactive-registry";

function isShown(obj: Container): boolean {
	for (let o: Container | null = obj; o; o = o.parent as Container | null) {
		if (!o.visible || o.alpha === 0) return false;
	}
	return true;
}

export function pixiInteractive(
	app: Application,
	obj: Container,
	id: string,
	intervalMs = 33, // 측정 간격 (약 30fps)
) {
	const canvas = app.canvas; // v7이면 app.view as HTMLCanvasElement
	let last = -Infinity;

	const measure = () => {
		if (obj.destroyed || !isShown(obj)) {
			interactiveManager.unregisterInteractiveHitbox(id);
			return;
		}

		const b = obj.getBounds();
		const r = "rectangle" in b ? b.rectangle : b;

		const c = canvas.getBoundingClientRect();
		const sx = c.width / app.screen.width;
		const sy = c.height / app.screen.height;

		interactiveManager.registerInteractiveHitbox(id, {
			x: c.left + r.x * sx,
			y: c.top + r.y * sy,
			width: r.width * sx,
			height: r.height * sy,
		});
	};

	const tick = () => {
		const now = performance.now();
		if (now - last < intervalMs) return; // 간격 안 지났으면 스킵
		last = now;
		measure();
	};

	app.ticker.add(tick);
	measure(); // 등록 즉시 1회는 제한 없이 측정
	last = performance.now();

	const stop = () => {
		// app.ticker.remove(tick);
		interactiveManager.unregisterInteractiveHitbox(id);
	};
	obj.once("destroyed", stop);

	return stop;
}