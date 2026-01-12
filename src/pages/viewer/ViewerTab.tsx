import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { createSignal, createEffect, onCleanup, onMount } from 'solid-js';
import type { Component } from 'solid-js';
import { PathSelection } from '../../features/DirectoryTree/routes/PathSelection';
import { ImageCanvas } from '../../features/Image/ImageCanvas';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
const appWindow = getCurrentWebviewWindow();

export type File = {
  key: string;
  file_type: string;
  path: string;
  name: string;
};

export type Directory = {
  path: string;
  name: string;
  children: FileTree[];
};

export type FileTree =
  | {
      File: File;
    }
  | {
      Directory: Directory;
    };

export type TabState = {
  title: string;
  key: string;
  path: string;
  viewing?: File;
  tree: FileTree[];
};

type Props = {
  isActiveTab: boolean;
  initialPath: string;
  initialTabKey: string;
};

export const ViewerTab: Component<Props> = (props) => {
  const [viewing, setViewing] = createSignal<File | undefined>(undefined);
  const [tree, setTree] = createSignal<FileTree[]>([]);
  let unListenTabStateRef: UnlistenFn | undefined = undefined;
  let unListenDirChangedRef: UnlistenFn | undefined = undefined;

  const moveForward = () => {
    invoke('move_forward', { label: appWindow.label });
  };

  const moveBackward = () => {
    invoke('move_backward', { label: appWindow.label });
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

  // onMountで非同期リスナー登録を確実に待つ
  onMount(async () => {
    // ディレクトリ監視を開始（参照カウント付き）
    await invoke('subscribe_dir_notification', {
      filepath: props.initialPath,
      tabKey: props.initialTabKey,
    });

    // タブ状態変更イベントをリッスン
    unListenTabStateRef = await appWindow.listen(
      'viewer-tab-state-changed',
      (event) => {
        const { key, viewing, tree } = event.payload as TabState;
        if (key !== props.initialTabKey) return;
        setViewing(viewing);
        setTree(tree);
      },
    );

    // ディレクトリ変更イベントをリッスン（グローバルイベント）
    unListenDirChangedRef = await listen<string>(
      'directory-tree-changed',
      (event) => {
        // このタブが監視しているパスと一致する場合のみツリーを更新
        if (event.payload === props.initialPath) {
          invoke('refresh_viewer_tab_tree', {
            tabKey: props.initialTabKey,
            label: appWindow.label,
          });
        }
      },
    );

    invoke('request_restore_viewer_tab_state', {
      label: appWindow.label,
      key: props.initialTabKey,
    });
  });

  document.addEventListener('keydown', handleOnKeyDown, false);
  document.addEventListener('mouseup', handleOnButtonDown, false);

  onCleanup(async () => {
    document.removeEventListener('keydown', handleOnKeyDown, false);
    document.removeEventListener('mouseup', handleOnButtonDown, false);
    // Tauriイベントリスナーを解除
    unListenTabStateRef?.();
    unListenDirChangedRef?.();
    // ディレクトリ監視を解除（参照カウント管理）
    await invoke('unsubscribe_dir_notification', {
      filepath: props.initialPath,
    });
  });

  const changeViewing = (tabKey: string, file: File) => {
    invoke('change_viewing', {
      tabKey: tabKey,
      key: file.key,
      label: appWindow.label,
    });
  };

  // 閲覧履歴を記録する (Phase 2: リコメンド基盤)
  createEffect(() => {
    const currentViewing = viewing();
    if (!currentViewing || !props.isActiveTab) return;

    // フォルダパスとサムネイル画像パスを取得
    const folderPath = props.initialPath;
    // viewing の path がサムネイルとして使用される画像のパス
    const thumbnailImagePath = currentViewing.path;

    // バックエンドに閲覧を記録
    invoke('record_folder_view', {
      folderPath,
      thumbnailImagePath,
    }).catch((e) => console.error('Failed to record folder view:', e));
  });

  return (
    <div class="flex h-full flex-row">
      <ImageCanvas
        viewing={viewing()}
        moveForward={moveForward}
        moveBackward={moveBackward}
      />
      <PathSelection
        viewing={viewing()}
        tree={tree()}
        onSelectedChanged={(file) => changeViewing(props.initialTabKey, file)}
      />
    </div>
  );
};
