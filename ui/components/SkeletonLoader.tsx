import React from 'react';

export interface SkeletonLoaderProps {
  /**
   * Type of skeleton to render
   */
  variant?: 'text' | 'rect' | 'circle' | 'list' | 'table';
  
  /**
   * Width of the skeleton (CSS value)
   */
  width?: string | number;
  
  /**
   * Height of the skeleton (CSS value)
   */
  height?: string | number;
  
  /**
   * Number of lines/items for list/table variants
   */
  count?: number;
  
  /**
   * Dark or light theme
   */
  dark?: boolean;
  
  /**
   * Additional CSS class name
   */
  className?: string;
  
  /**
   * Accessible label for screen readers
   */
  ariaLabel?: string;
}

/**
 * Skeleton loader component with shimmer effect
 * 
 * @example
 * ```tsx
 * // Single line
 * <SkeletonLoader variant="text" width="80%" />
 * 
 * // List of items
 * <SkeletonLoader variant="list" count={5} />
 * 
 * // Table rows
 * <SkeletonLoader variant="table" count={3} />
 * ```
 */
export function SkeletonLoader({
  variant = 'rect',
  width = '100%',
  height = variant === 'text' ? 16 : 40,
  count = 1,
  dark = true,
  className = '',
  ariaLabel = 'Loading content',
}: SkeletonLoaderProps) {
  const baseColor = dark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.06)';
  const shimmerColor = dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.10)';
  
  const baseStyle: React.CSSProperties = {
    background: `linear-gradient(90deg, ${baseColor} 0%, ${shimmerColor} 50%, ${baseColor} 100%)`,
    backgroundSize: '200% 100%',
    animation: 'skeleton-shimmer 1.5s ease-in-out infinite',
    borderRadius: variant === 'circle' ? '50%' : variant === 'text' ? 4 : 6,
  };

  const renderSkeleton = () => {
    switch (variant) {
      case 'text':
        return (
          <div
            style={{
              ...baseStyle,
              width: typeof width === 'number' ? `${width}px` : width,
              height: typeof height === 'number' ? `${height}px` : height,
              marginBottom: 8,
            }}
          />
        );

      case 'circle':
        const size = typeof width === 'number' ? width : 40;
        return (
          <div
            style={{
              ...baseStyle,
              width: size,
              height: size,
            }}
          />
        );

      case 'list':
        return (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {Array.from({ length: count }).map((_, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                <div
                  style={{
                    ...baseStyle,
                    width: 40,
                    height: 40,
                    borderRadius: '50%',
                    flexShrink: 0,
                  }}
                />
                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <div
                    style={{
                      ...baseStyle,
                      width: `${60 + Math.random() * 30}%`,
                      height: 14,
                    }}
                  />
                  <div
                    style={{
                      ...baseStyle,
                      width: `${40 + Math.random() * 20}%`,
                      height: 12,
                    }}
                  />
                </div>
              </div>
            ))}
          </div>
        );

      case 'table':
        return (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {/* Header */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  style={{
                    ...baseStyle,
                    height: 16,
                    width: '80%',
                  }}
                />
              ))}
            </div>
            {/* Rows */}
            {Array.from({ length: count }).map((_, i) => (
              <div key={i} style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12 }}>
                {[1, 2, 3].map((j) => (
                  <div
                    key={j}
                    style={{
                      ...baseStyle,
                      height: 14,
                      width: `${50 + Math.random() * 40}%`,
                    }}
                  />
                ))}
              </div>
            ))}
          </div>
        );

      case 'rect':
      default:
        return (
          <div
            style={{
              ...baseStyle,
              width: typeof width === 'number' ? `${width}px` : width,
              height: typeof height === 'number' ? `${height}px` : height,
            }}
          />
        );
    }
  };

  return (
    <div
      className={className}
      role="status"
      aria-busy="true"
      aria-label={ariaLabel}
      style={{ width: '100%' }}
    >
      <style>{`
        @keyframes skeleton-shimmer {
          0% {
            background-position: 200% 0;
          }
          100% {
            background-position: -200% 0;
          }
        }
      `}</style>
      {renderSkeleton()}
      <span style={{ position: 'absolute', width: 1, height: 1, overflow: 'hidden', clip: 'rect(0,0,0,0)' }}>
        {ariaLabel}
      </span>
    </div>
  );
}

/**
 * Skeleton loader for asset list
 */
export function AssetListSkeleton({ dark = true, count = 3 }: { dark?: boolean; count?: number }) {
  return <SkeletonLoader variant="list" count={count} dark={dark} ariaLabel="Loading asset list" />;
}

/**
 * Skeleton loader for fee table
 */
export function FeeTableSkeleton({ dark = true, count = 3 }: { dark?: boolean; count?: number }) {
  return <SkeletonLoader variant="table" count={count} dark={dark} ariaLabel="Loading fee table" />;
}

/**
 * Skeleton loader for limits section
 */
export function LimitsSkeleton({ dark = true }: { dark?: boolean }) {
  const baseColor = dark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.06)';
  const shimmerColor = dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.10)';
  
  const baseStyle: React.CSSProperties = {
    background: `linear-gradient(90deg, ${baseColor} 0%, ${shimmerColor} 50%, ${baseColor} 100%)`,
    backgroundSize: '200% 100%',
    animation: 'skeleton-shimmer 1.5s ease-in-out infinite',
    borderRadius: 6,
  };

  return (
    <div role="status" aria-busy="true" aria-label="Loading limits">
      <style>{`
        @keyframes skeleton-shimmer {
          0% {
            background-position: 200% 0;
          }
          100% {
            background-position: -200% 0;
          }
        }
      `}</style>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
        {[1, 2].map((i) => (
          <div key={i} style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <div style={{ ...baseStyle, width: '40%', height: 14 }} />
            <div style={{ ...baseStyle, width: '100%', height: 8 }} />
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8 }}>
              <div style={{ ...baseStyle, width: '30%', height: 12 }} />
              <div style={{ ...baseStyle, width: '30%', height: 12 }} />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export default SkeletonLoader;
