export interface LrcLine {
  timeMs: number;
  text: string;
}

export interface ParsedLrc {
  offsetMs: number;
  lines: LrcLine[];
}

const TAG_RE =
  /\[(\d{1,2}):(\d{2})(?:[.:](\d{2,3})|:(\d{2,3}))?\]/g;

/** Strip enhanced-LRC word timestamps for on-screen display (kept in raw for P2). */
export function stripEnhancedTags(text: string): string {
  return text
    .replace(/<\d{1,2}:\d{2}[.:]\d{2,3}>/g, "")
    .replace(/<\d{1,2}:\d{2}:\d{2}>/g, "")
    .trim();
}

export function parseTimestamp(minutes: number, seconds: number, frac?: string): number {
  const base = minutes * 60_000 + seconds * 1000;
  if (!frac) return base;
  if (frac.length === 2) return base + Number(frac) * 10;
  if (frac.length === 3) return base + Number(frac);
  return base;
}

export function parseLrc(content: string): ParsedLrc {
  let offsetMs = 0;
  const lines: LrcLine[] = [];

  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (!line.trim()) continue;

    const offsetMatch = line.match(/^\[offset:\s*([+-]?\d+)\s*\]/i);
    if (offsetMatch) {
      offsetMs = Number(offsetMatch[1]);
      continue;
    }

    const tags: number[] = [];
    let match: RegExpExecArray | null;
    TAG_RE.lastIndex = 0;
    while ((match = TAG_RE.exec(line)) !== null) {
      const mins = Number(match[1]);
      const secs = Number(match[2]);
      const frac = match[3] ?? match[4];
      if (!Number.isNaN(mins) && !Number.isNaN(secs)) {
        tags.push(parseTimestamp(mins, secs, frac));
      }
    }

    if (tags.length === 0) continue;

    let text = line.replace(TAG_RE, "").trim();
    text = stripEnhancedTags(text);
    if (!text) continue;

    for (const timeMs of tags) {
      lines.push({ timeMs: timeMs + offsetMs, text });
    }
  }

  lines.sort((a, b) => a.timeMs - b.timeMs);
  return { offsetMs, lines };
}

export function findActiveLineIndex(lines: LrcLine[], timeMs: number): number {
  if (lines.length === 0) return -1;
  let lo = 0;
  let hi = lines.length - 1;
  let best = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].timeMs <= timeMs) {
      best = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return best;
}