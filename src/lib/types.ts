export type MediaType = 'image' | 'video';

export interface Author {
  id: string;
  username: string;
  displayName: string;
  avatarUrl?: string;
  avatarLocalPath?: string;
}

export interface Media {
  id: string;
  kind: MediaType;
  remoteUrl?: string;
  previewUrl?: string;
  localPath?: string;
  thumbnailPath?: string;
  width?: number;
  height?: number;
  durationMs?: number;
}

export interface Quote {
  id: string;
  text: string;
  canonicalUrl?: string;
  authorName?: string;
  username?: string;
}

export interface Tweet {
  id: string;
  text: string;
  archiveText?: string;
  createdAt?: string;
  likedAt?: string;
  canonicalUrl: string;
  enrichmentStatus: string;
  enrichmentError?: string;
  author?: Author;
  media: Media[];
  quote?: Quote;
}

export interface TweetQuery {
  search?: string;
  author?: string;
  mediaType?: MediaType;
  before?: string;
  after?: string;
  status?: string;
  offset: number;
  limit: number;
}

export interface TweetPage {
  items: Tweet[];
  total: number;
}

export interface Profile {
  id: string;
  username?: string;
  displayName: string;
  avatarUrl?: string;
  avatarLocalPath?: string;
  postCount: number;
}

export interface ProfileState {
  profiles: Profile[];
  activeProfileId?: string;
}

export interface ImportSummary {
  imported: number;
  existing: number;
  pending: number;
  failed: number;
}

export interface JobStatus {
  phase: string;
  processed: number;
  total: number;
  failed: number;
  message: string;
}

export interface Filters {
  author: string;
  mediaType: '' | MediaType;
  after: string;
  before: string;
  status: string;
}
