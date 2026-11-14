import "../styles/font.css";
import "$shared/ui/base.css";
import { mount } from "svelte";
import DesignPreview from "./DesignPreview.svelte";

mount(DesignPreview, { target: document.getElementById("app")! });
