import { Component } from "solid-js";
import { Thumbnail } from "../types/Thumbnail";
import fallback from '../../../assets/noimage.png';

type Props = {
  thumb: Thumbnail;
  onClick: (thumb: Thumbnail) => void;
};

export const Folder: Component<Props> = (props) => {
  return (
    <div class="flex flex-col w-48 h-48 overflow-hidden">
      <img class="block cursor-pointer w-40 h-40 object-contain" onClick={() => props.onClick(props.thumb)} src={`data:image/jpeg;base64,${props.thumb.thumbnail}`} onError={(e) => e.currentTarget.src = fallback} />
      <div class="whitespace-nowrap text-ellipsis">{props.thumb.filename}</div>
    </div>
  );
};
