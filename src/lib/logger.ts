import {
  attachConsole,
  error as logError,
  warn as logWarn,
  info as logInfo,
  debug as logDebug,
} from '@tauri-apps/plugin-log';
import { invoke } from '@tauri-apps/api/core';

let initialized = false;

/**
 * Initializes frontend log capture, attaching console output and global
 * error/unhandledrejection listeners to pipe logs to the native file target.
 */
export async function initLogger() {
  if (initialized || typeof window === 'undefined') return;
  initialized = true;

  try {
    // Attach Rust logs to the webview console in development
    await attachConsole();
  } catch (err) {
    // Expected when running outside Tauri or if console attachment is unsupported
  }

  // Global uncaught JS error handler
  window.addEventListener('error', (event) => {
    const errorDetails = {
      message: event.message,
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno,
      stack: event.error?.stack,
    };
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

  // Intercept console.error to log to native file
  const originalConsoleError = console.error;
  console.error = (...args: unknown[]) => {
    originalConsoleError(...args);
    const message = args
      .map((a) =>
        a instanceof Error
          ? `${a.message}\n${a.stack}`
          : typeof a === 'object'
            ? JSON.stringify(a)
            : String(a),
      )
      .join(' ');
    logError(`[Console Error] ${message}`);
  };

  // Intercept console.warn to log to native file
  const originalConsoleWarn = console.warn;
  console.warn = (...args: unknown[]) => {
    originalConsoleWarn(...args);
    const message = args
      .map((a) =>
        a instanceof Error
          ? `${a.message}\n${a.stack}`
          : typeof a === 'object'
            ? JSON.stringify(a)
            : String(a),
      )
      .join(' ');
    logWarn(`[Console Warn] ${message}`);
  };

  logInfo('[Frontend] Logger initialized successfully');
}

/**
 * Opens the application's log directory or selects the log file in the user's OS file manager.
 * Returns the path of the target opened.
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

export {
  logError as error,
  logWarn as warn,
  logInfo as info,
  logDebug as debug,
};
