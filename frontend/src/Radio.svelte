<script>
import { onMount } from 'svelte';
export let name, station;

let items = [];

let radio;

onMount(async () => {
  const res = await fetch(`/api/${station}`);
  items = await res.json();
  items.sort((lhs, rhs) => lhs.start_at - rhs.start_at);
  for (let item of items) {
    item.start_at = new Date(item.start_at);
    item.end_at = new Date(item.end_at);
  }
});

function scrollToCurrent() {
  radio.querySelector(".schedule").scrollTop = radio.querySelector(".current").offsetTop;
}
</script>

<style>
.schedule {
  overflow: scroll;
  max-height: 80vh;
  position: relative;
}
.item {
  border: 1px solid black;
}
.current {
  border: 5px solid crimson;
}
</style>

<div class="radio" bind:this={radio}>
  <h1>{name}</h1>
  <button on:click={scrollToCurrent}>now</button>
  <div class="schedule">
    {#each items as item (item.start_at)}
    <div class="item" class:current={new Date() >= item.start_at && new Date() < item.end_at}>
      <time datetime={item.start_at.toISOString()}>{item.start_at.toISOString()}</time>
      <h3>{item.name}</h3>
      <h5>{item.description}</h5>
      <ul>
        {#each item.hosts as host (host)}
        <li>{host}</li>
        {/each}
      </ul>
    </div>
    {/each}
  </div>
</div>
