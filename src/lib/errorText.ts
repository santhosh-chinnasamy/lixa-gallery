export function errorText(error: unknown): string {
  if (typeof error === 'string') {
    const trimmed = error.trim();
    if (trimmed) return trimmed;
  }
  if (error instanceof Error) {
    const trimmed = error.message.trim();
    if (trimmed) return trimmed;
  }
  if (error && typeof error === 'object' && 'message' in error) {
    const message = (error as { message: unknown }).message;
    if (typeof message === 'string' && message.trim()) return message.trim();
  }
  return 'Something went wrong.';
}

export function folderOpenError(folderPath: string, error: unknown): string {
  return `Couldn't open ${folderPath}. ${errorText(error)}`;
}

export function favouriteToggleError(error: unknown): string {
  return `Couldn't update favourite. ${errorText(error)}`;
}
