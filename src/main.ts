import "$/styles/styles.css";
import App from "./App.svelte";
import { mount } from "svelte";
import "$/lib/utils/i18n";
import "$ipc/mockup";

document.addEventListener(
	"contextmenu",
	function (e) {
		e.preventDefault(); // 기본 동작 막기
	},
	false,
);

const app = mount(App, {
	target: document.getElementById("app")!,
});

export default app;
