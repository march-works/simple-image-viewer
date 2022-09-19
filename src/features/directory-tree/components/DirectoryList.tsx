import { FC } from 'react';
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

export const DirectoryList: FC<Props> = ({ selected, tree, onClick }) => {
  return (
    <div className="flex flex-col">
      {tree.map((node) =>
        match(node)
          .with({ type: 'Directory' }, (nd) => (
            <DirectoryNode
              key={nd.path}
              tree={nd}
              selected={selected}
              onClick={onClick}
            />
          ))
          .with({ type: 'File' }, (nd) => (
            <FileNode
              key={nd.path}
              node={nd}
              selected={selected}
              onClick={onClick}
            />
          ))
          .with({ type: 'Zip' }, (nd) => (
            <ZipNode
              key={nd.path + nd.name}
              node={nd}
              selected={selected}
              onClick={onClick}
            />
          ))
          .exhaustive()
      )}
    </div>
  );
};
