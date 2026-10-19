import '$/styles/styles.css'
import Widget from './WidgetApp.svelte';
import { mount } from 'svelte'
import "$/lib/utils/i18n";

const app = mount(Widget, {
  target: document.getElementById('app')!,
})

export default app