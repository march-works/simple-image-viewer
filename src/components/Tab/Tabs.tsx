import { For, JSX } from 'solid-js';
import {
  FaSolidXmark,
  FaSolidFolderOpen,
  FaSolidRectangleList,
} from 'solid-icons/fa';

type TabInfo<T> = T & {
  key: string;
  title: string;
};

type Props<T> = {
  viewing?: string;
  tabs: TabInfo<T>[];
  intoContent: (info: TabInfo<T>) => JSX.Element;
  handleOnClick: (key: string) => void;
  handleOnClose: (key: string) => void;
  handleOnAdd: () => void;
  handleOnOpenExplorer: () => void;
};

export const Tabs = <T,>(props: Props<T>) => {
  return (
    <div class="relative flex w-full flex-1 flex-col">
      <div class="flex h-8 w-full flex-none flex-row bg-neutral-800 align-baseline">
        <div
          class="mx-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={() => props.handleOnOpenExplorer()}
        >
          <FaSolidRectangleList class="h-5 w-5" />
        </div>
        <For each={props.tabs}>
          {(tab) => (
            <div
              class={
                'flex w-48 min-w-0 flex-row justify-between items-end rounded-t-md border-2 border-b-0 border-neutral-500 p-1 transition-colors' +
                (tab.key === props.viewing
                  ? ' bg-gradient-to-b from-neutral-500 to-neutral-900 text-neutral-100'
                  : ' bg-neutral-900 text-neutral-400 hover:bg-gradient-to-b hover:from-neutral-600 hover:to-neutral-900 hover:text-neutral-300')
              }
              onMouseDown={(e) =>
                e.button === 1 && props.handleOnClose(tab.key)
              }
            >
              <div
                class="flex-1 self-center truncate"
                onClick={() => props.handleOnClick(tab.key)}
              >
                {tab.title}
              </div>
              <div
                class="flex pt-0.5 h-5 w-5 justify-center rounded-full text-neutral-100 transition-colors hover:bg-neutral-500"
                onClick={() => props.handleOnClose(tab.key)}
              >
                <FaSolidXmark />
              </div>
            </div>
          )}
        </For>
        <div
          class="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300"
          onClick={() => props.handleOnAdd()}
        >
          <FaSolidFolderOpen class="ml-0.5 h-5 w-5" />
        </div>
      </div>
      <div class="relative" style={{ height: 'calc(100% - 2rem)' }}>
        <For each={props.tabs}>
          {(tab) => (
            <div
              class={`w-full h-full${
                tab?.key === props.viewing ? '' : ' hidden'
              }`}
            >
              {props.intoContent(tab)}
            </div>
          )}
        </For>
      </div>
    </div>
  );
};
