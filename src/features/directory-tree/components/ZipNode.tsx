import { FileOutlined } from '@ant-design/icons';
import { FC, useEffect, useRef } from 'react';
import { useInView } from 'react-intersection-observer';
import { DirectoryTree, Zip } from '../types/DirectoryTree';

type Props = {
  node: Zip;
  selected?: DirectoryTree;
  onClick?: (path: string) => void;
};

export const ZipNode: FC<Props> = ({ node, selected, onClick }) => {
  const nodeRef = useRef<HTMLDivElement | null>(null);
  const { ref, inView } = useInView({
    threshold: 0.9,
  });

  const isSelected =
    node.path + node.name === (selected?.path ?? '') + (selected?.name ?? '');
  useEffect(() => {
    isSelected &&
      !inView &&
      nodeRef.current &&
      nodeRef.current.scrollIntoView({
        behavior: 'smooth',
        block: 'nearest',
        inline: 'nearest',
      });
  }, [selected, inView]);

  return (
    <div
      ref={(el) => {
        nodeRef.current = el;
        ref(el);
      }}
      onClick={() => onClick && onClick(node.path + node.name)}
      className={`text-base ml-4 truncate flex flex-row cursor-pointer items-baseline gap-1 text-neutral-400 hover:bg-neutral-800 hover:text-neutral-300${
        isSelected ? ' !bg-neutral-600 !text-neutral-100' : ''
      }`}
    >
      <FileOutlined />
      {node.name}
    </div>
  );
};
