import { onDestroy } from 'svelte';
import type {
  Author, ImportSummary, JobStatus, Media, MediaType, Playlist, ProfileState, SyncInfo, Tweet, TweetQuery
} from './lib/types';
import { getImportStatus, getPlaylists, getProfiles, getSyncStatus, getTweets } from './api';

/** The three sections in the left nav; everything else opens on top of one. */
export type Section = 'home' | 'search' | 'lists';

export type View =
  | { kind: 'post'; tweet: Tweet }
  | { kind: 'profile'; author: Author }
  | { kind: 'list'; playlist: Playlist };

/** How the Latest tab sorts and filters (the desktop's old sort toggle and filter panel). */
export interface LatestOptions {
  sort: 'newest' | 'oldest';
  media: '' | MediaType;
  author: string;
  after: string;
  before: string;
  status: string;
}

export const defaultLatest: LatestOptions = { sort: 'newest', media: '', author: '', after: '', before: '', status: '' };
export const filterCount = (o: LatestOptions) =>
  [o.media, o.author.trim(), o.after, o.before, o.status].filter(Boolean).length;

const LATEST_KEY = 'kb.desktop.latest';

function loadLatest(): LatestOptions {
  try {
    const saved = JSON.parse(localStorage.getItem(LATEST_KEY) ?? 'null');
    if (saved && typeof saved === 'object') return { ...defaultLatest, ...saved };
  } catch {
    // Storage unavailable or corrupt: start from the defaults.
  }
  return { ...defaultLatest };
}

export function message(value: unknown): string {
  if (value instanceof Error) return value.message;
  return typeof value === 'string' ? value : 'Something went wrong.';
}

export const newSeed = () => Math.floor(Math.random() * 2 ** 31);

class Desk {
  section = $state<Section>('home');
  homeTab = $state<'random' | 'newest'>('random');
  /** The middle column's scroll container (set by App). */
  scroller: HTMLElement | undefined;
  stack = $state<View[]>([]);
  profiles = $state<ProfileState>({ profiles: [] });
  playlists = $state<Playlist[]>([]);
  playlistError = $state<string>();
  job = $state<JobStatus>();
  importing = $state(false);
  importSummary = $state<ImportSummary>();
  importError = $state<string>();
  sync = $state<SyncInfo>();
  latest = $state<LatestOptions>(loadLatest());
  /** The search box, shared by the right column and the Search section. */
  search = $state('');
  phoneOpen = $state(false);
  listDialog = $state<{ playlist?: Playlist; addTweet?: string }>();
  toast = $state<string>();
  viewer = $state<{ media: Media[]; index: number }>();
  /** Bumped when the library changed underneath every feed (account switch, import). */
  libraryVersion = $state(0);
  #toastTimer: ReturnType<typeof setTimeout> | undefined;

  get top(): View | undefined {
    return this.stack.at(-1);
  }

  get activeProfile() {
    return this.profiles.profiles.find((p) => p.id === this.profiles.activeProfileId) ?? this.profiles.profiles[0];
  }

  get likeCount() {
    return this.activeProfile?.postCount ?? 0;
  }

  open(view: View) {
    this.stack = [...this.stack, view];
  }

  openProfile(author: Author | undefined) {
    if (!author?.username) return;
    const top = this.top;
    if (top?.kind === 'profile' && top.author.username.toLowerCase() === author.username.toLowerCase()) return;
    this.open({ kind: 'profile', author });
  }

  back(): boolean {
    if (!this.stack.length) return false;
    this.stack = this.stack.slice(0, -1);
    return true;
  }

  go(section: Section) {
    this.section = section;
    this.stack = [];
  }

  scrollTop(smooth = true) {
    this.scroller?.scrollTo({ top: 0, behavior: smooth ? 'smooth' : 'auto' });
  }

  setLatest(next: LatestOptions) {
    this.latest = next;
    try {
      localStorage.setItem(LATEST_KEY, JSON.stringify(next));
    } catch {
      // Not saved; it still applies until the app closes.
    }
  }

