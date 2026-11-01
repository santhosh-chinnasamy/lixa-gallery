export type CullFlag = 'unflagged' | 'picked' | 'rejected';
export type CullAction = 'pick' | 'reject' | 'unflag';

export interface PhotoCullState {
  path: string;
  flag: CullFlag;
}

/** One flag per photo. Pick and reject toggle off when pressed again. */
export function flagAfterAction(
  current: CullFlag,
  action: CullAction,
): CullFlag {
  if (action === 'unflag') return 'unflagged';
  if (action === 'pick') return current === 'picked' ? 'unflagged' : 'picked';
  return current === 'rejected' ? 'unflagged' : 'rejected';
}
