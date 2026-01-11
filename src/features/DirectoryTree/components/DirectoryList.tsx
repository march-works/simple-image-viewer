import { For, createMemo } from 'solid-js';
import type { Component } from 'solid-js';
import { P, match } from 'ts-pattern';
import { DirectoryNode } from './DirectoryNode';
import { ImageNode } from './ImageNode';
import { VideoNode } from './VideoNode';
import { ZipNode } from './ZipNode';
import type { FileTree, File } from '../../../pages/viewer/ViewerTab';
import equal from 'fast-deep-equal';

type Props = {
  viewing?: File;
  tree: FileTree[];
  onClick: (path: File) => void;
};

export const DirectoryList: Component<Props> = (props) => {
  const tree = createMemo(() => props.tree, undefined, {
    equals: equal,
  });
  return (
    <div class="flex flex-col overflow-x-hidden">
      <For each={tree()}>
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
  );
};
