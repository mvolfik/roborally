import { AssetMap, parse_map } from "frontend-wasm";

const assets = import.meta.globEager("../assets/textures/*.png", {
  assert: { type: "url" },
}) as Record<string, { default: string }>;

export function getTexture(name: string): string {
  return (
    assets["../assets/textures/" + name]?.default ??
    (console.warn(`Unknown asset ${name}, using floor as fallback`),
    assets["../assets/textures/floor.png"].default)
  );
}

const cardAssets = import.meta.globEager("../assets/cards/*.png", {
  assert: { type: "url" },
}) as Record<string, { default: string }>;

export function getCardAsset(name: string): string {
  return (
    cardAssets["../assets/cards/" + name]?.default ??
    (console.warn(`Unknown card ${name}, using Again as fallback`),
    cardAssets["../assets/cards/again.png"].default)
  );
}

export async function fetchMap(name: string): Promise<AssetMap> {
  const r = await fetch("/api/map?" + new URLSearchParams({ name }));
  return parse_map(new Uint8Array(await r.arrayBuffer()));
}
