import { Component, createEffect } from 'solid-js';
import { Zip } from '../types/DirectoryTree';
import { NodeBaseStyle } from './NodeBaseStyle';
import { FaSolidImage } from 'solid-icons/fa';

type Props = {
  node: Zip;
  isSelected: boolean;
  onClick?: (path: string) => void;
};

export const ZipNode: Component<Props> = (props) => {
  let nodeRef: HTMLDivElement | undefined = undefined;

  createEffect(() => {
    props.isSelected &&
      nodeRef &&
      nodeRef.scrollIntoView({
        behavior: 'smooth',
        block: 'center',
        inline: 'center',
      });
  });

  return (
    <NodeBaseStyle
      class="pl-3"
      ref={nodeRef}
      isSelected={props.isSelected}
      onClick={() =>
        props.onClick && props.onClick(props.node.path + props.node.name)
      }
    >
      <FaSolidImage />
      <div class="hidden lg:block">{props.node.name}</div>
    </NodeBaseStyle>
  );
};
