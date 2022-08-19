import { DataNode } from 'antd/lib/tree';
import DirectoryList from 'antd/lib/tree/DirectoryTree';
import { FC } from 'react';
import { match } from 'ts-pattern';
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
  const selectedKeys = match(selected)
    .with({ type: 'File' }, (file) => file.path)
    .with({ type: 'Zip' }, (file) => file.path + file.name)
    .otherwise(() => '');

  const convertTreeToNode = (tree: DirectoryTree): DataNode => {
    return {
      title: tree.name,
      key:
        tree.path +
        (tree.type === 'Directory'
          ? 'dir'
          : tree.type === 'Zip'
          ? tree.name
          : ''),
      isLeaf: tree.type === 'File' || tree.type === 'Zip',
      children:
        tree.type === 'Directory'
          ? tree.children.map(convertTreeToNode)
          : undefined,
    };
  };

  const directoryData: DataNode[] = tree.map(convertTreeToNode);

  return (
    <div className="flex flex-col flex-1 space-y-2 max-w-0 md:max-w-xs">
      <div className="overflow-y-auto">
        <DirectoryList
          className="bg-neutral-200 truncate"
          defaultExpandAll
          onSelect={(key) => {
            const path = key[0] as string;
            path.endsWith('dir') || onSelectedChanged(key[0] as string);
          }}
          onExpand={() => {
            // do nothing
          }}
          treeData={directoryData}
          selectedKeys={[selectedKeys]}
          onKeyDown={() => true}
        />
      </div>
    </div>
  );
};
