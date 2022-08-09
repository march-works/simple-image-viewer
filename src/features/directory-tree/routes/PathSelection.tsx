import { FolderOpenOutlined } from '@ant-design/icons';
import { open } from '@tauri-apps/api/dialog';
import { FileEntry, readDir } from '@tauri-apps/api/fs';
import { DataNode } from 'antd/lib/tree';
import DirectoryList from 'antd/lib/tree/DirectoryTree';
import { FC } from 'react';
import { DirectoryTree } from '../types/DirectoryTree';

type Props = {
  tree: DirectoryTree[];
  selected: string;
  setTree: (tree: DirectoryTree[]) => void;
  onSelectedChanged: (entries: string) => void;
};

export const PathSelection: FC<Props> = ({
  tree,
  selected,
  setTree,
  onSelectedChanged,
}) => {
  const convertEntryToTree = (entry: FileEntry): DirectoryTree => {
    if (entry.children === null || entry.children === undefined) {
      return {
        type: 'File',
        name: entry.name ?? '',
        path: entry.path,
      };
    }
    return {
      type: 'Directory',
      name: entry.name ?? '',
      path: entry.path,
      children: entry.children.map(convertEntryToTree),
    };
  };

  const handleOnClick = async () => {
    const dir = await open({
      directory: true,
    });
    if (Array.isArray(dir)) {
      return;
    }
    if (!dir) {
      return;
    }
    const entries = await readDir(dir, {
      recursive: true,
    });
    setTree(entries.map(convertEntryToTree));
  };

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
      <div className="flex flex-col">
        <FolderOpenOutlined
          className="text-xl block self-end p-3"
          onClick={handleOnClick}
        />
      </div>
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
