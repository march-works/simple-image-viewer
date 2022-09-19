import { FileOutlined } from '@ant-design/icons';
import { FC, useEffect, useRef } from 'react';
import { DirectoryTree, File } from '../types/DirectoryTree';
import { NodeBaseStyle } from './NodeBaseStyle';

type Props = {
  node: File;
  selected?: DirectoryTree;
  onClick?: (path: string) => void;
};

export const FileNode: FC<Props> = ({ node, selected, onClick }) => {
  const nodeRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    node.path === selected?.path &&
      nodeRef.current &&
      nodeRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'center',
        inline: 'center',
      });
  }, [selected]);
  return (
    <NodeBaseStyle
      className="pl-4"
      ref={nodeRef}
      isSelected={node.path === selected?.path}
      onClick={() => onClick && onClick(node.path)}
    >
      <FileOutlined />
      <div className="hidden lg:block">{node.name}</div>
    </NodeBaseStyle>
  );
};
