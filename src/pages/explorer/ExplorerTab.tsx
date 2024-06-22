import { Component, For, createSignal, onCleanup, onMount } from 'solid-js';
import { Pagination } from '../../components/Pagination/Pagination';
import { Folder } from '../../features/Folder/routes/Folder';
import { Thumbnail } from '../../features/Folder/types/Thumbnail';
import { invoke } from '@tauri-apps/api';
import { open } from '@tauri-apps/api/dialog';
import { FaSolidFolderOpen } from 'solid-icons/fa';
import { RiDocumentFolderTransferFill } from 'solid-icons/ri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { appWindow } from '@tauri-apps/api/window';

export type TabState = {
  title: string;
  key: string;
  path?: string;
  transferPath?: string;
  page: number;
  end: number;
  folders: Thumbnail[];
};

type Props = {
  tabKey: string;
  path?: string;
  transferPath?: string;
};

export const ExplorerTab: Component<Props> = (props) => {
  const [transferPath, setTransferPath] = createSignal<string>();
  const [folders, setFolders] = createSignal<Thumbnail[]>([]);
  const [page, setPage] = createSignal<number>(1);
  const [end, setEnd] = createSignal<number>(1);
  let unListenRef: UnlistenFn | undefined = undefined;

  onMount(async () => {
    listen('explorer-tab-state-changed', (event) => {
      const { key, transferPath, page, end, folders } =
        event.payload as TabState;
      if (key !== props.tabKey) return;
      console.log(event.payload);
      setPage(page);
      setEnd(end);
      setTransferPath(transferPath);
      setFolders(folders);
    }).then((unListen) => (unListenRef = unListen));

    invoke('request_restore_explorer_tab_state', {
      label: appWindow.label,
      key: props.tabKey,
    });
  });

  onCleanup(() => {
    unListenRef && unListenRef();
  });

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
    setTransferPath(dir);
  };

  const onClick = (thumb: Thumbnail) => {
    if (thumb.thumbnail) {
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
    });
  };

  const resetTab = () => {
    invoke('reset_explorer_tab', { label: appWindow.label, key: props.tabKey });
  };

  const movePage = (page: number) => {
    invoke('change_explorer_page', {
      label: appWindow.label,
      key: props.tabKey,
      page,
    });
  };

  const moveForward = () => {
    invoke('move_explorer_forward', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveBackward = () => {
    invoke('move_explorer_backward', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveFirst = () => {
    invoke('move_explorer_to_start', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

  const moveLast = () => {
    invoke('move_explorer_to_end', {
      label: appWindow.label,
      key: props.tabKey,
    });
  };

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
      <div class="relative flex flex-row flex-wrap p-5 gap-5 overflow-auto">
        <For each={folders()}>
          {(item) => (
            <Folder
              thumb={item}
              onMarkedAsRead={(path) => {
                const to = transferPath();
                if (!to) {
                  return;
                }
                transfer(path, to);
              }}
              onClick={onClick}
            />
          )}
        </For>
      </div>
      <div class="p-1 h-12 self-center">
        <Pagination
          current={page()}
          end={end()}
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
