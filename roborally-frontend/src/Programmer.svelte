<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { flip } from "svelte/animate";

  export let round_registers: number;
  export let cards_assets_names: [string, string][];
  export let selected: undefined | number[] = undefined;

  type CardWithId = {
    card: number | undefined;
    id: number | string;
    selected: boolean;
    position: number;
  };
  export let initialCards: number[];

  let cardsInHand: CardWithId[] = [];
  let programmedCards: CardWithId[] = [];

  onMount(() => {
    cardsInHand = initialCards.map((c, i) => ({
      card: c,
      id: i,
      selected: false,
      position: i,
    }));
    if (selected === undefined)
      programmedCards = new Array(round_registers)
        .fill(undefined)
        .map((_, i) => ({
          card: undefined,
          id: Math.random().toString(),
          selected: true,
          position: i,
        }));
    else
      programmedCards = selected.map((c, i) => ({
        card: c,
        id: i + initialCards.length,
        selected: true,
        position: i,
      }));
  });

  let eventSource = createEventDispatcher<{
    programmingDone: number[];
  }>();

  function handleDragOver(e: DragEvent, card: CardWithId) {
    let incoming = e.dataTransfer?.getData("application/json");
    if (incoming === undefined || incoming === "") return;
    let incomingCard = JSON.parse(incoming);
    if (incomingCard.id === card.id) return;
    if (card.card !== undefined && card.selected) {
      return;
    }
    e.preventDefault();
    if (!card.selected) {
      while (
        cardsInHand[card.position - 1] !== undefined &&
        cardsInHand[card.position - 1].card === undefined
      ) {
        card = cardsInHand[card.position - 1];
      }
    }
    dragTarget = card;
  }

  function handleDrop(e: DragEvent) {
    let incomingCard = JSON.parse(e.dataTransfer!.getData("application/json"));

    if (incomingCard.selected) {
      programmedCards[incomingCard.position] = {
        card: undefined,
        id: Math.random().toString(),
        selected: true,
        position: null,
      };
    } else {
      cardsInHand = [
        ...cardsInHand.slice(0, incomingCard.position),
        ...cardsInHand.slice(incomingCard.position + 1),
      ];
    }

    if (dragTarget.selected) {
      programmedCards[dragTarget.position] = incomingCard;
    } else {
      let position = cardsInHand.findIndex((c) => c.id === dragTarget.id);
      cardsInHand = [
        ...cardsInHand.slice(0, position),
        incomingCard,
        ...cardsInHand.slice(position),
      ];
    }

    cardsInHand = cardsInHand.filter((c) => c.card !== undefined);
    while (cardsInHand.length < initialCards.length) {
      cardsInHand.push({
        card: undefined,
        id: Math.random().toString(),
        selected: false,
        position: null,
      });
    }

    for (let i = 0; i < cardsInHand.length; i++) {
      cardsInHand[i].position = i;
      cardsInHand[i].selected = false;
    }
    for (let i = 0; i < programmedCards.length; i++) {
      programmedCards[i].position = i;
      programmedCards[i].selected = true;
    }
  }

  let dragTarget = undefined;
</script>

<div
  class="outer"
  style:--total-slots={initialCards.length + round_registers}
  style:--register-slots={round_registers}
  style:--hand-slots={initialCards.length}
>
  <div class="inner">
    {#if cardsInHand.length > 0}
      <div class="header">
        <div class="hand-title">Cards in your hand</div>
        <div class="registers-title">
          Programmed registers
          <button
            disabled={programmedCards.some((c) => c.card === undefined) ||
              selected !== undefined}
            on:click={() => {
              eventSource(
                "programmingDone",
                programmedCards.map((c) => c.card)
              );
            }}>Save program</button
          >
        </div>
      </div>
    {/if}
    <div class="dragger-content" class:enabled={selected === undefined}>
      {#each cardsInHand.concat(programmedCards) as card (card.id)}
        <div
          animate:flip={{
            duration: card.card === undefined ? 0 : (d) => Math.sqrt(d) * 80,
          }}
          class:empty={card.card === undefined}
          class:selected={card.selected}
          class:drag-target={card.selected && dragTarget?.id === card.id}
          on:dragstart={(e) => {
            e.dataTransfer.setData("application/json", JSON.stringify(card));
            e.dataTransfer.effectAllowed = "move";
          }}
          on:dragover={(e) => handleDragOver(e, card)}
          on:dragenter={(e) => handleDragOver(e, card)}
          on:dragend={() => (dragTarget = undefined)}
          on:dragleave={() => (dragTarget = undefined)}
          on:drop={handleDrop}
        >
          {#if !card.selected && card.id === dragTarget?.id}
            <div class="drag-target-before" />
          {/if}
          {#if card.card !== undefined}
            <img
              src={cards_assets_names[card.card][0]}
              alt={cards_assets_names[card.card][1]}
              draggable={selected === undefined}
            />
          {:else if card.selected}
            {card.position + 1}
          {/if}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  * {
    box-sizing: border-box;
  }
  .outer {
    max-width: 100%;
  }
  .inner {
    width: calc(
      var(--total-slots) * calc(var(--card-width) + var(--card-margin)) +
        var(--inner-padding) * 2
    );
    --inner-padding: 0.8rem;
    padding: 0.5rem var(--inner-padding);
  }
  .header {
    display: flex;
    align-items: center;
    margin-bottom: 0.3rem;
  }
  .hand-title {
    width: calc(
      var(--hand-slots) * calc(var(--card-width) + var(--card-margin))
    );
  }
  .registers-title {
    width: calc(
      var(--register-slots) * calc(var(--card-width) + var(--card-margin))
    );
  }
  .hand-title,
  .registers-title,
  .registers-title button {
    color: #eee;
    text-align: center;
  }
  .registers-title button {
    background-color: rgb(0, 68, 23);
    border-radius: 7px;
    border: 2px solid black;
    padding: 2px 5px;
  }
  .registers-title button:disabled {
    background-color: #555;
  }
  .registers-title button:not(:disabled):hover {
    background-color: rgb(0, 46, 15);
  }
  .dragger-content {
    display: flex;
  }
  .dragger-content > div {
    width: var(--card-width);
    height: calc(var(--card-width) * 10 / 7);
    margin: 0 calc(var(--card-margin) / 2);
    border-radius: var(--card-border-radius);
    position: relative;
  }
  .dragger-content div.drag-target-before {
    height: calc(var(--card-width) * 10 / 7 + var(--card-margin));
    width: calc(var(--card-margin) * 0.5);
    background-color: rgb(255, 240, 20);
    position: absolute;
    left: calc(-0.5 * var(--card-margin));
    top: calc(-0.5 * var(--card-margin));
    border-radius: 10em;
    transform: translateX(-50%);
  }
  .dragger-content.enabled > div:not(.empty) {
    cursor: grab;
  }
  .dragger-content > div > img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    border-radius: var(--card-border-radius);
  }
  .dragger-content > div.selected.empty {
    --highlight: black;
    font-size: 3rem;
    border: 2px dashed var(--highlight);
    color: var(--highlight);
    font-family: sans-serif;
    display: flex;
    justify-content: center;
    align-items: center;
  }
  .dragger-content > div.drag-target {
    --highlight: rgb(255, 240, 20) !important;
  }
</style>
