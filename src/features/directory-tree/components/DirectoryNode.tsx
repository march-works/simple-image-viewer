import {
  CaretDownOutlined,
  CaretRightOutlined,
  FolderOpenOutlined,
  FolderOutlined,
} from '@ant-design/icons';
import { FC, useState } from 'react';
import { match } from 'ts-pattern';
import { Directory, DirectoryTree } from '../types/DirectoryTree';
import { FileNode } from './FileNode';
import { ZipNode } from './ZipNode';

type Props = {
  tree: Directory;
  selected?: DirectoryTree;
  onClick?: (path: string) => void;
};

export const DirectoryNode: FC<Props> = ({ tree, selected, onClick }) => {
  const [open, setOpen] = useState<boolean>(false);
  return (
    <div className="w-full">
      <div
        className="text-base truncate flex flex-row items-baseline gap-1 w-full cursor-pointer text-neutral-100 hover:bg-neutral-600"
        onClick={() => {
          onClick && onClick(tree.path);
          setOpen((prev) => !prev);
        }}
      >
        {open ? (
          <CaretDownOutlined className="block text-xs" />
        ) : (
          <CaretRightOutlined className="block text-xs" />
        )}
        {open ? <FolderOpenOutlined /> : <FolderOutlined />}
        <div className="truncate">{tree.name}</div>
      </div>
      {tree.children.length > 0 && open && (
        <div className="ml-6">
          {tree.children.map((node) =>
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
      )}
    </div>
  );
};
