import { For, Show, createEffect, on } from 'solid-js';
import type { Component } from 'solid-js';
import { Folder } from '../../../features/Folder/routes/Folder';
import type { Thumbnail } from '../../../features/Folder/types/Thumbnail';
import { Loading } from '../../../components/Loading/Loading';
import { normalizePathForComparison } from '../../../utils/path';

type Props = {
  folders: Thumbnail[];
  isLoading: boolean;
  transferPath: string | undefined;
  activeViewerDir: string | undefined;
  onFolderClick: (thumb: Thumbnail) => void;
  onMarkedAsRead: (path: string) => void;
};

export const FolderGrid: Component<Props> = (props) => {
  let divRef: HTMLDivElement | undefined;

  // フォルダリストが変更されたらスクロールをリセット
  createEffect(
    on(
      () => props.folders,
      () => {
        if (divRef) {
          divRef.scrollTop = 0;
        }
      },
    ),
  );

  return (
    <Show when={!props.isLoading} fallback={<Loading size="lg" />}>
      <div
        ref={divRef}
        class="relative flex flex-row flex-wrap p-5 gap-5 overflow-auto"
      >
        <For each={props.folders}>
          {(item) => (
            <Folder
              thumb={item}
              showMarkAsRead={!!props.transferPath}
              isHighlighted={
                props.activeViewerDir !== undefined &&
                normalizePathForComparison(item.path) === props.activeViewerDir
              }
              onMarkedAsRead={props.onMarkedAsRead}
              onClick={props.onFolderClick}
            />
          )}
        </For>
      </div>
    </Show>
  );
};
