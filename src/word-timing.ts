/** Max span used to spread estimated word timings within one line (avoids huge instrumental gaps). */
export const MAX_LINE_SPREAD_MS = 7_000;

export interface TimedWord {
  text: string;
  startMs: number;
  endMs: number;
}

const ENHANCED_TAG_RE =
  /<\s*(\d{1,2}):(\d{2})(?:[.:](\d{2,3})|:(\d{2,3}))?\s*>/g;

export function parseEnhancedTimestamp(
  minutes: number,
  seconds: number,
  frac?: string,
): number {
  const base = minutes * 60_000 + seconds * 1000;
  if (!frac) return base;
  if (frac.length === 2) return base + Number(frac) * 10;
  if (frac.length === 3) return base + Number(frac);
  return base;
}

export interface EnhancedSegment {
  text: string;
  startMs: number | null;
}

/** Parse `<mm:ss.xx>` inline tags; returns display text pieces with optional start times. */
export function parseEnhancedSegments(rawLineText: string): EnhancedSegment[] {
  const segments: EnhancedSegment[] = [];
  let lastIndex = 0;
  let pendingStart: number | null = null;
  ENHANCED_TAG_RE.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = ENHANCED_TAG_RE.exec(rawLineText)) !== null) {
    if (match.index > lastIndex) {
      const chunk = rawLineText.slice(lastIndex, match.index);
      if (chunk) segments.push({ text: chunk, startMs: pendingStart });
    }
    pendingStart = parseEnhancedTimestamp(
      Number(match[1]),
      Number(match[2]),
      match[3] ?? match[4],
    );
    lastIndex = ENHANCED_TAG_RE.lastIndex;
  }
  if (lastIndex < rawLineText.length) {
    segments.push({ text: rawLineText.slice(lastIndex), startMs: pendingStart });
  }
  return segments;
}

export function stripLineTags(raw: string): string {
  return raw.replace(ENHANCED_TAG_RE, "").replace(/\s+/g, " ").trim();
}

export function syllableWeight(word: string): number {
  const w = word.trim();
  if (!w) return 0;
  if (/[\u0590-\u05FF]/.test(w)) {
    return Math.max(1, Math.ceil(w.length / 2.5));
  }
  const vowelGroups = w.toLowerCase().match(/[aeiouyáéíóúàèìòùäöü]+/g);
  const groups = vowelGroups?.length ?? 0;
  if (groups > 0) return Math.max(1, groups);
  return Math.max(1, Math.ceil(w.length / 3));
}

export function cappedLineEndMs(lineStartMs: number, nextLineStartMs: number): number {
  const gap = Math.max(0, nextLineStartMs - lineStartMs);
  const spread = Math.min(gap, MAX_LINE_SPREAD_MS);
  return lineStartMs + spread;
}

export function tokenizeLine(text: string): string[] {
  const tokens: string[] = [];
  const re = /(\S+\s*)/gu;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    tokens.push(m[1]);
  }
  return tokens.length ? tokens : [text];
}

function wordsFromEnhanced(
  rawText: string,
  _lineStartMs: number,
  lineEndMs: number,
): TimedWord[] | null {
  const segments = parseEnhancedSegments(rawText);
  const tagged = segments.filter((s) => s.startMs != null && s.text.trim());
  if (tagged.length < 2) return null;

  const words: TimedWord[] = [];
  for (let i = 0; i < tagged.length; i += 1) {
    const seg = tagged[i];
    const text = seg.text.trim();
    if (!text) continue;
    const startMs = seg.startMs!;
    const nextStart =
      i + 1 < tagged.length
        ? tagged[i + 1].startMs!
        : lineEndMs;
    words.push({
      text,
      startMs,
      endMs: Math.max(startMs + 40, Math.min(nextStart, lineEndMs)),
    });
  }
  return words.length ? words : null;
}

/** Estimate per-word timings for one lyric line. */
export function buildLineWordTimings(
  rawText: string,
  displayText: string,
  lineStartMs: number,
  nextLineStartMs: number,
): TimedWord[] {
  const lineEndMs = cappedLineEndMs(lineStartMs, nextLineStartMs);

  const enhanced = wordsFromEnhanced(rawText, lineStartMs, lineEndMs);
  if (enhanced?.length) return enhanced;

  const tokens = tokenizeLine(displayText || stripLineTags(rawText));
  const weights = tokens.map((t) => syllableWeight(t));
  const total = weights.reduce((a, b) => a + b, 0) || tokens.length;
  const vocalMs = lineEndMs - lineStartMs;

  const words: TimedWord[] = [];
  let cursor = lineStartMs;
  for (let i = 0; i < tokens.length; i += 1) {
    const share = vocalMs * (weights[i] / total);
    const end =
      i === tokens.length - 1 ? lineEndMs : Math.min(lineEndMs, cursor + share);
    words.push({
      text: tokens[i],
      startMs: Math.round(cursor),
      endMs: Math.round(Math.max(cursor + 30, end)),
    });
    cursor = end;
  }
  return words;
}

export function findActiveWordIndex(
  words: TimedWord[],
  timeMs: number,
): number {
  if (words.length === 0) return -1;
  for (let i = words.length - 1; i >= 0; i -= 1) {
    if (timeMs >= words[i].startMs) return i;
  }
  return -1;
}

export function wordProgress(words: TimedWord[], timeMs: number): number {
  const idx = findActiveWordIndex(words, timeMs);
  if (idx < 0) return 0;
  const w = words[idx];
  const dur = Math.max(1, w.endMs - w.startMs);
  return Math.max(0, Math.min(1, (timeMs - w.startMs) / dur));
}