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

export type CategoryType = 'all' | 'text' | 'link' | 'image';
