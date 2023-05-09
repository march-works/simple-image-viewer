import { invoke } from "@tauri-apps/api";
import { FaSolidFolderOpen, FaSolidRectangleList } from "solid-icons/fa";
import { createEffect, createSignal, For, onMount } from "solid-js";
import { Store } from "tauri-plugin-store-api";
import { Pagination } from "../../components/Pagination/Pagination";
import { Folder } from "../../features/Folder/routes/Folder";
import { Thumbnail } from "../../features/Folder/types/Thumbnail";

const Explorer = () => {
  const [root, setRoot] = createSignal<string>();
  const [folders, setFolders] = createSignal<Thumbnail[]>([]);
  const [page, setPage] = createSignal<number>(1);
  const [end, setEnd] = createSignal<number>(1);
  const store = new Store('.settings.dat');
  const storeKey = 'explorer-root';

  onMount(async () => {
    root() && invoke<number>('get_page_count', {
      filepath: root(),
    }).then((v) => setEnd(v));

    const path = await store.get<string>(storeKey);
    path && setRoot(path);
  });

  createEffect(() => {
    store.set(storeKey, root()).then(() => {
      store.save();
    });
  });

  createEffect(() => {
    root() && invoke<Thumbnail[]>('explore_path', {
      filepath: root(),
      page: page(),
    }).then((thumbs) => {
      setFolders(thumbs)
    });
    root() && invoke<number>('get_page_count', {
      filepath: root(),
    }).then((v) => setEnd(v));
});

  const onClick = (thumb: Thumbnail) => {
    if (thumb.thumbnail) {
      invoke('add_tab', {
        filepath: thumb.thumbpath,
      });
    } else {
      setFolders([]);
      setRoot(thumb.path);
    }
  };

  const showDevices = () => {
    setFolders([]);
    setRoot(undefined);
    invoke<Thumbnail[]>('show_devices').then((thumbs) => setFolders(thumbs));
  };

  return (
    <div class="App flex h-screen w-screen select-none bg-neutral-900 text-neutral-100 overflow-hidden">
      <div class="relative flex w-full flex-1 flex-col">
        <div class="flex h-8 w-full flex-none flex-row bg-neutral-800 align-baseline">
        <div
          class="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={showDevices}
        >
          <FaSolidFolderOpen class="ml-0.5 h-5 w-5" />
        </div>
        <div
          class="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={undefined}
        >
          <FaSolidRectangleList class="h-5 w-5" />
        </div>
        </div>
        <div class="relative flex flex-row flex-wrap p-5 gap-5 overflow-auto" style={{ height: 'calc(100% - 2rem)' }}>
          <For each={folders()}>
            {(item) => <Folder thumb={item} onClick={onClick} />}
          </For>
        </div>
        <div class="p-1 h-12 self-center">
          <Pagination
            current={page()}
            end={end()}
            onClickPrev={() => {
              setPage((prev) => Math.max(prev - 1, 1));
            }}
            onClickNext={() => {
              setPage((prev) => Math.min(prev + 1, end()));
            }}
            onClickPage={(page) => {
              setPage(page);
            }}
            onClickStart={() => {
              setPage(1);
            }}
            onClickEnd={() => {
              setPage(end);
            }}
          />
        </div>
      </div>
    </div>
  );
};

export default Explorer;
