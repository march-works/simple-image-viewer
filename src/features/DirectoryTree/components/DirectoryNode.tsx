import { Component, createSignal, For, Show } from 'solid-js';
import { match } from 'ts-pattern';
import { Directory, DirectoryTree } from '../types/DirectoryTree';
import { FileNode } from './FileNode';
import { NodeBaseStyle } from './NodeBaseStyle';
import { ZipNode } from './ZipNode';

type Props = {
  tree: Directory;
  selected?: DirectoryTree;
  onClick?: (path: string) => void;
};

export const DirectoryNode: Component<Props> = (props) => {
  const [open, setOpen] = createSignal<boolean>(false);
  return (
    <div class="w-full">
      <NodeBaseStyle
        onClick={() => {
          props.onClick && props.onClick(props.tree.path);
          setOpen((prev) => !prev);
        }}
      >
        <Show when={open()} fallback={<i class="fa-solid fa-caret-right" />}>
          <i class="fa-solid fa-caret-down" />
        </Show>
        <Show when={open()} fallback={<i class="fa-solid fa-folder" />}>
          <i class="fa-solid fa-folder-open" />
        </Show>
        <div class="hidden truncate lg:block">{props.tree.name}</div>
      </NodeBaseStyle>
      <Show when={props.tree.children.length > 0 && open()}>
        <div class="lg:ml-6">
          <For each={props.tree.children}>
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
                    selected={props.selected}
                    onClick={props.onClick}
                  />
                ))
                .with({ type: 'Zip' }, (nd) => (
                  <ZipNode
                    node={nd}
                    selected={props.selected}
                    onClick={props.onClick}
                  />
                ))
                .exhaustive()
            }
          </For>
        </div>
      </Show>
    </div>
  );
};
