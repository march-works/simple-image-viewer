import { invoke } from '@tauri-apps/api';
import { Component, createSignal, onCleanup, onMount } from 'solid-js';
import { PathSelection } from '../../features/DirectoryTree/routes/PathSelection';
import { ImageCanvas } from '../../features/Image/ImageCanvas';
import { appWindow } from '@tauri-apps/api/window';
import { UnlistenFn, listen } from '@tauri-apps/api/event';

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
  path: string;
  tabKey: string;
};

export const ViewerTab: Component<Props> = (props) => {
  // let unListenRef: UnlistenFn | undefined = undefined;
  const [viewing, setViewing] = createSignal<File | undefined>(undefined);
  const [tree, setTree] = createSignal<FileTree[]>([]);
  let unListenRef: UnlistenFn | undefined = undefined;

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

  onMount(() => {
    listen('viewer-tab-state-changed', (event) => {
      const { key, viewing, tree } = event.payload as TabState;
      if (key !== props.tabKey) return;
      setViewing(viewing);
      setTree(tree);
    }).then((unListen) => (unListenRef = unListen));

    invoke('subscribe_dir_notification', { filepath: props.path });
    invoke('request_restore_viewer_tab_state', {
      label: appWindow.label,
      key: props.tabKey,
    });
    // listen('directory-tree-changed', (event) => {
    //   if (event.payload === props.path) readDirAndSetTree();
    // }).then((unListen) => (unListenRef = unListen));

    document.addEventListener('keydown', handleOnKeyDown, false);
    document.addEventListener('mouseup', handleOnButtonDown, false);
  });

  onCleanup(() => {
    // unListenRef && unListenRef();
    document.removeEventListener('keydown', handleOnKeyDown, false);
    document.removeEventListener('mouseup', handleOnButtonDown, false);
    unListenRef && unListenRef();
  });

  const changeViewing = (tabKey: string, file: File) => {
    invoke('change_viewing', {
      tabKey: tabKey,
      key: file.key,
      label: appWindow.label,
    });
  };

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
        onSelectedChanged={(file) => changeViewing(props.tabKey, file)}
      />
    </div>
  );
};
