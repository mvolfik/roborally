<script lang="ts">
  import type { CardWrapper } from "frontend-wasm";
  import { dndzone, SHADOW_ITEM_MARKER_PROPERTY_NAME } from "svelte-dnd-action";
  import { createEventDispatcher, onMount } from "svelte";
  import { getCardAsset } from "./utils";
  import { flip } from "svelte/animate";

  type CardWithId = { originalCard: CardWrapper; asset: string; id: number };
  type MaybeCard = [CardWithId] | [];
  export let initialCards: Array<CardWrapper>;
  let cardsInHand: Array<CardWithId> = [];
  let programmedCards: [MaybeCard, MaybeCard, MaybeCard, MaybeCard, MaybeCard] =
    [[], [], [], [], []];
  let zoneId = Math.random().toString();

  onMount(() => {
    cardsInHand = initialCards.map((c, i) => ({
      originalCard: c,
      asset: getCardAsset(c.asset_name),
      id: i,
    }));
  });

  let flipDurationMs = 200;

  function handleDndEvent(e: CustomEvent, register_i?: number) {
    if (register_i === undefined) {
      cardsInHand = e.detail.items;
    } else {
      programmedCards[register_i] = e.detail.items;
    }
  }
  let eventSource = createEventDispatcher<{
    programmingDone: [
      CardWrapper,
      CardWrapper,
      CardWrapper,
      CardWrapper,
      CardWrapper
    ];
  }>();
</script>

<div class="outer">
  <p class="hand-title">Cards in your hand</p>
  <p class="registers-title">
    Programmed registers
    <button
      disabled={programmedCards.some((c) => c.length !== 1)}
      on:click={() => {
        eventSource(
          "programmingDone",
          programmedCards.map((c) => c[0].originalCard)
        );
      }}>Save program</button
    >
  </p>
  <div
    class="hand"
    use:dndzone={{ items: cardsInHand, flipDurationMs, type: zoneId }}
    on:consider={(e) => handleDndEvent(e)}
    on:finalize={(e) => handleDndEvent(e)}
  >
    {#each cardsInHand as card (card.id)}
      <img
        src={card.asset}
        alt="Card"
        animate:flip={{ duration: flipDurationMs }}
      />
    {/each}
  </div>
  <div class="registers">
    {#each programmedCards as maybeCard, i}
      <div class="register">
        <span>{i + 1}</span>
        <div
          class="programmed"
          use:dndzone={{
            items: maybeCard,
            flipDurationMs,
            type: zoneId,
            // there is some non-shadow card ([<empty>].some() returns false)
            dropFromOthersDisabled: maybeCard.some(
              (x) => !x[SHADOW_ITEM_MARKER_PROPERTY_NAME]
            ),
          }}
          on:consider={(e) => handleDndEvent(e, i)}
          on:finalize={(e) => handleDndEvent(e, i)}
        >
          {#each maybeCard as card (card.id)}
            <img
              src={card.asset}
              alt="Card"
              animate:flip={{ duration: flipDurationMs }}
            />
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .outer {
    padding: 0.5rem 2rem;
    display: grid;
    grid-template-columns: auto auto;
    row-gap: 0.2rem;
    --card-margin: 0.5rem;
  }
  p {
    margin: 0;
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
  .hand {
    width: calc((var(--card-width) + var(--card-margin)) * 9);
    display: flex;
  }
  .registers {
    width: calc((var(--card-width) + var(--card-margin)) * 5);
    display: flex;
  }

  .hand,
  .programmed {
    outline-offset: -2px;
  }
  .hand > img,
  .register,
  .programmed,
  .programmed > img {
    width: var(--card-width);
    height: auto;
  }
  .register,
  .hand > img {
    margin: 0 calc(var(--card-margin) / 2);
  }
  .register {
    position: relative;
  }
  .register span {
    position: absolute;
    /* centering: top/left moves top left corner of child relative to parent size, translate moves relative to child size */
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    font-size: 1.7rem;
    white-space: nowrap;
    color: black;
  }
  .programmed {
    border: 2px dashed black;
    height: 100%;
  }
  .programmed > img {
    position: absolute;
    top: 0;
    left: 0;
  }
  .programmed,
  .programmed > img,
  .hand > img {
    border-radius: 10px;
    box-sizing: border-box;
  }
</style>
