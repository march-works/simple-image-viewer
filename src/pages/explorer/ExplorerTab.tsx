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
    divRef,
    actions,
  } = useExplorerTab(props.tabKey, props.isActiveTab);

  return (
    <div class="h-full flex flex-col overflow-hidden">
      <ExplorerToolbar
        transferPath={transferPath()}
        sortConfig={sortConfig()}
        searchInput={searchInput()}
        onResetTab={actions.resetTab}
        onSelectTransferPath={actions.selectTransferPath}
        onSearchInput={actions.handleSearchInput}
        onSortChange={actions.handleSortChange}
      />
      <FolderGrid
        folders={folders()}
        isLoading={isLoading()}
        transferPath={transferPath()}
        activeViewerDir={activeViewerDir()}
        onFolderClick={actions.onClick}
        onMarkedAsRead={actions.handleMarkedAsRead}
        divRef={divRef}
      />
      <div class="p-1 h-12 self-center">
        <Pagination
          current={pagination()[0]}
          end={pagination()[1]}
          onClickPrev={actions.moveBackward}
          onClickNext={actions.moveForward}
          onClickPage={actions.movePage}
          onClickStart={actions.moveFirst}
          onClickEnd={actions.moveLast}
        />
      </div>
    </div>
  );
};
