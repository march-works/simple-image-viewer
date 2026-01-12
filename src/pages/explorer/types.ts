import type { Thumbnail } from '../../features/Folder/types/Thumbnail';
import type { SortConfig } from '../../features/Explorer/types/ExplorerQuery';

export type TabState = {
  title: string;
  key: string;
  path?: string;
  transfer_path?: string;
  page: number;
  end: number;
  folders: Thumbnail[];
  sort?: SortConfig;
  search_query?: string;
};

export type ExplorerTabProps = {
  tabKey: string;
  path?: string;
  transferPath?: string;
  isActiveTab: boolean;
};
