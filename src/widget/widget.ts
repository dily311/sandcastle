import "$/styles/styles.css";
import Widget from "./WidgetApp.svelte";
import { mount } from "svelte";
import "$/lib/utils/i18n";

window.addEventListener("keydown", (e) => {
	// Ctrl + P 또는 Cmd + P (인쇄 단축키) 감지
	if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
		e.preventDefault();
	}
});

const app = mount(Widget, {
	target: document.getElementById("app")!,
});

export default app;
