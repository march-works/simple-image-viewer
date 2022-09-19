import { FC } from 'react';
import { DirectoryList } from '../components/DirectoryList';
import { DirectoryTree, Zip, File } from '../types/DirectoryTree';

type Props = {
  tree: DirectoryTree[];
  selected?: File | Zip;
  onSelectedChanged: (entries: string) => void;
};

export const PathSelection: FC<Props> = ({
  tree,
  selected,
  onSelectedChanged,
}) => {
  return (
    <div className="flex max-w-0 flex-1 flex-col space-y-2 md:max-w-xs">
      <div className="overflow-y-auto">
        <DirectoryList
          selected={selected}
          tree={tree}
          onClick={(path) => {
            path.endsWith('dir') || onSelectedChanged(path);
          }}
        />
      </div>
    </div>
  );
};
