import { parseSearchResults } from "../src/parse/search-results";
import { parseAvailability } from "../src/parse/availability";
const search = await Bun.file(new URL("../fixtures/search-allwrk.html", import.meta.url)).text();
const frag = await Bun.file(new URL("../fixtures/record-recfmt.html", import.meta.url)).text();

function bench(name: string, fn: () => void, iters: number) {
  for (let i = 0; i < 50; i++) fn();            // warmup
  const t0 = performance.now();
  for (let i = 0; i < iters; i++) fn();
  const dt = performance.now() - t0;
  console.log(`${name.padEnd(24)} ${(dt / iters).toFixed(3)} ms/op   ${Math.round(iters / (dt / 1000)).toLocaleString()} ops/s`);
}

bench(`search page (204KB, 10)`, () => parseSearchResults(search), 1000);
bench(`availability fragment`, () => parseAvailability(frag), 5000);
