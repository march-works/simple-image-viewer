/**
 * パスを比較用に正規化する
 * バックスラッシュをスラッシュに変換し、小文字化
 */
export const normalizePathForComparison = (path: string): string => {
  return path.replace(/\\/g, '/').toLowerCase();
};
