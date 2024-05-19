import { Component, createMemo, createSignal, For, Show } from 'solid-js';
import { match, P } from 'ts-pattern';
import { ImageNode } from './ImageNode';
import { NodeBaseStyle } from './NodeBaseStyle';
import { ZipNode } from './ZipNode';
import {
  FaSolidCaretRight,
  FaSolidCaretDown,
  FaSolidFolderOpen,
  FaSolidFolder,
} from 'solid-icons/fa';
import { VideoNode } from './VideoNode';
import { Directory, File } from '../../../pages/viewer/ViewerTab';
import equal from 'fast-deep-equal';

type Props = {
  tree: Directory;
  viewing?: File;
  onClick: (path: File) => void;
};

export const DirectoryNode: Component<Props> = (props) => {
  const [open, setOpen] = createSignal<boolean>(false);
  const tree = createMemo(() => props.tree, undefined, {
    equals: equal,
  });
  return (
    <div class="w-full">
      <NodeBaseStyle onClick={() => setOpen((prev) => !prev)}>
        <Show when={open()} fallback={<FaSolidCaretRight />}>
          <FaSolidCaretDown />
        </Show>
        <Show when={open()} fallback={<FaSolidFolder />}>
          <FaSolidFolderOpen />
        </Show>
        <div class="hidden truncate lg:block">{props.tree.name}</div>
      </NodeBaseStyle>
      <Show when={props.tree.children.length > 0 && open()}>
        <div class="lg:ml-6">
          <For each={tree().children}>
            {(node) =>
              match(node)
                .with({ Directory: P.any }, (nd) => (
                  <DirectoryNode
                    tree={nd.Directory}
                    viewing={props.viewing}
                    onClick={props.onClick}
                  />
                ))
                .with({ File: { file_type: 'Image' } }, (nd) => (
                  <ImageNode
                    node={nd.File}
                    isSelected={nd.File.key === props.viewing?.key}
                    onClick={() => props.onClick(nd.File)}
                  />
                ))
                .with({ File: { file_type: 'Video' } }, (nd) => (
                  <VideoNode
                    node={nd.File}
                    isSelected={nd.File.key === props.viewing?.key}
                    onClick={() => props.onClick(nd.File)}
                  />
                ))
                .with({ File: { file_type: 'Zip' } }, (nd) => (
                  <ZipNode
                    node={nd.File}
                    isSelected={nd.File.key === props.viewing?.key}
                    onClick={() => props.onClick(nd.File)}
                  />
                ))
                .with({ File: P.select() }, () => undefined)
                .exhaustive()
            }
          </For>
        </div>
      </Show>
    </div>
  );
};
