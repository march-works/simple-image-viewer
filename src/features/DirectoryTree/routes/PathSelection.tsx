import { Component } from 'solid-js';
import { DirectoryList } from '../components/DirectoryList';
import { FileTree, File } from '../../../pages/viewer/ViewerTab';

type Props = {
  tree: FileTree[];
  viewing?: File;
  onSelectedChanged: (file: File) => void;
};

export const PathSelection: Component<Props> = (props) => {
  return (
    <div class="flex max-w-max flex-1 flex-col space-y-2 lg:max-w-xs">
      <div class="overflow-y-auto">
        <DirectoryList
          viewing={props.viewing}
          tree={props.tree}
          onClick={(path) => props.onSelectedChanged(path)}
        />
      </div>
    </div>
  );
};
