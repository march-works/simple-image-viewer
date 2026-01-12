import { For, Show } from 'solid-js';
import type { Component } from 'solid-js';
import { FaSolidFolderOpen } from 'solid-icons/fa';
import { RiDocumentFolderTransferFill } from 'solid-icons/ri';
import { FaSolidWandMagicSparkles } from 'solid-icons/fa';
import {
  SortConfig,
  getSortOptionIndex,
  sortOptions,
} from '../../../features/Explorer/types/ExplorerQuery';

type Props = {
  transferPath: string | undefined;
  sortConfig: SortConfig;
  searchInput: string;
  isRebuildingRecommendations: boolean;
  onResetTab: () => void;
  onSelectTransferPath: () => void;
  onSortChange: (index: number) => void;
  onSearchInput: (value: string) => void;
  onRebuildRecommendations: () => void;
};

export const ExplorerToolbar: Component<Props> = (props) => {
  return (
    <div class="p-1 h-12 flex flex-row gap-2 items-center">
      <div
        class="ml-1 flex h-8 w-8 shrink-0 flex-col items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300 cursor-pointer"
        onClick={props.onResetTab}
      >
        <FaSolidFolderOpen class="ml-0.5 h-5 w-5" />
      </div>
      <div
        class="p-2 flex flex-row h-8 shrink-0 items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300 cursor-pointer"
        onClick={props.onSelectTransferPath}
      >
        <RiDocumentFolderTransferFill class="ml-0.5 h-5 w-5" />
        <span class="text-xs">
          {props.transferPath ? '転送先を変更する' : '転送先を設定する'}
        </span>
      </div>
      <div
        class={`p-2 flex flex-row h-8 shrink-0 items-center justify-center rounded-full border-2 border-neutral-500 bg-neutral-900 transition-colors ${
          props.isRebuildingRecommendations
            ? 'text-yellow-400 cursor-not-allowed'
            : 'text-neutral-400 hover:bg-neutral-700 hover:text-neutral-300 cursor-pointer'
        }`}
        onClick={() =>
          !props.isRebuildingRecommendations && props.onRebuildRecommendations()
        }
        title="おすすめを再構築"
      >
        <FaSolidWandMagicSparkles
          class={`h-4 w-4 ${props.isRebuildingRecommendations ? 'animate-pulse' : ''}`}
        />
        <Show when={props.isRebuildingRecommendations}>
          <span class="ml-1 text-xs">処理中...</span>
        </Show>
      </div>
      <div class="flex-1" />
      <input
        type="text"
        placeholder="検索..."
        value={props.searchInput}
        onInput={(e) => props.onSearchInput(e.currentTarget.value)}
        class="h-8 px-3 w-48 rounded-lg border-2 border-neutral-500 bg-neutral-900 text-neutral-300 text-sm placeholder-neutral-500 focus:outline-none focus:border-neutral-400"
      />
      <select
        value={getSortOptionIndex(props.sortConfig)}
        onChange={(e) => props.onSortChange(parseInt(e.currentTarget.value))}
        class="h-8 px-2 mr-1 rounded-lg border-2 border-neutral-500 bg-neutral-900 text-neutral-300 text-sm focus:outline-none focus:border-neutral-400"
      >
        <For each={sortOptions}>
          {(option, index) => <option value={index()}>{option.label}</option>}
        </For>
      </select>
    </div>
  );
};
