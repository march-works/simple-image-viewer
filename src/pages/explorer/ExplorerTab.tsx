import {
  For,
  Show,
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount,
} from 'solid-js';
import type { Component } from 'solid-js';
import { Pagination } from '../../components/Pagination/Pagination';
import { Folder } from '../../features/Folder/routes/Folder';
import type { Thumbnail } from '../../features/Folder/types/Thumbnail';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { FaSolidFolderOpen } from 'solid-icons/fa';
import { RiDocumentFolderTransferFill } from 'solid-icons/ri';
import { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
const appWindow = getCurrentWebviewWindow();

export type TabState = {
  title: string;
  key: string;
  path?: string;
  transfer_path?: string;
  page: number;
  end: number;
  folders: Thumbnail[];
};

type Props = {
  tabKey: string;
  path?: string;
  transferPath?: string;
  isActiveTab: boolean;
};

export const ExplorerTab: Component<Props> = (props) => {
  const [transferPath, setTransferPath] = createSignal<string>();
  const [folders, setFolders] = createSignal<Thumbnail[]>([]);
  const [pagination, setPagination] = createSignal<[number, number]>([1, 1]);
  const [isLoading, setIsLoading] = createSignal<boolean>(false);
  const [activeViewerDir, setActiveViewerDir] = createSignal<
    string | undefined
  >();
  let unListenRef: UnlistenFn | undefined = undefined;
  let activeViewerDirListenRef: UnlistenFn | undefined = undefined;
  let divRef!: HTMLDivElement;

  // アクティブなViewerのアクティブなタブのディレクトリを取得
  const updateActiveViewerDirectory = async () => {
    try {
      const dir = await invoke<string | null>('get_active_viewer_directory');
      setActiveViewerDir(dir ?? undefined);
    } catch {
      // エラーは無視（Viewerが開いていない場合など）
      setActiveViewerDir(undefined);
    }
  };

  // onMountで非同期リスナー登録を確実に待つ
  onMount(async () => {
    // タブ状態変更イベントリスナー
    unListenRef = await appWindow.listen(
      'explorer-tab-state-changed',
      (event) => {
        const {
          key,
          transfer_path: transferPath,
          page,
          end,
          folders,
        } = event.payload as TabState;
        if (key !== props.tabKey) return;
        setPagination([page, end]);
        setTransferPath(transferPath);
        setFolders(folders);
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
      key: props.tabKey,
    });

    // 初回のアクティブディレクトリを取得
    updateActiveViewerDirectory();
  });

  createEffect(
    on(folders, () => {
      if (!divRef) return;
      divRef.scrollTop = 0;
    }),
  );

  const selectTransferPath = async () => {
    const dir = await open({
      directory: true,
    });
    if (Array.isArray(dir)) {
      return;
    }
    if (!dir) {
      return;
    }
    await invoke('change_explorer_transfer_path', {
      transferPath: dir,
      key: props.tabKey,
      label: appWindow.label,
    });
  };

  const onClick = (thumb: Thumbnail) => {
    if (thumb.thumbpath) {
      invoke('open_new_viewer_tab', {
        path: thumb.thumbpath,
      });
    } else {
      invoke('change_explorer_path', {
        path: thumb.path,
        label: appWindow.label,
        key: props.tabKey,
      });
    }
  };

  const transfer = async (from: string, to: string) => {
    await invoke('transfer_folder', {
      from,
      to,
      label: appWindow.label,
    });
  };

  const closeViewerTabsForDirectory = async (directory: string) => {
    try {
      await invoke('close_viewer_tabs_by_directory', { directory });
    } catch (error) {
      console.error('Failed to close viewer tabs:', error);
    }
  };

  const normalizePathForComparison = (path: string): string => {
    return path.replace(/\\/g, '/').toLowerCase();
  };

  const resetTab = () => {
    invoke('reset_explorer_tab', { label: appWindow.label, key: props.tabKey });
  };

  const movePage = (page: number) => {
    setIsLoading(true);
    invoke('change_explorer_page', {
      label: appWindow.label,
      key: props.tabKey,
      page,
    });
  };

  const moveForward = () => {
    setIsLoading(true);
    invoke('move_explorer_forward', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveBackward = () => {
    setIsLoading(true);
    invoke('move_explorer_backward', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveFirst = () => {
    setIsLoading(true);
    invoke('move_explorer_to_start', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveLast = () => {
    setIsLoading(true);
    invoke('move_explorer_to_end', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const handleOnKeyDown = (event: KeyboardEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.key === 'ArrowLeft') moveBackward();
    else if (event.key === 'ArrowRight') moveForward();
  };

  const handleOnButtonDown = (event: MouseEvent) => {
    if (!props.isActiveTab) return;
    event.preventDefault();
    if (event.button === 3) moveBackward();
    else if (event.button === 4) moveForward();
  };

  document.addEventListener('keydown', handleOnKeyDown, false);
  document.addEventListener('mouseup', handleOnButtonDown, false);

  onCleanup(() => {
    document.removeEventListener('keydown', handleOnKeyDown, false);
    document.removeEventListener('mouseup', handleOnButtonDown, false);
    // Tauriイベントリスナーを解除
    unListenRef?.();
    activeViewerDirListenRef?.();
  });

  return (
    <div class="h-full flex flex-col overflow-hidden">
      <div class="p-1 h-12 flex flex-row gap-2">
        <div
          class="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={() => resetTab()}
        >
          <FaSolidFolderOpen class="ml-0.5 h-5 w-5" />
        </div>
        <div
          class="mr-1 p-2 flex flex-row h-8 shrink-0 items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={() => selectTransferPath()}
        >
          <RiDocumentFolderTransferFill class="ml-0.5 h-5 w-5" />
          <span class="text-xs">
            {transferPath() ? '転送先を変更する' : '転送先を設定する'}
          </span>
        </div>
      </div>
      <Show
        when={!isLoading()}
        fallback={
          <div class="flex flex-1 items-center justify-center">
            <div class="animate-spin rounded-full h-32 w-32 border-b-2 border-neutral-500" />
          </div>
        }
      >
        <div
          ref={divRef}
          class="relative flex flex-row flex-wrap p-5 gap-5 overflow-auto"
        >
          <For each={folders()}>
            {(item) => (
              <Folder
                thumb={item}
                showMarkAsRead={!!transferPath()}
                isHighlighted={
                  activeViewerDir() !== undefined &&
                  normalizePathForComparison(item.path) === activeViewerDir()
                }
                onMarkedAsRead={(path) => {
                  const to = transferPath();
                  if (!to) {
                    return;
                  }
                  transfer(path, to);
                  // 転送後にViewerのタブを閉じる
                  closeViewerTabsForDirectory(path);
                }}
                onClick={onClick}
              />
            )}
          </For>
        </div>
      </Show>
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
