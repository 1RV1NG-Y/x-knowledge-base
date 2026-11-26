import type { Author, Media, MediaType, Playlist, ProfileState, SyncInfo, Tweet, TweetQuery } from '../../src/lib/types';
import { getPlaylists, getProfiles, getSyncStatus, getTweets } from './api';

export type Tab = 'home' | 'search' | 'lists' | 'sync';

export type Overlay =
  | { kind: 'post'; tweet: Tweet }
  | { kind: 'viewer'; media: Media[]; index: number }
  | { kind: 'playlist'; playlist: Playlist }
  | { kind: 'drawer' }
  | { kind: 'actions'; tweet: Tweet }
  | { kind: 'latestOptions' }
  | { kind: 'profile'; author: Author };

/** How the Latest tab sorts and filters, like the desktop's sort toggle and filter panel. */
export interface LatestOptions {
  sort: 'newest' | 'oldest';
  media: '' | MediaType;
  author: string;
  after: string;
  before: string;
  status: string;
}

export const defaultLatest: LatestOptions = { sort: 'newest', media: '', author: '', after: '', before: '', status: '' };

/** Filters in use, not counting the sort. */
export const filterCount = (o: LatestOptions) =>
  [o.media, o.author.trim(), o.after, o.before, o.status].filter(Boolean).length;

const LATEST_KEY = 'kb.latestOptions';

function loadLatest(): LatestOptions {
  try {
    const saved = JSON.parse(localStorage.getItem(LATEST_KEY) ?? 'null');
    if (saved && typeof saved === 'object') return { ...defaultLatest, ...saved };
  } catch {
    // Storage unavailable or corrupt: start from the defaults.
  }
  return { ...defaultLatest };
}

class AppState {
  tab = $state<Tab>('home');
  overlays = $state<Overlay[]>([]);
  profiles = $state<ProfileState>({ profiles: [] });
  playlists = $state<Playlist[]>([]);
  sync = $state<SyncInfo>();
  /** Bumped when a sync replaced the library, so open feeds can offer a refresh. */
  libraryVersion = $state(0);
  toast = $state<string>();
  latest = $state<LatestOptions>(loadLatest());
  #toastTimer: ReturnType<typeof setTimeout> | undefined;

  get activeProfile() {
    return this.profiles.profiles.find((p) => p.id === this.profiles.activeProfileId) ?? this.profiles.profiles[0];
  }

  push(overlay: Overlay) {
    this.overlays = [...this.overlays, overlay];
  }

  /** Their liked posts. Opening the profile you're already on does nothing. */
  openProfile(author: Author | undefined) {
    if (!author?.username) return;
    const top = this.overlays.at(-1);
    if (top?.kind === 'profile' && top.author.username?.toLowerCase() === author.username.toLowerCase()) return;
    this.push({ kind: 'profile', author });
  }

  pop(): boolean {
    if (!this.overlays.length) return false;
    this.overlays = this.overlays.slice(0, -1);
    return true;
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

  async refreshLibrary() {
    const [profiles, playlists] = await Promise.all([getProfiles(), getPlaylists()]);
    this.profiles = profiles;
    this.playlists = playlists;
  }

  async refreshSync() {
    try {
      this.sync = await getSyncStatus();
    } catch (cause) {
      this.sync = { phone: true, error: message(cause) };
    }
  }
}

export const app = new AppState();

export function message(value: unknown): string {
  if (value instanceof Error) return value.message;
  return typeof value === 'string' ? value : 'Something went wrong.';
}

export const newSeed = () => Math.floor(Math.random() * 2 ** 31);

type Base = Omit<TweetQuery, 'offset' | 'limit'>;
const PAGE = 20;

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
      this.items = [...this.items, ...page.items];
      this.total = page.total;
      this.done = page.items.length < PAGE || this.items.length >= page.total;
    } catch (cause) {
      if (generation === this.#generation) this.error = message(cause);
    } finally {
      if (generation === this.#generation) this.loading = false;
    }
  }

  retry() {
    this.error = undefined;
    void this.more();
  }
}
