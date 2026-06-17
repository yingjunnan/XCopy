export interface ClipboardEntry {
  id: string;
  contentType: 'text' | 'link' | 'image';
  content: string;
  sourceApp: string;
  preview: string;
  createdAt: string;
  imagePath?: string;
}

export interface ClipboardFilter {
  query?: string;
  contentType?: string;
  limit?: number;
  offset?: number;
}

export interface AppSettings {
  autoStart: boolean;
  shortcut: string;
  maxHistoryEntries: number;
  retentionDays: number;
}

export interface StorageUsage {
  databaseBytes: number;
  imagesBytes: number;
}

export type CategoryType = 'all' | 'text' | 'link' | 'image';
