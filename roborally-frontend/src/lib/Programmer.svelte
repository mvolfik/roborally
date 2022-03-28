<script lang="ts">
  import type { CardWrapper } from "../../frontend-wasm";
  import { dndzone, type DndEvent } from "svelte-dnd-action";
  import { createEventDispatcher, onMount } from "svelte";
  import { getCardAsset } from "./utils";
  import { flip } from "svelte/animate";

  export let seat: number;
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

  function handleDnd(e: CustomEvent, register_i?: number) {
    if (register_i === undefined) {
      cardsInHand = e.detail.items;
    } else {
      programmedCards[register_i] = e.detail.items;
    }
  }
  let eventSource = createEventDispatcher();
</script>

<div class="outer" style:--player-i={seat}>
  <p class="hand-title">Cards in your hand</p>
  <p class="registers-title">
    Programmed registers
    <button
      disabled={ programmedCards.some((c) => c.length !== 1)}
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
    on:consider={(e) => handleDnd(e)}
    on:finalize={(e) => handleDnd(e)}
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
            dropFromOthersDisabled: maybeCard.some((x) => !x.isDndShadowItem),
          }}
          on:consider={(e) => handleDnd(e, i)}
          on:finalize={(e) => handleDnd(e, i)}
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
    width: var(--programmer-width);
    padding: 10px 20px 20px;

    --programmer-inner-width: calc(var(--programmer-width) - 2 * 20px);

    background-color: hsla(calc(228 + var(--player-i) * 0.7 / 3.9 * 360), 93%, 22%, 0.62);
    border-radius: 20px 20px 0 0;
    box-sizing: border-box;
    display: grid;
    grid-template-columns: auto auto;
    row-gap: 10px;
    --card-gap: 10px;
  }
  p {
    margin: 0;
  }
  .hand-title,
  .registers-title,
  .registers-title button {
    color: #eee;
    text-align: center;
    font-size: 1.5vw;
  }
  .registers-title button {
    background-color: rgb(0, 68, 23);
    border-radius: 7px;
    padding: 2px 5px;
  }
  .registers-title button:disabled {
    background-color: #555;
  }
  .registers-title button:not(:disabled):hover {
    background-color: rgb(0, 46, 15);
    cursor: pointer;
  }
  .hand {
    width: calc(var(--programmer-inner-width) / 14 * 9);
    display: flex;
  }
  .registers {
    width: calc(var(--programmer-inner-width) / 14 * 5);
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
    width: calc(var(--programmer-inner-width) / 14 - var(--card-gap));
    height: calc(
      (var(--programmer-inner-width) / 14 - var(--card-gap)) / 140 * 200
    );
  }
  .register,
  .hand > img {
    margin: 0 calc(var(--card-gap) / 2);
  }
  .register {
    position: relative;
  }
  .register span {
    position: absolute;
    top: 50%;
    left: 50%;
    /* centering: translate is relative to self size, top/left moves top left corner relative to parent */
    transform: translate(-50%, -50%);
    font-size: 2vw;
    white-space: nowrap;
    color: black;
  }
  .programmed {
    border: 2px dashed black;
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
