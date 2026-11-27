/**
 * Internationalization formatting utilities using Intl API.
 *
 * Provides consistent formatting for amounts, currencies, dates, and times
 * across all AnchorKit UI components.
 */

import React, { useCallback } from 'react';
import { useI18n } from './i18n';

// ─── Types ────────────────────────────────────────────────────────────────

/** Asset information for formatting */
export interface AssetInfo {
  /** Asset code (e.g., "USDC", "XLM", "EUR") */
  code: string;
  /** Asset issuer (for non-native assets) */
  issuer?: string;
  /** Number of decimal places (default: 7 for Stellar) */
  decimals?: number;
  /** Display name override */
  displayName?: string;
}

/** Format amount options */
export interface FormatAmountOptions {
  /** Locale for formatting (defaults to current i18n locale) */
  locale?: string;
  /** Minimum fraction digits */
  minimumFractionDigits?: number;
  /** Maximum fraction digits */
  maximumFractionDigits?: number;
  /** Use grouping separators (default: true) */
  useGrouping?: boolean;
  /** Currency display style (default: 'code') */
  currencyDisplay?: 'code' | 'symbol' | 'narrowSymbol' | 'name';
  /** Compact notation for large numbers (default: false) */
  compactDisplay?: 'short' | 'long' | false;
  /** Show sign for positive numbers (default: false) */
  signDisplay?: 'auto' | 'never' | 'always' | 'exceptZero';
}

/** Format date/time options */
export interface FormatDateTimeOptions {
  /** Locale for formatting (defaults to current i18n locale) */
  locale?: string;
  /** Date style */
  dateStyle?: 'full' | 'long' | 'medium' | 'short';
  /** Time style */
  timeStyle?: 'full' | 'long' | 'medium' | 'short';
  /** Specific format options (overrides dateStyle/timeStyle) */
  formatOptions?: Intl.DateTimeFormatOptions;
  /** Time zone (default: system timezone) */
  timeZone?: string;
  /** Hour cycle (default: system preference) */
  hourCycle?: 'h11' | 'h12' | 'h23' | 'h24';
}

/** Format relative time options */
export interface FormatRelativeTimeOptions {
  /** Locale for formatting (defaults to current i18n locale) */
  locale?: string;
  /** Numeric style (default: 'auto' for "yesterday", "in 2 days") */
  numeric?: 'always' | 'auto';
  /** Style for relative time (default: 'long') */
  style?: 'long' | 'short' | 'narrow';
}

/** Stellar network asset info map */
export const KNOWN_ASSETS: Record<string, AssetInfo> = {
  XLM: { code: 'XLM', decimals: 7, displayName: 'Lumens' },
  USDC: { code: 'USDC', decimals: 7, displayName: 'USD Coin' },
  USDT: { code: 'USDT', decimals: 7, displayName: 'Tether USD' },
  EURC: { code: 'EURC', decimals: 7, displayName: 'Euro Coin' },
  BTC: { code: 'BTC', decimals: 7, displayName: 'Bitcoin' },
  ETH: { code: 'ETH', decimals: 7, displayName: 'Ethereum' },
  YXLM: { code: 'YXLM', decimals: 7, displayName: 'YieldBlox XLM' },
  AQUA: { code: 'AQUA', decimals: 7, displayName: 'Aqua' },
};

/**
 * Get asset info from known assets or create default.
 */
export function getAssetInfo(asset: string | AssetInfo): AssetInfo {
  if (typeof asset === 'object') return asset;
  const upper = asset.toUpperCase();
  return KNOWN_ASSETS[upper] ?? { code: upper, decimals: 7 };
}

/**
 * Format an amount using Intl.NumberFormat.
 *
 * @param value - The amount to format (in base units, e.g., stroops for XLM)
 * @param asset - Asset code or AssetInfo object
 * @param options - Formatting options
 * @returns Formatted amount string
 *
 * @example
 * ```ts
 * formatAmount('10000000', 'XLM') // "1.00 XLM" (10M stroops = 1 XLM)
 * formatAmount('1500000', 'USDC', { maximumFractionDigits: 2 }) // "0.15 USDC"
 * formatAmount(1234567.89, { code: 'EUR', decimals: 2 }, { locale: 'de-DE' }) // "1.234.567,89 EUR"
 * ```
 */
