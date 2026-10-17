import type { OutlineItem } from "../lib/types";

const RE_HEADING = /^(#{1,6})\s+(.*)$/;
const RE_FENCE = /^```/;

export function parseOutline(text: string): OutlineItem[] {
  const items: OutlineItem[] = [];
  let inFence = false;
  let pos = 0;
  for (const line of text.split("\n")) {
    if (RE_FENCE.test(line)) inFence = !inFence;
    if (!inFence) {
      const m = RE_HEADING.exec(line);
      if (m) items.push({ level: m[1].length, text: m[2].trim(), pos });
    }
    pos += line.length + 1;
  }
  return items;
}
