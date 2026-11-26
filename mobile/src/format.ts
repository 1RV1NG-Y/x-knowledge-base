import type { Tweet } from '../../src/lib/types';

const short = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric' });
const shortYear = new Intl.DateTimeFormat(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
const time = new Intl.DateTimeFormat(undefined, { hour: 'numeric', minute: '2-digit' });

function parse(value?: string): Date | undefined {
  if (!value) return undefined;
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? undefined : date;
}

/** "45s", "12m", "3h", "Mar 4", "Mar 4, 2021" — like a timeline. */
export function relativeTime(value?: string): string {
  const date = parse(value);
  if (!date) return '';
  const seconds = Math.max(0, (Date.now() - date.valueOf()) / 1000);
  if (seconds < 60) return `${Math.floor(seconds)}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h`;
  return date.getFullYear() === new Date().getFullYear() ? short.format(date) : shortYear.format(date);
}

/** "3:41 PM · Jan 4, 2024" — like a post's own page. */
export function fullTime(value?: string): string {
  const date = parse(value);
  return date ? `${time.format(date)} · ${shortYear.format(date)}` : '';
}

export function longDate(value?: string): string {
  const date = parse(value);
  return date ? shortYear.format(date) : '';
}

export function postText(tweet: Tweet): string {
  return (tweet.text || tweet.archiveText || '').trim();
}

export type Segment = { kind: 'text' | 'link' | 'mention' | 'tag'; text: string; href?: string };

const TOKEN = /(https?:\/\/[^\s]+)|(@[A-Za-z0-9_]{1,15})|(#[\p{L}\p{N}_]+)/gu;

/** Splits post text into plain runs and links, mentions and hashtags. */
export function segments(text: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const match of text.matchAll(TOKEN)) {
    const index = match.index ?? 0;
    if (index > last) out.push({ kind: 'text', text: text.slice(last, index) });
    const [token, url, mention] = match;
    if (url) {
      const trimmed = url.replace(/[.,!?)\]]+$/, '');
      const display = trimmed.replace(/^https?:\/\/(www\.)?/, '');
      out.push({ kind: 'link', text: display.length > 32 ? `${display.slice(0, 31)}…` : display, href: trimmed });
      if (trimmed.length < url.length) out.push({ kind: 'text', text: url.slice(trimmed.length) });
    } else if (mention) {
      out.push({ kind: 'mention', text: token, href: `https://x.com/${token.slice(1)}` });
    } else {
      out.push({ kind: 'tag', text: token, href: `https://x.com/hashtag/${encodeURIComponent(token.slice(1))}` });
    }
    last = index + token.length;
  }
  if (last < text.length) out.push({ kind: 'text', text: text.slice(last) });
  return out;
}

export function duration(ms?: number): string {
  if (!ms) return '';
  const total = Math.round(ms / 1000);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, '0')}`;
}

export function bytes(n: number): string {
  if (n < 1e6) return `${Math.round(n / 1e3)} KB`;
  if (n < 1e9) return `${(n / 1e6).toFixed(n < 1e7 ? 1 : 0)} MB`;
  return `${(n / 1e9).toFixed(2)} GB`;
}

export function ago(iso?: string): string {
  const date = parse(iso);
  if (!date) return 'never';
  const minutes = Math.round((Date.now() - date.valueOf()) / 60000);
  if (minutes < 1) return 'just now';
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return longDate(iso);
}