export function formatAmount(
  value: string | number,
  asset: string | AssetInfo,
  options: FormatAmountOptions = {},
): string {
  const {
    locale,
    minimumFractionDigits,
    maximumFractionDigits,
    useGrouping = true,
    currencyDisplay = 'code',
    compactDisplay = false,
    signDisplay = 'auto',
  } = options;

  const assetInfo = getAssetInfo(asset);
  const decimals = assetInfo.decimals ?? 7;

  // Convert from base units (stroops) to display units
  const numValue = typeof value === 'string' ? parseFloat(value) : value;
  const displayValue = numValue / Math.pow(10, decimals);

  const formatLocale = locale ?? (typeof navigator !== 'undefined' ? navigator.language : 'en-US');

  const formatter = new Intl.NumberFormat(formatLocale, {
    style: 'currency',
    currency: assetInfo.code,
    currencyDisplay,
    minimumFractionDigits: minimumFractionDigits ?? Math.min(decimals, 2),
    maximumFractionDigits: maximumFractionDigits ?? decimals,
    useGrouping,
    compactDisplay,
    signDisplay,
  });

  // Handle special case: if currency is not a valid ISO code, format as number + code
  try {
    return formatter.format(displayValue);
  } catch {
    // Fallback for non-ISO currency codes
    const numberFormatter = new Intl.NumberFormat(formatLocale, {
      minimumFractionDigits: minimumFractionDigits ?? Math.min(decimals, 2),
      maximumFractionDigits: maximumFractionDigits ?? decimals,
      useGrouping,
      compactDisplay,
      signDisplay,
    });
    return `${numberFormatter.format(displayValue)} ${assetInfo.code}`;
  }
}

/**
 * Format a raw amount (already in display units) without unit conversion.
 */
export function formatRawAmount(
  value: string | number,
  asset: string | AssetInfo,
  options: FormatAmountOptions = {},
): string {
  const assetInfo = getAssetInfo(asset);
  const { locale, ...restOptions } = options;
  const formatLocale = locale ?? (typeof navigator !== 'undefined' ? navigator.language : 'en-US');

  const numValue = typeof value === 'string' ? parseFloat(value) : value;

  try {
    const formatter = new Intl.NumberFormat(formatLocale, {
      style: 'currency',
      currency: assetInfo.code,
      currencyDisplay: options.currencyDisplay ?? 'code',
      minimumFractionDigits: options.minimumFractionDigits ?? 2,
      maximumFractionDigits: options.maximumFractionDigits ?? (assetInfo.decimals ?? 2),
      useGrouping: options.useGrouping ?? true,
      compactDisplay: options.compactDisplay ?? false,
      signDisplay: options.signDisplay ?? 'auto',
    });
    return formatter.format(numValue);
  } catch {
    const numberFormatter = new Intl.NumberFormat(formatLocale, {
      minimumFractionDigits: options.minimumFractionDigits ?? 2,
      maximumFractionDigits: options.maximumFractionDigits ?? (assetInfo.decimals ?? 2),
      useGrouping: options.useGrouping ?? true,
      compactDisplay: options.compactDisplay ?? false,
      signDisplay: options.signDisplay ?? 'auto',
    });
    return `${numberFormatter.format(numValue)} ${assetInfo.code}`;
  }
}

/**
 * Format a timestamp using Intl.DateTimeFormat.
 *
 * @param timestamp - Timestamp in milliseconds or ISO string
 * @param options - Formatting options
 * @returns Formatted date/time string
 *
 * @example
 * ```ts
 * formatDateTime(Date.now()) // "Nov 15, 2023, 2:30 PM"
 * formatDateTime('2023-11-15T14:30:00Z', { dateStyle: 'short', timeStyle: 'short' }) // "11/15/23, 2:30 PM"
 * formatDateTime(1699987800000, { locale: 'de-DE', dateStyle: 'long', timeStyle: 'short' }) // "15. November 2023, 14:30"
 * ```
 */
export function formatDateTime(
  timestamp: number | string | Date,
  options: FormatDateTimeOptions = {},
): string {
  const {
    locale,
    dateStyle = 'medium',
    timeStyle = 'short',
    formatOptions,
    timeZone,
    hourCycle,
  } = options;

  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);

  if (isNaN(date.getTime())) {
    return 'Invalid date';
  }

  const formatLocale = locale ?? (typeof navigator !== 'undefined' ? navigator.language : 'en-US');

  const formatterOptions: Intl.DateTimeFormatOptions = {
    dateStyle,
    timeStyle,
    timeZone,
    hourCycle,
    ...formatOptions,
  };

  // Remove dateStyle/timeStyle if formatOptions provides specific options
  if (formatOptions) {
    delete formatterOptions.dateStyle;
    delete formatterOptions.timeStyle;
  }

  try {
    const formatter = new Intl.DateTimeFormat(formatLocale, formatterOptions);
    return formatter.format(date);
  } catch {
    // Fallback
    return date.toLocaleString(formatLocale, { dateStyle, timeStyle });
  }
}

/**
 * Format a date only (no time).
 */
export function formatDate(
  timestamp: number | string | Date,
  options: Omit<FormatDateTimeOptions, 'timeStyle'> = {},
): string {
  return formatDateTime(timestamp, { ...options, timeStyle: undefined, dateStyle: options.dateStyle ?? 'medium' });
}

/**
 * Format a time only (no date).
 */
export function formatTime(
  timestamp: number | string | Date,
  options: Omit<FormatDateTimeOptions, 'dateStyle'> = {},
): string {
  return formatDateTime(timestamp, { ...options, dateStyle: undefined, timeStyle: options.timeStyle ?? 'short' });
}

