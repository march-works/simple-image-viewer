import { faCaretDown, faCaretRight, faFolder, faFolderOpen } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { FC, useState } from 'react';
import { match } from 'ts-pattern';
import { Directory, DirectoryTree } from '../types/DirectoryTree';
import { FileNode } from './FileNode';
import { NodeBaseStyle } from './NodeBaseStyle';
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
      <NodeBaseStyle
        onClick={() => {
          onClick && onClick(tree.path);
          setOpen((prev) => !prev);
        }}
      >
        {open ? (
          <FontAwesomeIcon icon={faCaretDown} />
        ) : (
          <FontAwesomeIcon icon={faCaretRight} />
        )}
        {open ? <FontAwesomeIcon icon={faFolderOpen} /> : <FontAwesomeIcon icon={faFolder} />}
        <div className="hidden truncate lg:block">{tree.name}</div>
      </NodeBaseStyle>
      {tree.children.length > 0 && open && (
        <div className="lg:ml-6">
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
