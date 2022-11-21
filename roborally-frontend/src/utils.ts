import { ParsedMap, parse_map } from "frontend-wasm";

const assets = import.meta.glob("./assets/textures/*.???", {
  eager: true,
  as: "url",
}) as Record<string, string>;

export function getTexture(name: string): string {
  return (
    assets["./assets/textures/" + name] ??
    (console.warn(`Unknown asset ${name}, using floor as fallback`),
    assets["./assets/textures/floor.jpg"])
  );
}

export async function fetchMap(name: string): Promise<ParsedMap> {
  const r = await fetch("/api/map?" + new URLSearchParams({ name }));
  return parse_map(new Uint8Array(await r.arrayBuffer()));
}
