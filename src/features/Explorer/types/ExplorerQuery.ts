export type SortField =
  | 'Name'
  | 'DateModified'
  | 'DateCreated'
  | 'Recommendation';
export type SortOrder = 'Asc' | 'Desc';

export type SortConfig = {
  field: SortField;
  order: SortOrder;
};

export const defaultSortConfig: SortConfig = {
  field: 'DateModified',
  order: 'Desc',
};

export type SortOption = {
  label: string;
  config: SortConfig;
};

export const sortOptions: SortOption[] = [
  { label: '名前 ↑', config: { field: 'Name', order: 'Asc' } },
  { label: '名前 ↓', config: { field: 'Name', order: 'Desc' } },
  { label: '更新日 ↑', config: { field: 'DateModified', order: 'Asc' } },
  { label: '更新日 ↓', config: { field: 'DateModified', order: 'Desc' } },
  { label: '作成日 ↑', config: { field: 'DateCreated', order: 'Asc' } },
  { label: '作成日 ↓', config: { field: 'DateCreated', order: 'Desc' } },
  { label: 'おすすめ', config: { field: 'Recommendation', order: 'Desc' } },
];

export const getSortOptionIndex = (config: SortConfig): number => {
  return sortOptions.findIndex(
    (opt) =>
      opt.config.field === config.field && opt.config.order === config.order,
  );
};
