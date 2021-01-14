<script>
  import Radio from './Radio.svelte';
</script>

<main>
  <Radio class="radio" name="Radio 357" station="357" />
  <Radio class="radio" name="Radio Nowy Świat" station="rns" />
</main>

<style>
	main {
		text-align: center;
		padding: 1em;
		margin: 0 auto;
    position: relative;
    overflow: scroll;
    max-height: 90vh;
	}

  .radio {
    width: 600px;
  }
</style>
