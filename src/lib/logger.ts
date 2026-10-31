import {
  error as logError,
  warn as logWarn,
  info as logInfo,
  debug,
} from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';

let initialized = false;

/**
 * Initializes frontend log capture for uncaught errors and unhandled promise rejections.
 * Native browser console methods are NOT intercepted, preserving DevTools performance and stack traces.
 */
export async function initLogger() {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  // Global uncaught JS error handler
  window.addEventListener('error', (event) => {
    logError(
      `[Frontend Uncaught Error] ${event.message} at ${event.filename}:${event.lineno}:${event.colno}\nStack: ${event.error?.stack || 'No stack'}`,
    );
  });

  // Global unhandled promise rejection handler
  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason;
    const message =
      reason instanceof Error
        ? `${reason.message}\nStack: ${reason.stack}`
        : typeof reason === 'string'
          ? reason
          : JSON.stringify(reason);
    logError(`[Frontend Unhandled Rejection] ${message}`);
  });

  logInfo('[Frontend] Logger initialized successfully');
}

/**
 * Opens the application's log directory or selects the session log file.
 * Flushes the in-memory flight recorder to the session dump on demand.
 * Crash reports are written separately as crash-*.log and are not overwritten.
 */
export async function openLogsFolder(): Promise<string> {
  return await invoke<string>('open_logs_dir');
}

/**
 * Retrieves the application's log directory path.
 */
export async function getLogDirectory(): Promise<string> {
  return await invoke<string>('get_log_dir');
}

export { logError as error, logWarn as warn, logInfo as info, debug };