/**
 * Format relative time (e.g., "2 hours ago", "in 3 days").
 *
 * @param timestamp - Target timestamp in milliseconds
 * @param options - Formatting options
 * @returns Relative time string
 *
 * @example
 * ```ts
 * formatRelativeTime(Date.now() - 3600000) // "1 hour ago"
 * formatRelativeTime(Date.now() + 86400000) // "in 1 day"
 * ```
 */
export function formatRelativeTime(
  timestamp: number | string | Date,
  options: FormatRelativeTimeOptions = {},
): string {
  const { locale, numeric = 'auto', style = 'long' } = options;

  const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
  const now = Date.now();
  const diffMs = date.getTime() - now;
  const diffSec = Math.round(diffMs / 1000);

  const formatLocale = locale ?? (typeof navigator !== 'undefined' ? navigator.language : 'en-US');

  try {
    const formatter = new Intl.RelativeTimeFormat(formatLocale, { numeric, style });

    // Find the appropriate unit
    const absSec = Math.abs(diffSec);
    if (absSec < 60) {
      return formatter.format(diffSec, 'second');
    }
    const diffMin = Math.round(diffSec / 60);
    if (absSec < 3600) {
      return formatter.format(diffMin, 'minute');
    }
    const diffHour = Math.round(diffSec / 3600);
    if (absSec < 86400) {
      return formatter.format(diffHour, 'hour');
    }
    const diffDay = Math.round(diffSec / 86400);
    if (absSec < 2592000) {
      return formatter.format(diffDay, 'day');
    }
    const diffMonth = Math.round(diffSec / 2592000);
    if (absSec < 31536000) {
      return formatter.format(diffMonth, 'month');
    }
    const diffYear = Math.round(diffSec / 31536000);
    return formatter.format(diffYear, 'year');
  } catch {
    // Fallback for environments without RelativeTimeFormat
    const absSec = Math.abs(diffSec);
    if (absSec < 60) return diffSec < 0 ? `${absSec} seconds ago` : `in ${diffSec} seconds`;
    const diffMin = Math.round(absSec / 60);
    if (absSec < 3600) return diffSec < 0 ? `${diffMin} minutes ago` : `in ${diffMin} minutes`;
    const diffHour = Math.round(absSec / 3600);
    if (absSec < 86400) return diffSec < 0 ? `${diffHour} hours ago` : `in ${diffHour} hours`;
    const diffDay = Math.round(absSec / 86400);
    if (absSec < 2592000) return diffSec < 0 ? `${diffDay} days ago` : `in ${diffDay} days`;
    const diffMonth = Math.round(absSec / 2592000);
    if (absSec < 31536000) return diffSec < 0 ? `${diffMonth} months ago` : `in ${diffMonth} months`;
    const diffYear = Math.round(absSec / 31536000);
    return diffSec < 0 ? `${diffYear} years ago` : `in ${diffYear} years`;
  }
}

/**
 * Format a duration in milliseconds to a human-readable string.
 */
export function formatDuration(
  ms: number,
  options: { locale?: string; style?: 'long' | 'short' | 'narrow' } = {},
): string {
  const { locale, style = 'long' } = options;
  const formatLocale = locale ?? (typeof navigator !== 'undefined' ? navigator.language : 'en-US');

  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  try {
    const formatter = new Intl.RelativeTimeFormat(formatLocale, { numeric: 'always', style });

    if (days > 0) return formatter.format(days, 'day');
    if (hours > 0) return formatter.format(hours, 'hour');
    if (minutes > 0) return formatter.format(minutes, 'minute');
    return formatter.format(seconds, 'second');
  } catch {
    if (days > 0) return `${days}d ${hours % 24}h`;
    if (hours > 0) return `${hours}h ${minutes % 60}m`;
    if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
    return `${seconds}s`;
  }
}

/**
 * React hook for formatting amounts with current locale.
 */
export function useFormatAmount() {
  const { locale } = useI18n();

  return useCallback(
    (value: string | number, asset: string | AssetInfo, options: FormatAmountOptions = {}) => {
      return formatAmount(value, asset, { ...options, locale: options.locale ?? locale });
    },
    [locale],
  );
}

/**
 * React hook for formatting dates/times with current locale.
 */
export function useFormatDateTime() {
  const { locale } = useI18n();

  return useCallback(
    (timestamp: number | string | Date, options: FormatDateTimeOptions = {}) => {
      return formatDateTime(timestamp, { ...options, locale: options.locale ?? locale });
    },
    [locale],
  );
}

/**
 * React hook for formatting relative time with current locale.
 */
export function useFormatRelativeTime() {
  const { locale } = useI18n();

  return useCallback(
    (timestamp: number | string | Date, options: FormatRelativeTimeOptions = {}) => {
      return formatRelativeTime(timestamp, { ...options, locale: options.locale ?? locale });
    },
    [locale],
  );
}

export default {
  formatAmount,
  formatRawAmount,
  formatDateTime,
  formatDate,
  formatTime,
  formatRelativeTime,
  formatDuration,
  getAssetInfo,
  useFormatAmount,
  useFormatDateTime,
  useFormatRelativeTime,
};