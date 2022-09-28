import { Component, For } from 'solid-js';
import { match } from 'ts-pattern';
import { DirectoryTree } from '../types/DirectoryTree';
import { DirectoryNode } from './DirectoryNode';
import { FileNode } from './FileNode';
import { ZipNode } from './ZipNode';

type Props = {
  selected?: DirectoryTree;
  tree: DirectoryTree[];
  onClick: (path: string) => void;
};

export const DirectoryList: Component<Props> = (props) => {
  return (
    <div class="flex flex-col">
      <For each={props.tree}>
        {(node) =>
          match(node)
            .with({ type: 'Directory' }, (nd) => (
              <DirectoryNode
                tree={nd}
                selected={props.selected}
                onClick={props.onClick}
              />
            ))
            .with({ type: 'File' }, (nd) => (
              <FileNode
                node={nd}
                isSelected={nd.path === props.selected?.path}
                onClick={props.onClick}
              />
            ))
            .with({ type: 'Zip' }, (nd) => (
              <ZipNode
                node={nd}
                isSelected={
                  nd.path + nd.name ===
                  (props.selected?.path ?? '') + (props.selected?.name ?? '')
                }
                onClick={props.onClick}
              />
            ))
            .exhaustive()
        }
      </For>
    </div>
  );
};
