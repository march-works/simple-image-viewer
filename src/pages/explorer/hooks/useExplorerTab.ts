import { createSignal, onCleanup, onMount } from 'solid-js';
import { debounce } from '@solid-primitives/scheduled';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { Thumbnail } from '../../../features/Folder/types/Thumbnail';
import {
  SortConfig,
  defaultSortConfig,
  sortOptions,
} from '../../../features/Explorer/types/ExplorerQuery';
import type { TabState } from '../types';

const appWindow = getCurrentWebviewWindow();

export const useExplorerTab = (tabKey: string, isActiveTab: () => boolean) => {
  const [transferPath, setTransferPath] = createSignal<string>();
  const [folders, setFolders] = createSignal<Thumbnail[]>([]);
  const [pagination, setPagination] = createSignal<[number, number]>([1, 1]);
  const [isLoading, setIsLoading] = createSignal<boolean>(false);
  const [activeViewerDir, setActiveViewerDir] = createSignal<
    string | undefined
  >();
  const [sortConfig, setSortConfig] =
    createSignal<SortConfig>(defaultSortConfig);
  const [searchInput, setSearchInput] = createSignal<string>('');

  let unListenRef: UnlistenFn | undefined = undefined;
  let activeViewerDirListenRef: UnlistenFn | undefined = undefined;

  // デバウンスされた検索実行関数
  const debouncedSearch = debounce((value: string) => {
    setIsLoading(true);
    invoke('change_explorer_search', {
      label: appWindow.label,
      key: tabKey,
      query: value || null,
    });
  }, 300);

  // アクティブなViewerのアクティブなタブのディレクトリを取得
  const updateActiveViewerDirectory = async () => {
    try {
      const dir = await invoke<string | null>('get_active_viewer_directory');
      setActiveViewerDir(dir ?? undefined);
    } catch {
      setActiveViewerDir(undefined);
    }
  };

  onMount(async () => {
    // タブ状態変更イベントリスナー
    unListenRef = await appWindow.listen(
      'explorer-tab-state-changed',
      (event) => {
        const {
          key,
          transfer_path: transferPathValue,
          page,
          end,
          folders: foldersValue,
          sort,
          search_query,
        } = event.payload as TabState;
        if (key !== tabKey) return;
        setPagination([page, end]);
        setTransferPath(transferPathValue);
        setFolders(foldersValue);
        if (sort) {
          setSortConfig(sort);
        }
        setSearchInput(search_query ?? '');
        setIsLoading(false);
      },
    );

    // アクティブなViewerディレクトリ変更イベントリスナー
    activeViewerDirListenRef = await appWindow.listen(
      'active-viewer-directory-changed',
      (event) => {
        const dir = event.payload as string | null;
        setActiveViewerDir(dir ?? undefined);
      },
    );

    // 初回読み込み
    invoke('request_restore_explorer_tab_state', {
      label: appWindow.label,
      key: tabKey,
    });

    // 初回のアクティブディレクトリを取得
    updateActiveViewerDirectory();
  });

  // ナビゲーション
  const selectTransferPath = async () => {
    const dir = await open({ directory: true });
    if (Array.isArray(dir) || !dir) return;
    await invoke('change_explorer_transfer_path', {
      transferPath: dir,
      key: tabKey,
      label: appWindow.label,
    });
  };

  const onFolderClick = (thumb: Thumbnail) => {
    if (thumb.thumbpath) {
      invoke('open_new_viewer_tab', { path: thumb.thumbpath });
    } else {
      invoke('change_explorer_path', {
        path: thumb.path,
        label: appWindow.label,
        key: tabKey,
      });
    }
  };

  const transferFolder = async (from: string, to: string) => {
    await invoke('transfer_folder', { from, to, label: appWindow.label });
  };

  const closeViewerTabsForDirectory = async (directory: string) => {
    try {
      await invoke('close_viewer_tabs_by_directory', { directory });
    } catch (error) {
      console.error('Failed to close viewer tabs:', error);
    }
  };

  const resetTab = () => {
    invoke('reset_explorer_tab', { label: appWindow.label, key: tabKey });
  };

  // ページネーション
  const movePage = (page: number) => {
    setIsLoading(true);
    invoke('change_explorer_page', {
      label: appWindow.label,
      key: tabKey,
      page,
    });
  };

  const moveForward = () => {
    setIsLoading(true);
    invoke('move_explorer_forward', { label: appWindow.label, key: tabKey });
  };

  const moveBackward = () => {
    setIsLoading(true);
    invoke('move_explorer_backward', { label: appWindow.label, key: tabKey });
  };

  const moveFirst = () => {
    setIsLoading(true);
    invoke('move_explorer_to_start', { label: appWindow.label, key: tabKey });
  };

  const moveLast = () => {
    setIsLoading(true);
    invoke('move_explorer_to_end', { label: appWindow.label, key: tabKey });
  };

  // ソート・検索
  const handleSortChange = (index: number) => {
    const option = sortOptions[index];
    if (!option) return;
    setIsLoading(true);
    setSortConfig(option.config);
    invoke('change_explorer_sort', {
      label: appWindow.label,
      key: tabKey,
      sort: option.config,
    });
  };

  const handleSearchInput = (value: string) => {
    setSearchInput(value);
    debouncedSearch(value);
  };

  // キーボード・マウスナビゲーション
  const handleOnKeyDown = (event: KeyboardEvent) => {
    if (!isActiveTab()) return;
    if (event.target instanceof HTMLInputElement) return;
    if (event.key === 'ArrowLeft') {
      event.preventDefault();
      moveBackward();
    } else if (event.key === 'ArrowRight') {
      event.preventDefault();
      moveForward();
    }
  };

  const handleOnButtonDown = (event: MouseEvent) => {
    if (!isActiveTab()) return;
    event.preventDefault();
    if (event.button === 3) moveBackward();
    else if (event.button === 4) moveForward();
  };

  document.addEventListener('keydown', handleOnKeyDown, false);
  document.addEventListener('mouseup', handleOnButtonDown, false);

  onCleanup(() => {
    document.removeEventListener('keydown', handleOnKeyDown, false);
    document.removeEventListener('mouseup', handleOnButtonDown, false);
    debouncedSearch.clear();
    unListenRef?.();
    activeViewerDirListenRef?.();
  });

  return {
    // 状態
    transferPath,
    folders,
    pagination,
    isLoading,
    activeViewerDir,
    sortConfig,
    searchInput,
    // アクション
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
  };
};