  showToast(text: string) {
    this.toast = text;
    clearTimeout(this.#toastTimer);
    this.#toastTimer = setTimeout(() => (this.toast = undefined), 2600);
  }

  async refreshProfiles() {
    this.profiles = await getProfiles();
  }

  async refreshPlaylists() {
    try {
      this.playlists = await getPlaylists();
      this.playlistError = undefined;
      // A list deleted elsewhere (an agent, another window) closes its page.
      this.stack = this.stack.filter((v) => v.kind !== 'list' || this.playlists.some((p) => p.id === v.playlist.id));
    } catch (cause) {
      this.playlistError = message(cause);
    }
  }

  replacePlaylist(updated: Playlist) {
    const exists = this.playlists.some((p) => p.id === updated.id);
    this.playlists = exists ? this.playlists.map((p) => (p.id === updated.id ? updated : p)) : [updated, ...this.playlists];
    this.stack = this.stack.map((v) => (v.kind === 'list' && v.playlist.id === updated.id ? { kind: 'list', playlist: updated } : v));
  }

  async refreshJob() {
    try {
      const previous = this.job;
      const next = await getImportStatus();
      this.job = next;
      // Posts are being fetched from X: show the ones on screen as they arrive.
      if (next.processed !== previous?.processed || next.phase !== previous?.phase) {
        for (const feed of feeds) void feed.refreshLoaded();
      }
    } catch {
      // Status polling stays quiet; feeds report real problems.
    }
  }

  async refreshSync() {
    try {
      this.sync = await getSyncStatus();
    } catch (cause) {
      this.sync = { phone: false, error: message(cause) };
    }
  }

  /** A post was just saved to a new list: mark it wherever it's shown. */
  markListed(tweetId: string, playlistId: number) {
    const shown =
      [...feeds].flatMap((f) => f.items).find((t) => t.id === tweetId) ??
      this.stack.flatMap((v) => (v.kind === 'post' ? [v.tweet] : [])).find((t) => t.id === tweetId);
    if (shown && !shown.playlistIds.includes(playlistId)) this.patchTweet({ ...shown, playlistIds: [...shown.playlistIds, playlistId] });
  }

  /** A post changed (lists, retry): update it in every open feed and page. */
  patchTweet(tweet: Tweet) {
    for (const feed of feeds) feed.patch(tweet);
    this.stack = this.stack.map((v) => (v.kind === 'post' && v.tweet.id === tweet.id ? { kind: 'post', tweet } : v));
  }
}

export const desk = new Desk();

type Base = Omit<TweetQuery, 'offset' | 'limit'>;
const PAGE = 30;
const feeds = new Set<Feed>();

/** A paged list of posts for one query; `reset` starts over, `more` loads the next page. */
export class Feed {
  items = $state<Tweet[]>([]);
  total = $state(0);
  loading = $state(false);
  error = $state<string>();
  done = $state(false);
  started = $state(false);
  #generation = 0;
  #query: () => Base;

  constructor(query: () => Base) {
    this.#query = query;
  }

  async reset() {
    this.#generation += 1;
    this.items = [];
    this.total = 0;
    this.done = false;
    this.error = undefined;
    this.loading = false;
    this.started = true;
    await this.more();
  }

  /** Load the first page again, keeping the current posts on screen until it arrives. */
  async refresh() {
    this.#generation += 1;
    const generation = this.#generation;
    this.loading = true;
    try {
      const page = await getTweets({ ...this.#query(), offset: 0, limit: PAGE });
      if (generation !== this.#generation) return;
      this.items = page.items;
      this.total = page.total;
      this.done = page.items.length < PAGE || page.items.length >= page.total;
      this.error = undefined;
      this.started = true;
    } catch (cause) {
      if (generation === this.#generation) this.error = message(cause);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  async more() {
    if (this.loading || this.done || this.error) return;
    const generation = this.#generation;
    this.loading = true;
    try {
      const page = await getTweets({ ...this.#query(), offset: this.items.length, limit: PAGE });
      if (generation !== this.#generation) return;
      const seen = new Set(this.items.map((t) => t.id));
      this.items = [...this.items, ...page.items.filter((t) => !seen.has(t.id))];
      this.total = page.total;
      this.done = page.items.length < PAGE || this.items.length >= page.total;
    } catch (cause) {
      if (generation === this.#generation) this.error = message(cause);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  /** Reload the posts already shown (while some are still waiting to be fetched from X). */
  async refreshLoaded() {
    if (this.loading || !this.items.some((t) => t.enrichmentStatus === 'pending')) return;
    const generation = this.#generation;
    const target = this.items.length;
    const fresh: Tweet[] = [];
    let total = this.total;
    try {
      for (let offset = 0; offset < target; offset += 100) {
        const page = await getTweets({ ...this.#query(), offset, limit: Math.min(100, target - offset) });
        fresh.push(...page.items);
        total = page.total;
        if (page.items.length === 0) break;
      }
      if (generation === this.#generation && !this.loading) {
        this.items = fresh;
        this.total = total;
      }
    } catch {
      // A background refresh never replaces a working feed with an error.
    }
  }

  retry() {
    this.error = undefined;
    void this.more();
  }

  patch(tweet: Tweet) {
    if (this.items.some((t) => t.id === tweet.id)) this.items = this.items.map((t) => (t.id === tweet.id ? tweet : t));
  }
}

/** A feed that `desk.patchTweet` keeps current while its component is mounted. */
export function useFeed(query: () => Base): Feed {
  const feed = new Feed(query);
  feeds.add(feed);
  onDestroy(() => feeds.delete(feed));
  return feed;
}
