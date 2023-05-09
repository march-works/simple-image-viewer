import { HiSolidChevronDoubleLeft, HiSolidChevronDoubleRight, HiSolidChevronLeft, HiSolidChevronRight } from "solid-icons/hi";
import { Component, For } from "solid-js";

type Props = {
  current: number;
  end: number;
  onClickPage: (page: number) => void;
  onClickPrev: () => void;
  onClickNext: () => void;
  onClickStart: () => void;
  onClickEnd: () => void;
};

export const Pagination: Component<Props> = (props) => {
  const start = () => Math.max(props.current - 4, 1);
  const length = () => Math.min(props.end - start() + 1, 9);
  return (
    <div class="flex items-center">
      <div class="p-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300" onClick={() => props.onClickStart()}>
        <HiSolidChevronDoubleLeft class="text-2xl" />
      </div>
      <div class="p-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300" onClick={() => props.onClickPrev()}>
        <HiSolidChevronLeft class="text-2xl" />
      </div>
      <For each={Array(length()).fill(undefined)}>
        {(_, i) => (
          <div class={`px-4 py-3 leading-tight border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300 ${start() + i() === props.current ?  "!bg-neutral-600 !text-neutral-100" : "cursor-pointer"}`} onClick={() => props.onClickPage(start() + i())}>{start() + i()}</div>
        )}
      </For>
      <div class="p-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300" onClick={() => props.onClickNext()}>
        <HiSolidChevronRight class="text-2xl" />
      </div>
      <div class="p-2 border-neutral-500 bg-neutral-900 text-neutral-400 transition-colors hover:bg-neutral-700 hover:text-neutral-300" onClick={() => props.onClickEnd()}>
        <HiSolidChevronDoubleRight class="text-2xl" />
      </div>
    </div>
  );
};