import { Component } from 'solid-js';
import { DirectoryList } from '../components/DirectoryList';
import { DirectoryTree, File } from '../types/DirectoryTree';

type Props = {
  tree: DirectoryTree[];
  selected?: File;
  onSelectedChanged: (entries: string) => void;
};

export const PathSelection: Component<Props> = (props) => {
  return (
    <div class="flex max-w-max flex-1 flex-col space-y-2 lg:max-w-xs">
      <div class="overflow-y-auto">
        <DirectoryList
          selected={props.selected}
          tree={props.tree}
          onClick={(path) => {
            path.endsWith('dir') || props.onSelectedChanged(path);
          }}
        />
      </div>
    </div>
  );
};
