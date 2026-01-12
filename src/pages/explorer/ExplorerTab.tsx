import type { Component } from 'solid-js';
import { Pagination } from '../../components/Pagination/Pagination';
import { useExplorerTab } from './hooks/useExplorerTab';
import { ExplorerToolbar } from './components/ExplorerToolbar';
import { FolderGrid } from './components/FolderGrid';

// 型定義は types.ts に移動済み - 後方互換性のために再エクスポート
export type { TabState, ExplorerTabProps } from './types';

type Props = {
  tabKey: string;
  path?: string;
  transferPath?: string;
  isActiveTab: boolean;
};

export const ExplorerTab: Component<Props> = (props) => {
  const {
    transferPath,
    folders,
    pagination,
    isLoading,
    activeViewerDir,
    sortConfig,
    searchInput,
    selectTransferPath,
    onFolderClick,
    transferFolder,
    closeViewerTabsForDirectory,
    resetTab,
    movePage,
    moveForward,
    moveBackward,
    moveFirst,
    moveLast,
    handleSortChange,
    handleSearchInput,
  } = useExplorerTab(props.tabKey, () => props.isActiveTab);

  const handleMarkedAsRead = (path: string) => {
    const to = transferPath();
    if (!to) return;
    transferFolder(path, to);
    closeViewerTabsForDirectory(path);
  };

  return (
    <div class="h-full flex flex-col overflow-hidden">
      <ExplorerToolbar
        transferPath={transferPath()}
        sortConfig={sortConfig()}
        searchInput={searchInput()}
        onResetTab={resetTab}
        onSelectTransferPath={selectTransferPath}
        onSearchInput={handleSearchInput}
        onSortChange={handleSortChange}
      />
      <FolderGrid
        folders={folders()}
        isLoading={isLoading()}
        transferPath={transferPath()}
        activeViewerDir={activeViewerDir()}
        onFolderClick={onFolderClick}
        onMarkedAsRead={handleMarkedAsRead}
      />
      <div class="p-1 h-12 self-center">
        <Pagination
          current={pagination()[0]}
          end={pagination()[1]}
          onClickPrev={moveBackward}
          onClickNext={moveForward}
          onClickPage={movePage}
          onClickStart={moveFirst}
          onClickEnd={moveLast}
        />
      </div>
    </div>
  );
};
