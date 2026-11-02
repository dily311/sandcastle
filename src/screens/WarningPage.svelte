<script lang="ts">
	import Button from "$/components/ui/button/Button.svelte";
	import { _ } from "svelte-i18n";
	import { push } from "svelte-spa-router";
</script>

<div class="page">
	<p>
		에러!!!!! <br />
		잘못된 페이지 경로
	</p>
	<Button onclick={() => push("/")}>홈으로 돌아가기</Button>
</div>

<style>
</style>
