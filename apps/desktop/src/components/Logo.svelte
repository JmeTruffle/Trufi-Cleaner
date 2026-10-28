<script lang="ts">
  import logo from "../assets/logo.png";
  let { size = 22 }: { size?: number } = $props();
</script>

<img src={logo} width={size} height={size} alt="" aria-hidden="true" draggable="false" />

<style>
  img { display: block; flex: none; }
</style>
