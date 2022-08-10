import { DataNode } from 'antd/lib/tree';
import DirectoryList from 'antd/lib/tree/DirectoryTree';
import { FC } from 'react';
import { DirectoryTree } from '../types/DirectoryTree';

type Props = {
  tree: DirectoryTree[];
  selected: string;
  onSelectedChanged: (entries: string) => void;
};

export const PathSelection: FC<Props> = ({
  tree,
  selected,
  onSelectedChanged,
}) => {
  const convertTreeToNode = (tree: DirectoryTree): DataNode => {
    return {
      title: tree.name,
      key: tree.path + (tree.type === 'Directory' ? 'dir' : ''),
      isLeaf: tree.type === 'File',
      children:
        tree.type === 'Directory'
          ? tree.children.map(convertTreeToNode)
          : undefined,
    };
  };

  const directoryData: DataNode[] = tree.map(convertTreeToNode);

  return (
    <div className="flex flex-col flex-1 space-y-2">
      <div className="overflow-y-auto">
        <DirectoryList
          className="bg-neutral-200"
          defaultExpandAll
          onSelect={(key) => {
            const path = key[0] as string;
            path.endsWith('dir') || onSelectedChanged(key[0] as string);
          }}
          onExpand={() => {
            // do nothing
          }}
          treeData={directoryData}
          selectedKeys={[selected]}
          onKeyDown={() => true}
        />
      </div>
    </div>
  );
};
