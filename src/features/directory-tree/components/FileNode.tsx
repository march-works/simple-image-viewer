import { faImage } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
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
      className="pl-3"
      ref={nodeRef}
      isSelected={node.path === selected?.path}
      onClick={() => onClick && onClick(node.path)}
    >
      <FontAwesomeIcon icon={faImage} />
      <div className="hidden lg:block">{node.name}</div>
    </NodeBaseStyle>
  );
};
