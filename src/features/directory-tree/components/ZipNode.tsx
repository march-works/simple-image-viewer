import { faImage } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { FC, useEffect, useRef } from 'react';
import { DirectoryTree, Zip } from '../types/DirectoryTree';
import { NodeBaseStyle } from './NodeBaseStyle';

type Props = {
  node: Zip;
  selected?: DirectoryTree;
  onClick?: (path: string) => void;
};

export const ZipNode: FC<Props> = ({ node, selected, onClick }) => {
  const nodeRef = useRef<HTMLDivElement | null>(null);

  const isSelected =
    node.path + node.name === (selected?.path ?? '') + (selected?.name ?? '');
  useEffect(() => {
    isSelected &&
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
      isSelected={isSelected}
      onClick={() => onClick && onClick(node.path + node.name)}
    >
      <FontAwesomeIcon icon={faImage} />
      <div className="hidden lg:block">{node.name}</div>
    </NodeBaseStyle>
  );
};
