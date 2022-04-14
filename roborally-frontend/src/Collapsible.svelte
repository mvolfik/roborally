<script context="module" lang="ts">
  import { Readable, readable, Writable, writable } from "svelte/store";
  const globalExpandedList: Set<Writable<boolean>> = new Set();
  const onlyShowOne: Readable<boolean> = readable(null, (set) => {
    const update = (e: Pick<MediaQueryListEvent, "matches">) => {
      set(e.matches);
    };
    const query = window.matchMedia("(max-width: 40rem), (max-height: 40rem)");
    query.addEventListener("change", update);
    update({ matches: query.matches });
    return () => query.removeEventListener("change", update);
  });
</script>

<script lang="ts">
  import { onMount } from "svelte";

  import { elasticOut } from "svelte/easing";
  import { fly, type TransitionConfig } from "svelte/transition";

  import unfoldLess from "./assets/unfold_less.svg?url";
  import unfoldMore from "./assets/unfold_more.svg?url";
  export let side: "top" | "right" | "bottom" | "left";
  export let expandedStore = writable(false);
  export let label: string;
  /** If this value is provided, whenever it changes, the component is re-created */
  export let key: any = undefined;
  export let noBackground = false;

  onMount(() => {
    globalExpandedList.add(expandedStore);
    let unsub = expandedStore.subscribe((newExpanded) => {
      if (newExpanded && $onlyShowOne) {
        for (const otherExpanded of globalExpandedList) {
          if (otherExpanded !== expandedStore) {
            // skip self
            otherExpanded.set(false);
          }
        }
      }
    });
    return () => {
      unsub();
      globalExpandedList.delete(expandedStore);
    };
  });

  const duration = 200;

  function flippyTransition(el: HTMLElement, out: boolean): TransitionConfig {
    // from svelte docs
    const baseTransform = getComputedStyle(el).transform.replace("none", "");

    return {
      // wait for slide, wait a bit, play out, wait a bit, play in
      delay: (out ? 0.5 : 2) * duration,
      duration: out ? duration : duration * 3,
      easing: out ? undefined : elasticOut,
      css: (t: number) => `transform: ${baseTransform} scaleY(${t * 100}%)`,
    };
  }
</script>

{#key key}
  <div
    class="wrapper"
    class:top={side === "top"}
    class:right={side === "right"}
    class:bottom={side === "bottom"}
    class:left={side === "left"}
    class:collapsed={!$expandedStore}
    style:transition-duration="{duration}ms"
    transition:fly={side === "top"
      ? { y: -200 }
      : side === "right"
      ? { x: 200 }
      : side === "bottom"
      ? { y: 200 }
      : { x: -200 }}
  >
    {#if $expandedStore}
      <button
        class="marker"
        on:click={() => ($expandedStore = false)}
        in:flippyTransition={false}
        out:flippyTransition={true}
      >
        <img src={unfoldLess} alt="Collapse" />
        Hide
      </button>
    {:else}
      <button
        class="marker"
        on:click={() => ($expandedStore = true)}
        in:flippyTransition={false}
        out:flippyTransition={true}
      >
        <img src={unfoldMore} alt="Uncollapse" />
        {label}
      </button>
    {/if}
    <div class="inner" class:noBackground>
      <slot />
    </div>
  </div>
{/key}

<style>
  .wrapper {
    position: absolute;
    transition: transform ease-out;
  }
  .inner {
    overflow: hidden;
  }
  .inner:not(.noBackground) {
    background-color: var(--seat-color);
    color: white;
  }
  .marker {
    position: absolute;
    border: none;
    background-color: var(--seat-color);
    color: white;
    padding: 0.25rem 0.7rem 0.25rem 0.25rem;
    display: flex;
    align-items: center;
    white-space: nowrap;
    /* transition: padding 0.2s ease-in-out; */
  }
  .marker:hover {
    filter: brightness(0.9);
    /* padding: 1rem 1rem 1rem 0.5rem; */
  }

  .wrapper:is(.right, .left) .inner {
    max-height: 90vh;
    overflow-y: auto;
  }

  .wrapper:is(.top, .bottom) .inner {
    max-width: 90vw;
    overflow-x: auto;
  }

  .wrapper.top {
    top: 0;
    left: 50%;
    transform: translateX(-50%);
  }
  .wrapper.top.collapsed {
    transform: translateX(-50%) translateY(-100%);
  }
  .wrapper.top .inner {
    border-radius: 0 0 5px 5px;
  }
  .wrapper.top .marker {
    border-radius: 0 0 5px 5px;
    border-top: 1px solid black;
    transform-origin: 0 0;
    transform: translateX(-50%);
    left: 50%;
    top: 100%;
  }

  .wrapper.right {
    right: 0;
    top: 3rem;
  }
  .wrapper.right.collapsed {
    transform: translateX(100%);
  }
  .wrapper.right .inner {
    border-bottom-left-radius: 5px;
  }
  .wrapper.right .marker {
    border-radius: 5px 5px 0 0;
    border-bottom: 1px solid black;
    transform-origin: 100% 100%;
    transform: translateY(-100%) rotate(-90deg);
    right: 100%;
    top: 0;
  }

  .wrapper.bottom {
    bottom: 0;
    right: 1rem;
  }
  .wrapper.bottom.collapsed {
    transform: translateY(100%);
  }
  .wrapper.bottom .inner {
    border-top-left-radius: 5px;
  }
  .wrapper.bottom .marker {
    border-radius: 5px 5px 0 0;
    border-bottom: 1px solid black;
    transform-origin: 0 100%;
    bottom: 100%;
    right: 0;
  }

  .wrapper.left {
    left: 0;
    top: 3rem;
  }
  .wrapper.left.collapsed {
    transform: translateX(-100%);
  }
  .wrapper.left .inner {
    border-bottom-right-radius: 5px;
  }
  .wrapper.left .marker {
    border-radius: 0 0 5px 5px;
    border-top: 1px solid black;
    transform-origin: 0 0;
    transform: rotate(-90deg) translateX(-100%);
    left: 100%;
    top: 0;
  }
</style>
